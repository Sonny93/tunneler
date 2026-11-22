use std::net::IpAddr;

use super::plan_types::ConfigImportRisk;
use crate::store::{Settings, TunnelDefinition, TunnelKind};

const LOCALHOST_NAME: &str = "localhost";

/// What makes an importable tunnel worth a second look on this machine.
pub(super) fn find_risks(tunnel: &TunnelDefinition, settings: &Settings) -> Vec<ConfigImportRisk> {
    let mut risks = Vec::new();
    let address = tunnel.effective_listen_address(settings);
    if !is_loopback_host(address) {
        risks.push(ConfigImportRisk::NonLoopbackListen {
            address: address.trim().to_owned(),
        });
    }
    let target_host = tunnel.effective_target_host();
    if tunnel.kind == TunnelKind::Remote && !is_loopback_host(target_host) {
        risks.push(ConfigImportRisk::RemoteNonLoopbackTarget {
            target_host: target_host.trim().to_owned(),
        });
    }
    let launching_tags: Vec<String> = tunnel
        .tags
        .iter()
        .filter(|tag| settings.start_on_launch_tags.contains(tag))
        .cloned()
        .collect();
    if !launching_tags.is_empty() {
        risks.push(ConfigImportRisk::StartsOnLaunch {
            tags: launching_tags,
        });
    }
    risks
}

/// Whether the host stays on this machine: `localhost`, or a loopback IP
/// literal, with or without brackets. Anything else, a wildcard included,
/// does not.
fn is_loopback_host(host: &str) -> bool {
    let trimmed = host.trim();
    let unbracketed = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(trimmed);
    if unbracketed.eq_ignore_ascii_case(LOCALHOST_NAME) {
        return true;
    }
    unbracketed
        .parse::<IpAddr>()
        .is_ok_and(|address| address.is_loopback())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::store::test_support::tunnel;

    fn with_address(kind: TunnelKind, address: &str) -> TunnelDefinition {
        TunnelDefinition {
            listen_address: Some(address.to_owned()),
            ..tunnel(kind)
        }
    }

    #[test]
    fn should_find_no_risk_when_a_local_tunnel_listens_on_the_default_loopback() {
        assert_eq!(
            find_risks(&tunnel(TunnelKind::Local), &Settings::default()),
            Vec::new()
        );
    }

    #[test]
    fn should_accept_every_spelling_of_loopback_when_a_local_tunnel_listens() {
        for address in [
            "127.0.0.1",
            "127.0.0.5",
            "::1",
            "[::1]",
            "localhost",
            "LocalHost",
        ] {
            assert_eq!(
                find_risks(
                    &with_address(TunnelKind::Local, address),
                    &Settings::default()
                ),
                Vec::new(),
                "{address}"
            );
        }
    }

    #[test]
    fn should_flag_the_address_when_a_local_tunnel_listens_beyond_loopback() {
        for address in ["0.0.0.0", "::", "*", "192.0.2.10", "nova.example"] {
            assert_eq!(
                find_risks(
                    &with_address(TunnelKind::Local, address),
                    &Settings::default()
                ),
                vec![ConfigImportRisk::NonLoopbackListen {
                    address: address.to_owned()
                }],
                "{address}"
            );
        }
    }

    #[test]
    fn should_flag_the_default_address_when_the_settings_bind_beyond_loopback() {
        let settings = Settings {
            default_bind_address: "0.0.0.0".to_owned(),
            ..Settings::default()
        };

        assert_eq!(
            find_risks(&tunnel(TunnelKind::Dynamic), &settings),
            vec![ConfigImportRisk::NonLoopbackListen {
                address: "0.0.0.0".to_owned()
            }]
        );
    }

    #[test]
    fn should_flag_the_server_side_address_when_a_remote_tunnel_listens_beyond_loopback() {
        assert_eq!(
            find_risks(
                &with_address(TunnelKind::Remote, "0.0.0.0"),
                &Settings::default()
            ),
            vec![ConfigImportRisk::NonLoopbackListen {
                address: "0.0.0.0".to_owned()
            }]
        );
    }

    #[test]
    fn should_not_flag_a_remote_tunnel_on_the_default_remote_address() {
        assert_eq!(
            find_risks(&tunnel(TunnelKind::Remote), &Settings::default()),
            Vec::new()
        );
    }

    #[test]
    fn should_flag_the_default_remote_address_when_the_settings_bind_it_beyond_loopback() {
        let settings = Settings {
            default_remote_bind_address: "0.0.0.0".to_owned(),
            ..Settings::default()
        };

        assert_eq!(
            find_risks(&tunnel(TunnelKind::Remote), &settings),
            vec![ConfigImportRisk::NonLoopbackListen {
                address: "0.0.0.0".to_owned()
            }]
        );
    }

    #[test]
    fn should_flag_the_target_when_a_remote_tunnel_forwards_beyond_loopback() {
        let definition = TunnelDefinition {
            target_host: Some("db.internal".to_owned()),
            ..tunnel(TunnelKind::Remote)
        };

        assert_eq!(
            find_risks(&definition, &Settings::default()),
            vec![ConfigImportRisk::RemoteNonLoopbackTarget {
                target_host: "db.internal".to_owned()
            }]
        );
    }

    #[test]
    fn should_not_flag_the_target_when_a_local_tunnel_forwards_beyond_loopback() {
        let definition = TunnelDefinition {
            target_host: Some("db.internal".to_owned()),
            ..tunnel(TunnelKind::Local)
        };

        assert_eq!(find_risks(&definition, &Settings::default()), Vec::new());
    }

    #[test]
    fn should_flag_the_tags_when_some_start_on_launch() {
        let settings = Settings {
            start_on_launch_tags: vec!["boot".to_owned(), "other".to_owned()],
            ..Settings::default()
        };
        let definition = TunnelDefinition {
            tags: vec!["dev".to_owned(), "boot".to_owned()],
            ..tunnel(TunnelKind::Local)
        };

        assert_eq!(
            find_risks(&definition, &settings),
            vec![ConfigImportRisk::StartsOnLaunch {
                tags: vec!["boot".to_owned()]
            }]
        );
    }
}
