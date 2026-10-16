use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use thiserror::Error;
use ulid::Ulid;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_REMOTE_BIND_ADDRESS: &str = "localhost";
const DEFAULT_TARGET_HOST: &str = "127.0.0.1";
const DEFAULT_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const DEFAULT_KEEPALIVE_MAX: u32 = 3;
const DEFAULT_SESSION_IDLE_GRACE: Duration = Duration::from_secs(30);
const DEFAULT_RECONNECT_INITIAL_DELAY: Duration = Duration::from_millis(1000);
const DEFAULT_RECONNECT_MAX_DELAY: Duration = Duration::from_millis(60_000);

/// The identifier of a tunnel: a ULID, stable across renames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TunnelId(Ulid);

impl TunnelId {
    /// Generates a new unique identifier.
    pub fn generate() -> Self {
        Self(Ulid::generate())
    }

    pub(crate) fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }
}

impl fmt::Display for TunnelId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The text is not a valid tunnel identifier.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("'{0}' is not a valid tunnel id (expected a 26-character ULID)")]
pub struct InvalidTunnelId(String);

impl FromStr for TunnelId {
    type Err = InvalidTunnelId;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ulid::from_string(text)
            .map(Self)
            .map_err(|_| InvalidTunnelId(text.to_owned()))
    }
}

/// The direction of a port forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TunnelKind {
    /// `ssh -L`: listens locally, connects to the target from the server.
    Local,
    /// `ssh -R`: listens on the server, connects to the target locally.
    Remote,
    /// `ssh -D`: listens locally as a SOCKS5 proxy.
    Dynamic,
}

/// A configured port forward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelDefinition {
    /// Stable identifier.
    pub id: TunnelId,
    /// Unique, human-readable name; the CLI addresses tunnels by it.
    pub name: String,
    /// The `ssh_config` alias the tunnel goes through.
    pub host: String,
    /// The direction of the forward.
    pub kind: TunnelKind,
    /// The port to listen on.
    pub listen_port: u16,
    /// Overrides the default bind address.
    pub listen_address: Option<String>,
    /// The host the traffic is forwarded to; defaults to `127.0.0.1`.
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to; defaults to the listen port.
    pub target_port: Option<u16>,
    /// Free-form labels; a tunnel can carry several.
    pub tags: Vec<String>,
}

impl TunnelDefinition {
    /// The address to listen on: the override, else the default bind address
    /// for local and dynamic tunnels or the default remote bind address for
    /// remote tunnels.
    pub fn effective_listen_address<'a>(&'a self, settings: &'a Settings) -> &'a str {
        if let Some(address) = &self.listen_address {
            return address;
        }
        match self.kind {
            TunnelKind::Local | TunnelKind::Dynamic => &settings.default_bind_address,
            TunnelKind::Remote => &settings.default_remote_bind_address,
        }
    }

    /// The host the traffic is forwarded to.
    pub fn effective_target_host(&self) -> &str {
        self.target_host.as_deref().unwrap_or(DEFAULT_TARGET_HOST)
    }

    /// The port the traffic is forwarded to.
    pub fn effective_target_port(&self) -> u16 {
        self.target_port.unwrap_or(self.listen_port)
    }
}

/// Application-wide settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Bind address of local and dynamic tunnels without an override.
    pub default_bind_address: String,
    /// Bind address of remote tunnels without an override.
    pub default_remote_bind_address: String,
    /// Delay between keepalive probes; zero disables keepalive.
    pub keepalive_interval: Duration,
    /// Unanswered keepalive probes tolerated before the session is lost.
    pub keepalive_max: u32,
    /// How long an unused session stays open.
    pub session_idle_grace: Duration,
    /// Delay before the first reconnection attempt.
    pub reconnect_initial_delay: Duration,
    /// Upper bound of the reconnection backoff.
    pub reconnect_max_delay: Duration,
    /// Whether the target of a tunnel is probed periodically.
    pub health_probe: bool,
    /// Whether tunnels that were active at the last exit are restarted.
    pub restore_on_launch: bool,
    /// Tags whose tunnels start when the daemon launches.
    pub start_on_launch_tags: Vec<String>,
    /// Whether the daemon starts at login.
    pub launch_at_login: bool,
    /// The local X11 display; `None` means `$DISPLAY`.
    pub x11_display: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_bind_address: DEFAULT_BIND_ADDRESS.to_owned(),
            default_remote_bind_address: DEFAULT_REMOTE_BIND_ADDRESS.to_owned(),
            keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
            keepalive_max: DEFAULT_KEEPALIVE_MAX,
            session_idle_grace: DEFAULT_SESSION_IDLE_GRACE,
            reconnect_initial_delay: DEFAULT_RECONNECT_INITIAL_DELAY,
            reconnect_max_delay: DEFAULT_RECONNECT_MAX_DELAY,
            health_probe: true,
            restore_on_launch: true,
            start_on_launch_tags: Vec::new(),
            launch_at_login: false,
            x11_display: None,
        }
    }
}

