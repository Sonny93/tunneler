use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use thiserror::Error;
use ulid::Ulid;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_REMOTE_BIND_ADDRESS: &str = "localhost";
const DEFAULT_TARGET_HOST: &str = "127.0.0.1";
const LOOPBACK_HOSTS: [&str; 3] = [DEFAULT_TARGET_HOST, "localhost", "::1"];
const DEFAULT_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const DEFAULT_KEEPALIVE_MAX: u32 = 3;
const DEFAULT_SESSION_IDLE_GRACE: Duration = Duration::from_secs(30);
const DEFAULT_RECONNECT_INITIAL_DELAY: Duration = Duration::from_millis(1000);
const DEFAULT_RECONNECT_MAX_DELAY: Duration = Duration::from_millis(60_000);
const DEFAULT_STANDBY_IDLE: Duration = Duration::from_secs(300);
/// The quietest volume of the sounds of the daemon, in percent.
pub const MIN_STANDBY_VOLUME: u8 = 5;
/// The loudest volume of the sounds of the daemon, in percent.
pub const MAX_STANDBY_VOLUME: u8 = 100;
const DEFAULT_STANDBY_VOLUME: u8 = 50;

/// The identifier of a tunnel: a ULID, stable across edits.
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

/// The URL scheme used to open a tunnel in the browser.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum OpenScheme {
    /// `http://`.
    #[default]
    Http,
    /// `https://`.
    Https,
}

impl OpenScheme {
    /// The scheme as written in a URL and in the store file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }
}

/// Which loopback name the browser is pointed at when a tunnel listens on a
/// loopback or wildcard address.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum OpenHost {
    /// The name `localhost`.
    #[default]
    Localhost,
    /// The loopback IP literal: `127.0.0.1`, or `[::1]` when the tunnel
    /// listens on IPv6.
    Ip,
}

impl OpenHost {
    /// The preference as written in the store file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Localhost => "localhost",
            Self::Ip => "ip",
        }
    }
}

/// What a tunnel without `open.with` opens with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum OpenDefault {
    /// The first opener whose ports contain the target port, else the browser.
    #[default]
    Suggested,
    /// Always the browser.
    Browser,
}

impl OpenDefault {
    /// The preference as written in the store file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Suggested => "suggested",
            Self::Browser => "browser",
        }
    }
}

/// How the daemon signals that a standby tunnel woke up or failed to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StandbyNotify {
    /// No signal.
    None,
    /// A silent desktop notification.
    #[default]
    Visual,
    /// A sound played by the daemon.
    Sound,
    /// Both.
    Both,
}

impl StandbyNotify {
    /// The mode as written in the store file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Visual => "visual",
            Self::Sound => "sound",
            Self::Both => "both",
        }
    }
}

/// The sound the daemon plays when a standby tunnel wakes up or fails to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StandbySound {
    /// The sound of the operating system, which its sound scheme may mute.
    System,
    /// Two bell notes.
    Chime,
    /// Two marimba notes.
    Marimba,
    /// Two plucked-string notes.
    #[default]
    Pluck,
}

impl StandbySound {
    /// The sound as written in the store file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Chime => "chime",
            Self::Marimba => "marimba",
            Self::Pluck => "pluck",
        }
    }
}

/// The language of the UI, the tray and the notifications.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Language {
    /// The language of the operating system.
    #[default]
    System,
    /// English.
    English,
    /// French.
    French,
}

impl Language {
    /// The language as written in the store file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::English => "en",
            Self::French => "fr",
        }
    }
}

/// How a tunnel is opened; every field is optional and falls back to a
/// default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenOptions {
    /// The URL scheme; `http` when absent.
    pub scheme: Option<OpenScheme>,
    /// The URL path; `/` when absent.
    pub path: Option<String>,
    /// Overrides `Settings::open_host` for this tunnel.
    pub host: Option<OpenHost>,
    /// What opens the tunnel: [`BROWSER_OPENER_ID`] or the id of an opener.
    /// Absent means the suggestion for the target port, else the browser.
    pub with: Option<String>,
}

impl OpenOptions {
    /// Whether no option is set.
    pub fn is_empty(&self) -> bool {
        self.scheme.is_none() && self.path.is_none() && self.host.is_none() && self.with.is_none()
    }
}

/// The reserved id that selects the browser instead of an opener.
pub const BROWSER_OPENER_ID: &str = "browser";

/// A program that opens a tunnel, started with explicit arguments and never
/// through a shell. `{host}`, `{port}` and `{url}` in `args` are replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opener {
    /// Stable identifier: lowercase letters, digits and dashes.
    pub id: String,
    /// The label shown to the user.
    pub name: String,
    /// The program to run.
    pub command: String,
    /// The arguments, one per entry.
    pub args: Vec<String>,
    /// The target ports this program is suggested for.
    pub ports: Vec<u16>,
}

