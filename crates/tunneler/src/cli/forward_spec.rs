//! Reads the `-L`, `-R` and `-D` specifications of `tunneler up` with the
//! OpenSSH syntax, and finds the saved tunnel a specification stands for.
//!
//! The syntax is the one of `ssh`, parsed by the same reader as the
//! `ssh_config` forwards. On top of it, `-L` and `-R` accept a bare port: the
//! tunnel then listens on that port and reaches the same port of the host,
//! like the defaults of the tunnel form of the app.

use ipc::protocol::{SnapshotDto, TemporaryTunnelSpecDto, TunnelDto, TunnelKindDto};
use ssh_config::{
    Endpoint, ForwardSpec, RemoteForward, parse_dynamic_forward_argument,
    parse_local_forward_argument, parse_remote_forward_argument,
};
use thiserror::Error;

const DEFAULT_TARGET_HOST: &str = "127.0.0.1";

/// Which option carried the specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardOption {
    /// `-L`.
    Local,
    /// `-R`.
    Remote,
    /// `-D`.
    Dynamic,
}

impl ForwardOption {
    const fn flag(self) -> &'static str {
        match self {
            Self::Local => "-L",
            Self::Remote => "-R",
            Self::Dynamic => "-D",
        }
    }

    const fn syntax(self) -> &'static str {
        match self {
            Self::Local | Self::Remote => "[bind_address:]port[:host:hostport]",
            Self::Dynamic => "[bind_address:]port",
        }
    }

    const fn kind(self) -> TunnelKindDto {
        match self {
            Self::Local => TunnelKindDto::Local,
            Self::Remote => TunnelKindDto::Remote,
            Self::Dynamic => TunnelKindDto::Dynamic,
        }
    }
}

/// Why a specification cannot become a tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ForwardSpecError {
    /// The text does not follow the syntax of the option.
    #[error("\"{specification}\" is not a valid {flag} specification (expected {syntax})")]
    Invalid {
        /// The option, such as `-L`.
        flag: &'static str,
        /// The text as typed.
        specification: String,
        /// The expected syntax.
        syntax: &'static str,
    },
    /// The specification uses a Unix socket, which tunnels cannot carry.
    #[error("\"{specification}\" uses a Unix socket, which tunnels do not support")]
    UnixSocket {
        /// The text as typed.
        specification: String,
    },
    /// `-R port` without a target asks for a SOCKS proxy on the remote side.
    #[error(
        "\"{specification}\" asks for a SOCKS proxy on the remote side, which tunnels do not support"
    )]
    RemoteSocksProxy {
        /// The text as typed.
        specification: String,
    },
}

/// A forward asked for on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardRequest {
    /// The direction of the forward.
    pub kind: TunnelKindDto,
    /// The bind address, when one was written.
    pub listen_address: Option<String>,
    /// The port to listen on.
    pub listen_port: u16,
    /// The host the traffic goes to, when one was written.
    pub target_host: Option<String>,
    /// The port the traffic goes to, when one was written.
    pub target_port: Option<u16>,
}

impl ForwardRequest {
    /// Reads the argument of `-L`, `-R` or `-D`.
    ///
    /// # Errors
    ///
    /// Fails when the text is not a TCP forward in the syntax of the option.
    pub fn parse(option: ForwardOption, specification: &str) -> Result<Self, ForwardSpecError> {
        let invalid = || ForwardSpecError::Invalid {
            flag: option.flag(),
            specification: specification.to_owned(),
            syntax: option.syntax(),
        };
        if option != ForwardOption::Dynamic
            && let Some(port) = parse_bare_port(specification)
        {
            return Ok(Self::listening_on(option, port));
        }
        let request = match option {
            ForwardOption::Local => parse_local_forward_argument(specification)
                .map_err(|_| invalid())
                .and_then(|spec| Self::from_fixed(option, &spec, specification))?,
            ForwardOption::Remote => match parse_remote_forward_argument(specification) {
                Ok(RemoteForward::Fixed(spec)) => Self::from_fixed(option, &spec, specification)?,
                Ok(RemoteForward::Dynamic(_)) => {
                    return Err(ForwardSpecError::RemoteSocksProxy {
                        specification: specification.to_owned(),
                    });
                }
                Err(_) => return Err(invalid()),
            },
            ForwardOption::Dynamic => parse_dynamic_forward_argument(specification)
                .map_err(|_| invalid())
                .and_then(|forward| {
                    let (listen_address, listen_port) = tcp_parts(&forward.listen, specification)?;
                    Ok(Self {
                        listen_address,
                        ..Self::listening_on(option, listen_port)
                    })
                })?,
        };
        if request.listen_port == 0 {
            return Err(invalid());
        }
        Ok(request)
    }

