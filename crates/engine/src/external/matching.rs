use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::command_line::SshForward;
use crate::store::{Settings, TunnelDefinition, TunnelKind};

const LOCALHOST: &str = "localhost";
const WILDCARD: &str = "*";
const IPV6_BRACKETS: [char; 2] = ['[', ']'];

/// Whether `forward`, read from an `ssh` command line, provides `definition`.
///
/// The ssh destination is deliberately not compared.
pub(crate) fn forward_matches(
    definition: &TunnelDefinition,
    settings: &Settings,
    forward: &SshForward,
) -> bool {
    definition.kind == forward.kind
        && definition.listen_port == forward.listen_port
        && forward_address_overlaps(
            definition.effective_listen_address(settings),
            forward.listen_address.as_deref(),
        )
        && targets_match(definition, forward)
}

/// Whether a socket bound to `listener` can be the one `configured` asks for.
///
/// Host names other than `localhost` are not resolved: they conservatively
/// overlap.
pub(crate) fn address_overlaps_listener(configured: &str, listener: IpAddr) -> bool {
    let configured = configured.trim().trim_matches(IPV6_BRACKETS);
    if configured.is_empty() || configured == WILDCARD || listener.is_unspecified() {
        return true;
    }
    if configured.eq_ignore_ascii_case(LOCALHOST) {
        return listener.is_loopback();
    }
    match configured.parse::<IpAddr>() {
        Ok(configured_ip) => configured_ip.is_unspecified() || configured_ip == listener,
        Err(_) => true,
    }
}

fn forward_address_overlaps(configured: &str, forward_address: Option<&str>) -> bool {
    match forward_listener_addresses(forward_address) {
        Some(addresses) => addresses
            .into_iter()
            .any(|address| address_overlaps_listener(configured, address)),
        None => true,
    }
}

fn forward_listener_addresses(forward_address: Option<&str>) -> Option<Vec<IpAddr>> {
    let Some(address) = forward_address else {
        return Some(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]);
    };
    let address = address.trim().trim_matches(IPV6_BRACKETS);
    if address.is_empty() || address == WILDCARD {
        return Some(vec![IpAddr::V4(Ipv4Addr::UNSPECIFIED)]);
    }
    if address.eq_ignore_ascii_case(LOCALHOST) {
        return Some(vec![
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        ]);
    }
    address.parse::<IpAddr>().ok().map(|ip| vec![ip])
}

fn targets_match(definition: &TunnelDefinition, forward: &SshForward) -> bool {
    if definition.kind == TunnelKind::Dynamic {
        return true;
    }
    let is_port_equal = forward.target_port == Some(definition.effective_target_port());
    let is_host_equivalent = forward
        .target_host
        .as_deref()
        .is_some_and(|host| hosts_are_equivalent(definition.effective_target_host(), host));
    is_port_equal && is_host_equivalent
}

