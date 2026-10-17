//! Conversion of parsed `ssh_config` forwards into import candidates.

use ssh_config::{
    DynamicForward, Endpoint, ForwardSpec, RemoteForward, parse_dynamic_forward_argument,
    parse_local_forward_argument, parse_remote_forward_argument,
};

use super::ImportCandidate;
use crate::store::TunnelKind;

/// What reading one forward specification gave.
pub(super) enum ParsedForward {
    /// A TCP forward, without host, label or line.
    Supported(ImportCandidate),
    /// A Unix socket or a remote SOCKS proxy.
    Unsupported,
    /// Not a forward specification.
    Malformed,
}

/// Reads the argument of `-L`, `-R` or `-D`.
pub(super) fn parse_forward(kind: TunnelKind, argument: &str) -> ParsedForward {
    match kind {
        TunnelKind::Local => parse_local_forward_argument(argument)
            .map_or(ParsedForward::Malformed, |spec| {
                supported_or_not(fixed_candidate(kind, &spec))
            }),
        TunnelKind::Remote => {
            parse_remote_forward_argument(argument).map_or(ParsedForward::Malformed, |forward| {
                match forward {
                    RemoteForward::Fixed(spec) => supported_or_not(fixed_candidate(kind, &spec)),
                    RemoteForward::Dynamic(_) => ParsedForward::Unsupported,
                }
            })
        }
        TunnelKind::Dynamic => parse_dynamic_forward_argument(argument)
            .map_or(ParsedForward::Malformed, |forward| {
                supported_or_not(dynamic_candidate(&forward))
            }),
    }
}

fn supported_or_not(candidate: Option<ImportCandidate>) -> ParsedForward {
    candidate.map_or(ParsedForward::Unsupported, ParsedForward::Supported)
}

/// A local or remote forward between two TCP endpoints; `None` for a Unix socket.
pub(super) fn fixed_candidate(kind: TunnelKind, spec: &ForwardSpec) -> Option<ImportCandidate> {
    let (listen_address, listen_port) = tcp_parts(&spec.listen)?;
    let (target_host, target_port) = tcp_parts(&spec.target)?;
    if listen_port == 0 {
        return None;
    }
    Some(ImportCandidate {
        target_host,
        target_port: Some(target_port),
        ..blank_candidate(kind, listen_address, listen_port)
    })
}

/// A SOCKS proxy on a TCP endpoint; `None` for a Unix socket.
pub(super) fn dynamic_candidate(forward: &DynamicForward) -> Option<ImportCandidate> {
    let (listen_address, listen_port) = tcp_parts(&forward.listen)?;
    if listen_port == 0 {
        return None;
    }
    Some(blank_candidate(
        TunnelKind::Dynamic,
        listen_address,
        listen_port,
    ))
}

fn blank_candidate(
    kind: TunnelKind,
    listen_address: Option<String>,
    listen_port: u16,
) -> ImportCandidate {
    ImportCandidate {
        host: None,
        kind,
        listen_address,
        listen_port,
        target_host: None,
        target_port: None,
        label: None,
        line: None,
    }
}

fn tcp_parts(endpoint: &Endpoint) -> Option<(Option<String>, u16)> {
    match endpoint {
        Endpoint::Tcp { address, port } => Some((address.clone(), *port)),
        Endpoint::Unix(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(kind: TunnelKind, argument: &str) -> ImportCandidate {
        match parse_forward(kind, argument) {
            ParsedForward::Supported(candidate) => candidate,
            _ => panic!("expected a supported forward for {argument}"),
        }
    }

    #[test]
    fn should_read_local_forward_when_argument_has_three_fields() {
        let found = candidate(TunnelKind::Local, "3333:localhost:80");

        assert_eq!(
            (
                found.listen_port,
                found.target_host.as_deref(),
                found.target_port
            ),
            (3333, Some("localhost"), Some(80))
        );
    }

    #[test]
    fn should_read_bind_address_when_argument_has_four_fields() {
        let found = candidate(TunnelKind::Local, "0.0.0.0:3333:db:80");

        assert_eq!(found.listen_address.as_deref(), Some("0.0.0.0"));
    }

    #[test]
    fn should_have_no_target_when_forward_is_dynamic() {
        let found = candidate(TunnelKind::Dynamic, "1080");

        assert_eq!((found.target_host, found.target_port), (None, None));
    }

    #[test]
    fn should_report_unsupported_when_remote_forward_has_no_target() {
        assert!(matches!(
            parse_forward(TunnelKind::Remote, "9000"),
            ParsedForward::Unsupported
        ));
    }

    #[test]
    fn should_report_unsupported_when_target_is_a_unix_socket() {
        assert!(matches!(
            parse_forward(TunnelKind::Local, "8080:/run/app.sock"),
            ParsedForward::Unsupported
        ));
    }

    #[test]
    fn should_report_malformed_when_argument_is_not_a_forward() {
        assert!(matches!(
            parse_forward(TunnelKind::Local, "nonsense"),
            ParsedForward::Malformed
        ));
    }
}