    fn listening_on(option: ForwardOption, listen_port: u16) -> Self {
        Self {
            kind: option.kind(),
            listen_address: None,
            listen_port,
            target_host: None,
            target_port: None,
        }
    }

    fn from_fixed(
        option: ForwardOption,
        spec: &ForwardSpec,
        specification: &str,
    ) -> Result<Self, ForwardSpecError> {
        let (listen_address, listen_port) = tcp_parts(&spec.listen, specification)?;
        let (target_host, target_port) = tcp_parts(&spec.target, specification)?;
        Ok(Self {
            listen_address,
            target_host,
            target_port: Some(target_port),
            ..Self::listening_on(option, listen_port)
        })
    }

    /// The definition of the temporary tunnel through `host`.
    pub fn into_temporary_spec(self, host: &str) -> TemporaryTunnelSpecDto {
        TemporaryTunnelSpecDto {
            host: host.to_owned(),
            kind: self.kind,
            listen_port: self.listen_port,
            listen_address: self.listen_address,
            target_host: self.target_host,
            target_port: self.target_port,
        }
    }

    /// The tunnel through `host` that does what this request asks for: same
    /// kind, listen port and target (after defaults), and the same bind
    /// address when the request names one.
    pub fn find_in<'a>(&self, host: &str, snapshot: &'a SnapshotDto) -> Option<&'a TunnelDto> {
        snapshot
            .tunnels
            .iter()
            .find(|tunnel| self.is_equivalent_to(host, tunnel))
    }

    fn is_equivalent_to(&self, host: &str, tunnel: &TunnelDto) -> bool {
        let spec = &tunnel.definition.spec;
        spec.kind == self.kind
            && spec.host.eq_ignore_ascii_case(host)
            && spec.listen_port == self.listen_port
            && self.has_same_bind_address(spec.listen_address.as_deref())
            && self.has_same_target(spec.target_host.as_deref(), spec.target_port)
    }

    fn has_same_bind_address(&self, other: Option<&str>) -> bool {
        match (&self.listen_address, other) {
            (Some(wanted), Some(existing)) => wanted.eq_ignore_ascii_case(existing),
            (Some(_), None) => false,
            (None, _) => true,
        }
    }

    fn has_same_target(&self, other_host: Option<&str>, other_port: Option<u16>) -> bool {
        if self.kind == TunnelKindDto::Dynamic {
            return true;
        }
        let wanted_host = self.target_host.as_deref().unwrap_or(DEFAULT_TARGET_HOST);
        let existing_host = other_host.unwrap_or(DEFAULT_TARGET_HOST);
        let wanted_port = self.target_port.unwrap_or(self.listen_port);
        let existing_port = other_port.unwrap_or(self.listen_port);
        wanted_host.eq_ignore_ascii_case(existing_host) && wanted_port == existing_port
    }
}

fn parse_bare_port(text: &str) -> Option<u16> {
    let is_number = !text.is_empty() && text.chars().all(|character| character.is_ascii_digit());
    if !is_number {
        return None;
    }
    text.parse::<u16>().ok().filter(|port| *port != 0)
}

