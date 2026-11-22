//! The address a tunnel is opened at: a pure function of its definition and
//! the settings, shared by every client through the daemon.

use std::net::IpAddr;

use thiserror::Error;

use crate::store::{
    OpenHost, OpenPathProblem, OpenScheme, Settings, TunnelDefinition, TunnelKind,
    find_open_path_problem,
};

const DEFAULT_PATH: &str = "/";
const LOCALHOST_NAME: &str = "localhost";
const IPV4_LOOPBACK: &str = "127.0.0.1";
const IPV6_LOOPBACK: &str = "[::1]";
const WILDCARD_ADDRESSES: [&str; 3] = ["0.0.0.0", "::", "*"];
const LOOPBACK_ADDRESSES: [&str; 3] = ["127.0.0.1", "::1", LOCALHOST_NAME];

/// Why a tunnel has no address to open.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OpenUrlError {
    /// Only local tunnels have something to browse on this machine.
    #[error("only a local tunnel can be opened in a browser")]
    NotOpenable {
        /// The kind of the tunnel.
        kind: TunnelKind,
    },
    /// The tunnel listens on a host name, which is never opened: it could
    /// lead outside the tunnel.
    #[error(
        "the tunnel listens on \"{address}\", a host name: only loopback, wildcard and IP addresses can be opened"
    )]
    UnsupportedListenAddress {
        /// The listen address.
        address: String,
    },
    /// The configured path is not acceptable in a URL.
    #[error("the open path \"{path}\" is invalid: {problem}")]
    InvalidPath {
        /// The path.
        path: String,
        /// What is wrong with it.
        problem: OpenPathProblem,
    },
}

/// Where a tunnel is reached from this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenAddress {
    /// The host as written in a URL: an IPv6 literal is bracketed.
    pub(crate) host: String,
    /// The listen port.
    pub(crate) port: u16,
    /// `{scheme}://{host}:{port}{path}`.
    pub(crate) url: String,
}

/// The address that opens `definition`: the host and port, and the URL
/// `{scheme}://{host}:{port}{path}` for a browser.
///
/// The host follows the tunnel's listen address: an IP literal is used as is,
/// a loopback or wildcard address is replaced by the preferred loopback name
/// (the tunnel's `open.host`, else `Settings::open_host`).
pub(crate) fn build_open_address(
    definition: &TunnelDefinition,
    settings: &Settings,
) -> Result<OpenAddress, OpenUrlError> {
    if definition.kind != TunnelKind::Local {
        return Err(OpenUrlError::NotOpenable {
            kind: definition.kind,
        });
    }
    let path = definition.open.path.as_deref().unwrap_or(DEFAULT_PATH);
    if let Some(problem) = find_open_path_problem(path) {
        return Err(OpenUrlError::InvalidPath {
            path: path.to_owned(),
            problem,
        });
    }
    let preference = definition.open.host.unwrap_or(settings.open_host);
    let listen_address = definition.effective_listen_address(settings);
    let host = host_to_open(listen_address, preference)?;
    let url_host = url_host(&host, listen_address, definition.open.host_name.as_deref());
    let scheme = definition.open.scheme.unwrap_or(OpenScheme::Http);
    let port = definition.listen_port;
    let url = format!("{}://{url_host}:{port}{path}", scheme.as_str());
    Ok(OpenAddress { host, port, url })
}

/// The host of the URL: the configured host name when the tunnel listens on
/// a loopback or wildcard address, else the host to open.
fn url_host(host: &str, listen_address: &str, host_name: Option<&str>) -> String {
    let address = listen_address.trim().trim_matches(['[', ']']);
    match host_name {
        Some(name) if is_loopback_or_wildcard(address) => name.to_owned(),
        _ => host.to_owned(),
    }
}

fn host_to_open(listen_address: &str, preference: OpenHost) -> Result<String, OpenUrlError> {
    let address = listen_address.trim().trim_matches(['[', ']']);
    if is_loopback_or_wildcard(address) {
        return Ok(loopback_host(address, preference));
    }
    match address.parse::<IpAddr>() {
        Ok(IpAddr::V6(ip)) => Ok(format!("[{ip}]")),
        Ok(IpAddr::V4(ip)) => Ok(ip.to_string()),
        Err(_) => Err(OpenUrlError::UnsupportedListenAddress {
            address: address.to_owned(),
        }),
    }
}

fn is_loopback_or_wildcard(address: &str) -> bool {
    address.is_empty()
        || WILDCARD_ADDRESSES.contains(&address)
        || LOOPBACK_ADDRESSES
            .iter()
            .any(|loopback| loopback.eq_ignore_ascii_case(address))
}