/// A configured port forward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelDefinition {
    /// Stable identifier.
    pub id: TunnelId,
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
    /// How the tunnel is opened in the browser.
    pub open: OpenOptions,
    /// Overrides `Settings::standby` for this tunnel; ignored for remote
    /// tunnels.
    pub standby: Option<bool>,
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

    /// Whether the tunnel holds its local port without an SSH session until
    /// a client connects: the override, else the setting. Never for a remote
    /// tunnel, whose listener lives on the server and needs a session.
    pub fn is_standby(&self, settings: &Settings) -> bool {
        match self.kind {
            TunnelKind::Local | TunnelKind::Dynamic => self.standby.unwrap_or(settings.standby),
            TunnelKind::Remote => false,
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

    /// What the user sees instead of a name: where the tunnel listens and what
    /// it reaches, such as `3333 → nova:3333`, `nova:8080 → 127.0.0.1:3000`
    /// or `1080 (SOCKS via nova)`.
    pub fn label(&self) -> String {
        let target_host = self.effective_target_host();
        let target_port = self.effective_target_port();
        match self.kind {
            TunnelKind::Local if is_loopback(target_host) => {
                format!("{} → {}:{target_port}", self.listen_port, self.host)
            }
            TunnelKind::Local => format!(
                "{} → {target_host}:{target_port} via {}",
                self.listen_port, self.host
            ),
            TunnelKind::Remote => format!(
                "{}:{} → {target_host}:{target_port}",
                self.host, self.listen_port
            ),
            TunnelKind::Dynamic => format!("{} (SOCKS via {})", self.listen_port, self.host),
        }
    }
}

fn is_loopback(host: &str) -> bool {
    LOOPBACK_HOSTS.contains(&host)
}

/// Application-wide settings.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent user setting"
)]
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
    /// Whether tunnels that were active at the last exit are restarted.
    pub restore_on_launch: bool,
    /// Tags whose tunnels start when the daemon launches.
    pub start_on_launch_tags: Vec<String>,
    /// Whether the daemon starts at login.
    pub launch_at_login: bool,
    /// Whether passphrases the user asked to remember are kept in the OS keyring.
    pub use_os_keyring: bool,
    /// The local X11 display; `None` means `$DISPLAY`.
    pub x11_display: Option<String>,
    /// Which loopback name opened tunnels use unless they say otherwise.
    pub open_host: OpenHost,
    /// What tunnels without an application of their own open with.
    pub open_default: OpenDefault,
    /// Whether local and dynamic tunnels without an override wait in standby.
    pub standby: bool,
    /// How long a standby tunnel stays connected without any connection
    /// before it releases its session.
    pub standby_idle: Duration,
    /// How a wake-up from standby, or its failure, is signalled.
    pub standby_notify: StandbyNotify,
    /// The sound played when a wake-up, or its failure, is signalled by sound.
    pub standby_sound: StandbySound,
    /// The volume of the synthesized sounds, in percent between
    /// [`MIN_STANDBY_VOLUME`] and [`MAX_STANDBY_VOLUME`]. The sound of the
    /// system follows the system volume instead.
    pub standby_volume: u8,
    /// The language of the UI, the tray and the notifications.
    pub language: Language,
    /// Whether the UI looks for a newer release when it opens.
    pub check_for_updates: bool,
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
            restore_on_launch: true,
            start_on_launch_tags: Vec::new(),
            launch_at_login: false,
            use_os_keyring: false,
            x11_display: None,
            open_host: OpenHost::default(),
            open_default: OpenDefault::default(),
            standby: false,
            standby_idle: DEFAULT_STANDBY_IDLE,
            standby_notify: StandbyNotify::default(),
            standby_sound: StandbySound::default(),
            standby_volume: DEFAULT_STANDBY_VOLUME,
            language: Language::default(),
            check_for_updates: true,
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
    /// The openers the user added.
    pub openers: Vec<Opener>,
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

        assert_eq!(
            definition.effective_listen_address(&settings),
            "198.51.100.10"
        );
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
    fn should_label_a_local_tunnel_by_its_ports_and_host() {
        assert_eq!(tunnel(TunnelKind::Local).label(), "3333 → nova:3333");
    }

    #[test]
    fn should_label_a_local_tunnel_with_its_target_when_the_target_is_not_the_ssh_host() {
        let definition = TunnelDefinition {
            target_host: Some("db.internal".to_owned()),
            target_port: Some(5432),
            ..tunnel(TunnelKind::Local)
        };

        assert_eq!(definition.label(), "3333 → db.internal:5432 via nova");
    }

    #[test]
    fn should_label_a_remote_tunnel_by_its_server_side_port_and_target() {
        assert_eq!(
            tunnel(TunnelKind::Remote).label(),
            "nova:3333 → 127.0.0.1:3333"
        );
    }

    #[test]
    fn should_label_a_dynamic_tunnel_as_a_socks_proxy() {
        assert_eq!(tunnel(TunnelKind::Dynamic).label(), "3333 (SOCKS via nova)");
    }

    #[test]
    fn should_not_be_standby_when_neither_the_tunnel_nor_the_settings_ask_for_it() {
        assert!(!tunnel(TunnelKind::Local).is_standby(&Settings::default()));
    }

    #[test]
    fn should_be_standby_when_the_setting_asks_for_it() {
        let settings = Settings {
            standby: true,
            ..Settings::default()
        };

        assert!(tunnel(TunnelKind::Local).is_standby(&settings));
    }

    #[test]
    fn should_be_standby_when_the_tunnel_asks_for_it_against_the_setting() {
        let definition = TunnelDefinition {
            standby: Some(true),
            ..tunnel(TunnelKind::Dynamic)
        };

        assert!(definition.is_standby(&Settings::default()));
    }

    #[test]
    fn should_not_be_standby_when_the_tunnel_refuses_it_against_the_setting() {
        let settings = Settings {
            standby: true,
            ..Settings::default()
        };
        let definition = TunnelDefinition {
            standby: Some(false),
            ..tunnel(TunnelKind::Local)
        };

        assert!(!definition.is_standby(&settings));
    }

    #[test]
    fn should_never_be_standby_when_the_tunnel_is_remote() {
        let settings = Settings {
            standby: true,
            ..Settings::default()
        };
        let definition = TunnelDefinition {
            standby: Some(true),
            ..tunnel(TunnelKind::Remote)
        };

        assert!(!definition.is_standby(&settings));
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