fn hosts_are_equivalent(first: &str, second: &str) -> bool {
    (is_loopback_host(first) && is_loopback_host(second)) || first.eq_ignore_ascii_case(second)
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case(LOCALHOST)
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::test_support::tunnel;

    fn forward(kind: TunnelKind, address: Option<&str>, port: u16) -> SshForward {
        SshForward {
            kind,
            listen_address: address.map(str::to_owned),
            listen_port: port,
            target_host: Some("127.0.0.1".to_owned()),
            target_port: Some(port),
        }
    }

    fn local_forward() -> SshForward {
        forward(TunnelKind::Local, None, 3333)
    }

    fn matches(definition: &TunnelDefinition, forward: &SshForward) -> bool {
        forward_matches(definition, &Settings::default(), forward)
    }

    #[test]
    fn should_match_when_forward_equals_the_definition() {
        assert!(matches(&tunnel(TunnelKind::Local), &local_forward()));
    }

    #[test]
    fn should_not_match_when_kinds_differ() {
        let remote = forward(TunnelKind::Remote, None, 3333);

        assert!(!matches(&tunnel(TunnelKind::Local), &remote));
    }

    #[test]
    fn should_not_match_when_listen_ports_differ() {
        let other_port = forward(TunnelKind::Local, None, 4444);

        assert!(!matches(&tunnel(TunnelKind::Local), &other_port));
    }

    #[test]
    fn should_not_match_when_target_ports_differ() {
        let other_target = SshForward {
            target_port: Some(1),
            ..local_forward()
        };

        assert!(!matches(&tunnel(TunnelKind::Local), &other_target));
    }

    #[test]
    fn should_not_match_when_target_hosts_differ() {
        let other_host = SshForward {
            target_host: Some("db.internal".to_owned()),
            ..local_forward()
        };

        assert!(!matches(&tunnel(TunnelKind::Local), &other_host));
    }

    #[test]
    fn should_match_when_target_hosts_are_both_loopback() {
        let localhost = SshForward {
            target_host: Some("localhost".to_owned()),
            ..local_forward()
        };

        assert!(matches(&tunnel(TunnelKind::Local), &localhost));
    }

    #[test]
    fn should_match_when_target_hosts_differ_only_by_case() {
        let definition = TunnelDefinition {
            target_host: Some("DB.internal".to_owned()),
            ..tunnel(TunnelKind::Local)
        };
        let forward = SshForward {
            target_host: Some("db.INTERNAL".to_owned()),
            ..local_forward()
        };

        assert!(matches(&definition, &forward));
    }

    #[test]
    fn should_match_when_target_host_is_in_the_loopback_block() {
        let forward = SshForward {
            target_host: Some("127.0.1.1".to_owned()),
            ..local_forward()
        };

        assert!(matches(&tunnel(TunnelKind::Local), &forward));
    }

    #[test]
    fn should_match_when_target_host_is_ipv6_loopback() {
        let forward = SshForward {
            target_host: Some("::1".to_owned()),
            ..local_forward()
        };

        assert!(matches(&tunnel(TunnelKind::Local), &forward));
    }

    #[test]
    fn should_match_dynamic_forward_without_comparing_targets() {
        let dynamic = SshForward {
            target_host: None,
            target_port: None,
            ..forward(TunnelKind::Dynamic, None, 3333)
        };

        assert!(matches(&tunnel(TunnelKind::Dynamic), &dynamic));
    }

    #[test]
    fn should_not_match_when_forward_binds_another_address() {
        let other_address = forward(TunnelKind::Local, Some("198.51.100.50"), 3333);

        assert!(!matches(&tunnel(TunnelKind::Local), &other_address));
    }

    #[test]
    fn should_match_when_forward_binds_the_wildcard() {
        let wildcard = forward(TunnelKind::Local, Some("*"), 3333);

        assert!(matches(&tunnel(TunnelKind::Local), &wildcard));
    }

    #[test]
    fn should_match_when_forward_binds_localhost() {
        let localhost = forward(TunnelKind::Local, Some("localhost"), 3333);

        assert!(matches(&tunnel(TunnelKind::Local), &localhost));
    }

    #[test]
    fn should_match_when_definition_binds_the_wildcard() {
        let definition = TunnelDefinition {
            listen_address: Some("0.0.0.0".to_owned()),
            ..tunnel(TunnelKind::Local)
        };

        assert!(matches(&definition, &local_forward()));
    }

    #[test]
    fn should_not_match_when_definition_binds_another_address_than_the_default() {
        let definition = TunnelDefinition {
            listen_address: Some("192.0.2.20".to_owned()),
            ..tunnel(TunnelKind::Local)
        };

        assert!(!matches(&definition, &local_forward()));
    }

    fn address(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn should_overlap_loopback_listener_when_configured_is_localhost() {
        assert!(address_overlaps_listener("localhost", address("::1")));
    }

    #[test]
    fn should_not_overlap_public_listener_when_configured_is_localhost() {
        assert!(!address_overlaps_listener(
            "localhost",
            address("198.51.100.10")
        ));
    }

    #[test]
    fn should_overlap_when_configured_is_the_wildcard() {
        assert!(address_overlaps_listener("*", address("198.51.100.10")));
    }

    #[test]
    fn should_overlap_when_configured_is_empty() {
        assert!(address_overlaps_listener("", address("198.51.100.10")));
    }

    #[test]
    fn should_overlap_when_configured_is_unspecified() {
        assert!(address_overlaps_listener("::", address("198.51.100.10")));
    }

    #[test]
    fn should_overlap_when_listener_is_unspecified() {
        assert!(address_overlaps_listener(
            "198.51.100.10",
            address("0.0.0.0")
        ));
    }

    #[test]
    fn should_overlap_when_addresses_are_equal() {
        assert!(address_overlaps_listener(
            "198.51.100.10",
            address("198.51.100.10")
        ));
    }

    #[test]
    fn should_not_overlap_when_addresses_differ() {
        assert!(!address_overlaps_listener(
            "198.51.100.10",
            address("198.51.100.20")
        ));
    }

    #[test]
    fn should_overlap_when_configured_is_an_unresolved_host_name() {
        assert!(address_overlaps_listener(
            "vpn.internal",
            address("198.51.100.20")
        ));
    }

    #[test]
    fn should_overlap_when_configured_is_a_bracketed_ipv6_address() {
        assert!(address_overlaps_listener("[::1]", address("::1")));
    }
}