fn loopback_host(listen_address: &str, preference: OpenHost) -> String {
    match preference {
        OpenHost::Localhost => LOCALHOST_NAME.to_owned(),
        OpenHost::Ip if is_ipv6(listen_address) => IPV6_LOOPBACK.to_owned(),
        OpenHost::Ip => IPV4_LOOPBACK.to_owned(),
    }
}

fn is_ipv6(address: &str) -> bool {
    address.contains(':')
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::store::{OpenOptions, TunnelId};

    fn local_tunnel(listen_address: Option<&str>) -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            host: "nova".to_owned(),
            kind: TunnelKind::Local,
            listen_port: 3333,
            listen_address: listen_address.map(str::to_owned),
            target_host: None,
            target_port: None,
            tags: Vec::new(),
            open: OpenOptions::default(),
            standby: None,
        }
    }

    fn build_open_url(
        definition: &TunnelDefinition,
        settings: &Settings,
    ) -> Result<String, OpenUrlError> {
        build_open_address(definition, settings).map(|address| address.url)
    }

    fn url_of(definition: &TunnelDefinition) -> Result<String, OpenUrlError> {
        build_open_url(definition, &Settings::default())
    }

    fn url_listening_on(
        listen_address: &str,
        preference: OpenHost,
    ) -> Result<String, OpenUrlError> {
        let settings = Settings {
            open_host: preference,
            ..Settings::default()
        };
        build_open_url(&local_tunnel(Some(listen_address)), &settings)
    }

    #[test]
    fn should_open_http_localhost_on_the_listen_port_by_default() {
        assert_eq!(
            url_of(&local_tunnel(None)),
            Ok("http://localhost:3333/".to_owned())
        );
    }

    #[test]
    fn should_refuse_a_remote_tunnel() {
        let definition = TunnelDefinition {
            kind: TunnelKind::Remote,
            ..local_tunnel(None)
        };

        assert_eq!(
            url_of(&definition),
            Err(OpenUrlError::NotOpenable {
                kind: TunnelKind::Remote
            })
        );
    }

    #[test]
    fn should_refuse_a_dynamic_tunnel() {
        let definition = TunnelDefinition {
            kind: TunnelKind::Dynamic,
            ..local_tunnel(None)
        };

        assert_eq!(
            url_of(&definition),
            Err(OpenUrlError::NotOpenable {
                kind: TunnelKind::Dynamic
            })
        );
    }

    #[test]
    fn should_use_the_scheme_and_path_of_the_tunnel() {
        let definition = TunnelDefinition {
            open: OpenOptions {
                scheme: Some(OpenScheme::Https),
                path: Some("/admin?tab=1".to_owned()),
                host: None,
                with: None,
                host_name: None,
            },
            ..local_tunnel(None)
        };

        assert_eq!(
            url_of(&definition),
            Ok("https://localhost:3333/admin?tab=1".to_owned())
        );
    }

    #[test]
    fn should_refuse_a_path_that_would_change_the_host() {
        let definition = TunnelDefinition {
            open: OpenOptions {
                path: Some("//evil.example".to_owned()),
                ..OpenOptions::default()
            },
            ..local_tunnel(None)
        };

        assert!(matches!(
            url_of(&definition),
            Err(OpenUrlError::InvalidPath {
                problem: OpenPathProblem::StartsWithDoubleSlash,
                ..
            })
        ));
    }

    #[test]
    fn should_open_the_ipv4_loopback_when_the_preference_is_ip() {
        assert_eq!(
            url_listening_on("127.0.0.1", OpenHost::Ip),
            Ok("http://127.0.0.1:3333/".to_owned())
        );
    }

    #[test]
    fn should_open_the_ipv6_loopback_when_the_preference_is_ip_and_the_listen_address_is_ipv6() {
        assert_eq!(
            url_listening_on("::1", OpenHost::Ip),
            Ok("http://[::1]:3333/".to_owned())
        );
    }

    #[test]
    fn should_open_the_ipv6_loopback_when_the_listen_address_is_the_ipv6_wildcard() {
        assert_eq!(
            url_listening_on("::", OpenHost::Ip),
            Ok("http://[::1]:3333/".to_owned())
        );
    }

    #[test]
    fn should_open_the_ipv4_loopback_when_the_listen_address_is_the_ipv4_wildcard() {
        assert_eq!(
            url_listening_on("0.0.0.0", OpenHost::Ip),
            Ok("http://127.0.0.1:3333/".to_owned())
        );
    }

    #[test]
    fn should_open_localhost_when_the_preference_is_localhost_whatever_the_loopback_spelling() {
        let urls: Vec<_> = ["127.0.0.1", "::1", "localhost", "0.0.0.0", "::", "", "*"]
            .iter()
            .map(|address| url_listening_on(address, OpenHost::Localhost))
            .collect();

        assert_eq!(urls, vec![Ok("http://localhost:3333/".to_owned()); 7]);
    }

    #[test]
    fn should_open_the_ipv4_loopback_when_the_listen_address_is_empty_and_the_preference_is_ip() {
        assert_eq!(
            url_listening_on("", OpenHost::Ip),
            Ok("http://127.0.0.1:3333/".to_owned())
        );
    }

    #[test]
    fn should_open_the_ipv4_loopback_when_the_listen_address_is_a_star_and_the_preference_is_ip() {
        assert_eq!(
            url_listening_on("*", OpenHost::Ip),
            Ok("http://127.0.0.1:3333/".to_owned())
        );
    }

    #[test]
    fn should_prefer_the_tunnel_host_over_the_settings() {
        let definition = TunnelDefinition {
            open: OpenOptions {
                host: Some(OpenHost::Ip),
                ..OpenOptions::default()
            },
            ..local_tunnel(None)
        };

        assert_eq!(url_of(&definition), Ok("http://127.0.0.1:3333/".to_owned()));
    }

    #[test]
    fn should_open_the_listen_address_when_it_is_a_specific_ipv4() {
        assert_eq!(
            url_listening_on("192.0.2.120", OpenHost::Localhost),
            Ok("http://192.0.2.120:3333/".to_owned())
        );
    }

    #[test]
    fn should_bracket_a_specific_ipv6_listen_address() {
        assert_eq!(
            url_listening_on("fe80::1", OpenHost::Localhost),
            Ok("http://[fe80::1]:3333/".to_owned())
        );
    }

    #[test]
    fn should_treat_a_bracketed_ipv6_loopback_like_the_plain_one() {
        assert_eq!(
            url_listening_on("[::1]", OpenHost::Ip),
            Ok("http://[::1]:3333/".to_owned())
        );
    }

    #[test]
    fn should_refuse_a_host_name_listen_address() {
        assert_eq!(
            url_listening_on("example.com", OpenHost::Localhost),
            Err(OpenUrlError::UnsupportedListenAddress {
                address: "example.com".to_owned()
            })
        );
    }

    #[test]
    fn should_use_the_default_bind_address_of_the_settings() {
        let settings = Settings {
            default_bind_address: "198.51.100.50".to_owned(),
            ..Settings::default()
        };

        assert_eq!(
            build_open_url(&local_tunnel(None), &settings),
            Ok("http://198.51.100.50:3333/".to_owned())
        );
    }

    const HOST_NAME: &str = "my-app.localhost";

    fn definition_with_host_name(listen_address: Option<&str>) -> TunnelDefinition {
        TunnelDefinition {
            open: OpenOptions {
                host_name: Some(HOST_NAME.to_owned()),
                ..OpenOptions::default()
            },
            ..local_tunnel(listen_address)
        }
    }

    #[test]
    fn should_open_the_host_name_when_the_tunnel_listens_on_loopback() {
        assert_eq!(
            url_of(&definition_with_host_name(Some("127.0.0.1"))),
            Ok("http://my-app.localhost:3333/".to_owned())
        );
    }

    #[test]
    fn should_open_the_host_name_when_the_tunnel_listens_on_a_wildcard_address() {
        assert_eq!(
            url_of(&definition_with_host_name(Some("0.0.0.0"))),
            Ok("http://my-app.localhost:3333/".to_owned())
        );
    }

    #[test]
    fn should_ignore_the_host_name_when_the_tunnel_listens_on_a_non_loopback_address() {
        assert_eq!(
            url_of(&definition_with_host_name(Some("192.0.2.120"))),
            Ok("http://192.0.2.120:3333/".to_owned())
        );
    }

    #[test]
    fn should_keep_the_loopback_host_for_programs_when_a_host_name_is_set() {
        let address = build_open_address(
            &definition_with_host_name(Some("127.0.0.1")),
            &Settings::default(),
        )
        .unwrap();

        assert_eq!(address.host, "localhost");
    }

    #[test]
    fn should_open_the_loopback_name_when_no_host_name_is_set() {
        assert_eq!(
            url_of(&local_tunnel(Some("127.0.0.1"))),
            Ok("http://localhost:3333/".to_owned())
        );
    }
}