fn tcp_parts(
    endpoint: &Endpoint,
    specification: &str,
) -> Result<(Option<String>, u16), ForwardSpecError> {
    match endpoint {
        Endpoint::Tcp { address, port } => Ok((address.clone(), *port)),
        Endpoint::Unix(_) => Err(ForwardSpecError::UnixSocket {
            specification: specification.to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use ipc::protocol::{
        MetricsDto, SettingsDto, TargetHealthDto, TrayStatusDto, TunnelDefinitionDto,
        TunnelSpecDto, TunnelStateDto,
    };
    use pretty_assertions::assert_eq;

    use super::*;

    fn parse(option: ForwardOption, specification: &str) -> ForwardRequest {
        ForwardRequest::parse(option, specification).unwrap()
    }

    #[test]
    fn should_listen_and_reach_the_same_port_when_local_forward_is_a_bare_port() {
        assert_eq!(
            parse(ForwardOption::Local, "5000"),
            ForwardRequest {
                kind: TunnelKindDto::Local,
                listen_address: None,
                listen_port: 5000,
                target_host: None,
                target_port: None,
            }
        );
    }

    #[test]
    fn should_read_the_target_when_local_forward_has_three_fields() {
        let request = parse(ForwardOption::Local, "5000:db.internal:5432");

        assert_eq!(
            (
                request.listen_port,
                request.target_host,
                request.target_port
            ),
            (5000, Some("db.internal".to_owned()), Some(5432))
        );
    }

    #[test]
    fn should_read_the_bind_address_when_local_forward_has_four_fields() {
        let request = parse(ForwardOption::Local, "0.0.0.0:5000:localhost:80");

        assert_eq!(
            (
                request.listen_address,
                request.listen_port,
                request.target_port
            ),
            (Some("0.0.0.0".to_owned()), 5000, Some(80))
        );
    }

    #[test]
    fn should_read_bracketed_ipv6_addresses_when_local_forward_has_them() {
        let request = parse(ForwardOption::Local, "[::1]:5000:[fe80::1]:80");

        assert_eq!(
            (request.listen_address, request.target_host),
            (Some("::1".to_owned()), Some("fe80::1".to_owned()))
        );
    }

    #[test]
    fn should_read_a_remote_forward_when_it_has_a_target() {
        let request = parse(ForwardOption::Remote, "9000:localhost:3000");

        assert_eq!(
            (request.kind, request.listen_port, request.target_port),
            (TunnelKindDto::Remote, 9000, Some(3000))
        );
    }

    #[test]
    fn should_read_a_remote_bare_port_as_the_same_port_locally() {
        let request = parse(ForwardOption::Remote, "9000");

        assert_eq!(
            (request.kind, request.listen_port, request.target_host),
            (TunnelKindDto::Remote, 9000, None)
        );
    }

    #[test]
    fn should_read_the_bind_address_when_remote_forward_has_one() {
        let request = parse(ForwardOption::Remote, "0.0.0.0:9000:localhost:3000");

        assert_eq!(request.listen_address.as_deref(), Some("0.0.0.0"));
    }

    #[test]
    fn should_read_a_dynamic_forward_when_it_is_a_bare_port() {
        let request = parse(ForwardOption::Dynamic, "1080");

        assert_eq!(
            (request.kind, request.listen_port, request.target_port),
            (TunnelKindDto::Dynamic, 1080, None)
        );
    }

    #[test]
    fn should_read_the_bind_address_when_dynamic_forward_has_one() {
        let request = parse(ForwardOption::Dynamic, "[::1]:1080");

        assert_eq!(request.listen_address.as_deref(), Some("::1"));
    }

    #[test]
    fn should_refuse_a_dynamic_forward_when_it_names_a_target() {
        assert!(ForwardRequest::parse(ForwardOption::Dynamic, "1080:host:80").is_err());
    }

    #[test]
    fn should_refuse_port_zero() {
        assert!(ForwardRequest::parse(ForwardOption::Local, "0").is_err());
    }

    #[test]
    fn should_refuse_a_port_out_of_range() {
        assert!(ForwardRequest::parse(ForwardOption::Local, "70000").is_err());
    }

    #[test]
    fn should_refuse_nonsense() {
        let error = ForwardRequest::parse(ForwardOption::Local, "nonsense").unwrap_err();

        assert_eq!(
            error.to_string(),
            "\"nonsense\" is not a valid -L specification (expected [bind_address:]port[:host:hostport])"
        );
    }

    #[test]
    fn should_refuse_a_unix_socket_target() {
        let error = ForwardRequest::parse(ForwardOption::Local, "8080:/run/app.sock").unwrap_err();

        assert!(matches!(error, ForwardSpecError::UnixSocket { .. }));
    }

    #[test]
    fn should_refuse_a_remote_socks_proxy_when_a_remote_forward_has_a_bind_address_only() {
        let error = ForwardRequest::parse(ForwardOption::Remote, "0.0.0.0:9000").unwrap_err();

        assert!(matches!(error, ForwardSpecError::RemoteSocksProxy { .. }));
    }

    fn tunnel(spec: TunnelSpecDto) -> TunnelDto {
        TunnelDto {
            definition: TunnelDefinitionDto {
                id: "01JA".to_owned(),
                spec,
            },
            state: TunnelStateDto::Stopped,
            health: TargetHealthDto::Unknown,
            metrics: MetricsDto::default(),
            is_temporary: false,
            default_opener: ipc::protocol::DefaultOpenerDto {
                id: "browser".to_owned(),
                name: "Browser".to_owned(),
            },
        }
    }

    fn local_spec(
        listen_port: u16,
        target_host: Option<&str>,
        target_port: Option<u16>,
    ) -> TunnelSpecDto {
        TunnelSpecDto {
            host: "nova".to_owned(),
            kind: TunnelKindDto::Local,
            listen_port,
            listen_address: None,
            target_host: target_host.map(str::to_owned),
            target_port,
            tags: Vec::new(),
            open: ipc::protocol::OpenOptionsDto::default(),
            standby: None,
        }
    }

    fn snapshot_with(tunnels: Vec<TunnelDto>) -> SnapshotDto {
        SnapshotDto {
            tunnels,
            settings: SettingsDto {
                default_bind_address: "127.0.0.1".to_owned(),
                default_remote_bind_address: "localhost".to_owned(),
                keepalive_interval_ms: 0,
                keepalive_max: 3,
                session_idle_grace_ms: 0,
                reconnect_initial_delay_ms: 0,
                reconnect_max_delay_ms: 0,
                restore_on_launch: false,
                start_on_launch_tags: Vec::new(),
                launch_at_login: false,
                use_os_keyring: false,
                x11_display: None,
                open_host: ipc::protocol::OpenHostDto::Localhost,
                standby: false,
                standby_idle_s: 300,
                standby_notify: ipc::protocol::StandbyNotifyDto::Visual,
                check_for_updates: true,
            },
            tray: TrayStatusDto::default(),
            externals: Vec::new(),
            x11_anchors: Vec::new(),
            x11_launchers: Vec::new(),
            openers: Vec::new(),
        }
    }

    #[test]
    fn should_find_the_saved_tunnel_when_the_defaults_match() {
        let snapshot = snapshot_with(vec![tunnel(local_spec(5000, None, None))]);

        let found = parse(ForwardOption::Local, "5000").find_in("nova", &snapshot);

        assert!(found.is_some());
    }

    #[test]
    fn should_find_the_saved_tunnel_when_the_target_is_written_with_the_defaults() {
        let snapshot = snapshot_with(vec![tunnel(local_spec(5000, None, None))]);

        let found = parse(ForwardOption::Local, "5000:127.0.0.1:5000").find_in("nova", &snapshot);

        assert!(found.is_some());
    }

    #[test]
    fn should_not_find_a_tunnel_when_the_target_differs() {
        let snapshot = snapshot_with(vec![tunnel(local_spec(5000, None, None))]);

        let found = parse(ForwardOption::Local, "5000:db:5432").find_in("nova", &snapshot);

        assert!(found.is_none());
    }

    #[test]
    fn should_not_find_a_tunnel_when_the_host_differs() {
        let snapshot = snapshot_with(vec![tunnel(local_spec(5000, None, None))]);

        let found = parse(ForwardOption::Local, "5000").find_in("other", &snapshot);

        assert!(found.is_none());
    }

    #[test]
    fn should_not_find_a_tunnel_when_the_kind_differs() {
        let snapshot = snapshot_with(vec![tunnel(local_spec(5000, None, None))]);

        let found = parse(ForwardOption::Remote, "5000").find_in("nova", &snapshot);

        assert!(found.is_none());
    }

    #[test]
    fn should_not_find_a_tunnel_when_the_bind_address_differs() {
        let snapshot = snapshot_with(vec![tunnel(local_spec(5000, None, None))]);

        let found =
            parse(ForwardOption::Local, "0.0.0.0:5000:127.0.0.1:5000").find_in("nova", &snapshot);

        assert!(found.is_none());
    }

    #[test]
    fn should_find_a_dynamic_tunnel_by_its_listen_port() {
        let mut spec = local_spec(1080, None, None);
        spec.kind = TunnelKindDto::Dynamic;
        let snapshot = snapshot_with(vec![tunnel(spec)]);

        let found = parse(ForwardOption::Dynamic, "1080").find_in("nova", &snapshot);

        assert!(found.is_some());
    }

    #[test]
    fn should_build_the_temporary_definition_through_the_host() {
        let spec = parse(ForwardOption::Local, "5000:db:5432").into_temporary_spec("nova");

        assert_eq!(
            (
                spec.host,
                spec.listen_port,
                spec.target_host,
                spec.target_port
            ),
            ("nova".to_owned(), 5000, Some("db".to_owned()), Some(5432))
        );
    }
}