/// Per-host overrides that belong to the app rather than to `ssh_config`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostOverrides {
    /// The local X11 display used for this host.
    pub x11_display: Option<String>,
}

/// A command run in an X11-enabled session on a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11Launcher {
    /// The `ssh_config` alias the command runs on.
    pub host: String,
    /// The label shown to the user, unique per host.
    pub name: String,
    /// The command line.
    pub command: String,
}

/// Everything the store persists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoreDocument {
    /// Application-wide settings.
    pub settings: Settings,
    /// The configured tunnels, in file order.
    pub tunnels: Vec<TunnelDefinition>,
    /// Per-host overrides, keyed by `ssh_config` alias.
    pub hosts: BTreeMap<String, HostOverrides>,
    /// The saved X11 launchers.
    pub x11_launchers: Vec<X11Launcher>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::test_support::tunnel;
    use pretty_assertions::assert_eq;

    #[test]
    fn should_use_default_bind_address_when_local_tunnel_has_no_override() {
        let settings = Settings::default();

        assert_eq!(
            tunnel(TunnelKind::Local).effective_listen_address(&settings),
            "127.0.0.1"
        );
    }

    #[test]
    fn should_use_default_bind_address_when_dynamic_tunnel_has_no_override() {
        let settings = Settings::default();

        assert_eq!(
            tunnel(TunnelKind::Dynamic).effective_listen_address(&settings),
            "127.0.0.1"
        );
    }

    #[test]
    fn should_use_default_remote_bind_address_when_remote_tunnel_has_no_override() {
        let settings = Settings::default();

        assert_eq!(
            tunnel(TunnelKind::Remote).effective_listen_address(&settings),
            "localhost"
        );
    }

    #[test]
    fn should_use_custom_default_when_settings_change_the_bind_address() {
        let settings = Settings {
            default_bind_address: "0.0.0.0".to_owned(),
            ..Settings::default()
        };

        assert_eq!(
            tunnel(TunnelKind::Local).effective_listen_address(&settings),
            "0.0.0.0"
        );
    }

    #[test]
    fn should_use_override_when_listen_address_is_set() {
        let settings = Settings::default();
        let definition = TunnelDefinition {
            listen_address: Some("198.51.100.10".to_owned()),
            ..tunnel(TunnelKind::Remote)
        };

        assert_eq!(definition.effective_listen_address(&settings), "198.51.100.10");
    }

    #[test]
    fn should_target_loopback_when_target_host_is_absent() {
        assert_eq!(
            tunnel(TunnelKind::Local).effective_target_host(),
            "127.0.0.1"
        );
    }

    #[test]
    fn should_use_target_host_when_it_is_set() {
        let definition = TunnelDefinition {
            target_host: Some("db.internal".to_owned()),
            ..tunnel(TunnelKind::Local)
        };

        assert_eq!(definition.effective_target_host(), "db.internal");
    }

    #[test]
    fn should_target_listen_port_when_target_port_is_absent() {
        assert_eq!(tunnel(TunnelKind::Local).effective_target_port(), 3333);
    }

    #[test]
    fn should_use_target_port_when_it_is_set() {
        let definition = TunnelDefinition {
            target_port: Some(5432),
            ..tunnel(TunnelKind::Local)
        };

        assert_eq!(definition.effective_target_port(), 5432);
    }

    #[test]
    fn should_round_trip_tunnel_id_through_text() {
        let id = TunnelId::generate();

        assert_eq!(id.to_string().parse::<TunnelId>(), Ok(id));
    }

    #[test]
    fn should_reject_tunnel_id_when_text_is_not_a_ulid() {
        assert!("not-a-ulid".parse::<TunnelId>().is_err());
    }

    #[test]
    fn should_generate_distinct_ids() {
        assert_ne!(TunnelId::generate(), TunnelId::generate());
    }
}
