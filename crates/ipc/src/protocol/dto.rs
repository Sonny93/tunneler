use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The default of `standby_idle_s`, in seconds.
pub const DEFAULT_STANDBY_IDLE_S: u64 = 300;

/// A point in time: milliseconds since the Unix epoch, UTC.
pub type EpochMillis = u64;

/// The direction of a port forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TunnelKindDto {
    /// `ssh -L`.
    Local,
    /// `ssh -R`.
    Remote,
    /// `ssh -D`.
    Dynamic,
}

/// The URL scheme a tunnel is opened with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpenSchemeDto {
    /// `http://`.
    Http,
    /// `https://`.
    Https,
}

/// Which loopback name a tunnel is opened at.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpenHostDto {
    /// The name `localhost`.
    #[default]
    Localhost,
    /// The loopback IP address.
    Ip,
}

/// How the daemon signals that a standby tunnel woke up or failed to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StandbyNotifyDto {
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

/// The sound the daemon plays when a standby tunnel wakes up or fails to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StandbySoundDto {
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

/// The language of the UI, the tray and the notifications.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LanguageDto {
    /// The language of the operating system.
    #[default]
    System,
    /// English.
    En,
    /// French.
    Fr,
}

/// What a sound announces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SoundEventDto {
    /// A standby tunnel woke up.
    Woke,
    /// A standby tunnel could not wake up.
    WakeFailed,
}

/// How a tunnel is opened; every field is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOptionsDto {
    /// The URL scheme; `http` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<OpenSchemeDto>,
    /// The URL path; `/` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Overrides the general `openHost` setting for this tunnel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<OpenHostDto>,
    /// What opens the tunnel: `browser` or the id of an opener; absent means
    /// the suggestion for the target port, else the browser.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with: Option<String>,
}

impl OpenOptionsDto {
    /// Whether no option is set.
    pub fn is_empty(&self) -> bool {
        self.scheme.is_none() && self.path.is_none() && self.host.is_none() && self.with.is_none()
    }
}

/// A program that opens a tunnel. It is run without a shell, one argument per
/// entry, with `{host}`, `{port}` and `{url}` replaced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenerSpecDto {
    /// Lowercase letters, digits and dashes; `browser` is reserved.
    pub id: String,
    /// The label shown to the user.
    pub name: String,
    /// The program to run.
    pub command: String,
    /// The arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// The target ports the program is suggested for.
    #[serde(default)]
    pub ports: Vec<u16>,
}

/// Where an available opener comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpenerSourceDto {
    /// A built-in preset whose program was found on this machine.
    Detected,
    /// An opener the user added.
    User,
}

/// An opener that can be used now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableOpenerDto {
    /// The opener, flattened into the same JSON object.
    #[serde(flatten)]
    pub spec: OpenerSpecDto,
    /// Where it comes from.
    pub source: OpenerSourceDto,
}

/// What a tunnel opens with when nothing is asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultOpenerDto {
    /// `browser` or the id of an opener.
    pub id: String,
    /// The name of the opener; its id when it is not available anymore.
    pub name: String,
}

/// The editable part of a tunnel definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelSpecDto {
    /// The `ssh_config` alias the tunnel goes through.
    pub host: String,
    /// The direction of the forward.
    pub kind: TunnelKindDto,
    /// The port to listen on.
    pub listen_port: u16,
    /// Overrides the default bind address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<String>,
    /// The host the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_port: Option<u16>,
    /// Free-form labels.
    #[serde(default)]
    pub tags: Vec<String>,
    /// How the tunnel is opened; absent when nothing is set.
    #[serde(default, skip_serializing_if = "OpenOptionsDto::is_empty")]
    pub open: OpenOptionsDto,
    /// Overrides the standby setting for this tunnel; absent inherits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standby: Option<bool>,
}

/// The definition of a temporary tunnel: a [`TunnelSpecDto`] without tags,
/// since a temporary tunnel is never saved and so is never selected by tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemporaryTunnelSpecDto {
    /// The `ssh_config` alias the tunnel goes through.
    pub host: String,
    /// The direction of the forward.
    pub kind: TunnelKindDto,
    /// The port to listen on.
    pub listen_port: u16,
    /// Overrides the default bind address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<String>,
    /// The host the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_port: Option<u16>,
}

/// A configured tunnel: its identifier and its [`TunnelSpecDto`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelDefinitionDto {
    /// The stable identifier.
    pub id: String,
    /// The definition, flattened into the same JSON object.
    #[serde(flatten)]
    pub spec: TunnelSpecDto,
}

/// Why a tunnel failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FailureReasonDto {
    /// The listen port is taken.
    PortInUse {
        /// The address that could not be bound.
        address: String,
    },
    /// Listening failed for another reason.
    ListenFailed {
        /// The address that could not be bound.
        address: String,
        /// The system's explanation.
        message: String,
    },
    /// The SSH connection could not be established.
    Connect {
        /// The explanation.
        message: String,
    },
    /// The SSH agent did not answer, typically while waiting for the user to
    /// confirm a signature.
    AgentUnresponsive {
        /// How long the agent was given, in milliseconds.
        timeout_ms: u64,
    },
    /// The SSH server refused to listen for a remote forward.
    RemoteForwardRefused {
        /// The address the server was asked to listen on.
        address: String,
    },
}

/// The process holding a port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortOwnerDto {
    /// The process identifier.
    pub pid: u32,
    /// The process name, such as `ssh`.
    pub process: String,
}

/// Where a tunnel is in its life.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TunnelStateDto {
    /// Not running.
    Stopped,
    /// Holding the listening port without an SSH session; the first
    /// connection opens one.
    Standby {
        /// Why the last wake-up failed; absent when it did not.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_failure: Option<FailureReasonDto>,
    },
    /// Connecting.
    Starting,
    /// Forwarding.
    Active,
    /// The session was lost; trying again.
    Reconnecting {
        /// The number of the upcoming attempt, starting at 1.
        attempt: u32,
        /// When the next attempt is due.
        next_retry_at: EpochMillis,
    },
    /// Gave up.
    Failed {
        /// Why.
        reason: FailureReasonDto,
    },
    /// The port is held by an ssh process whose command line has a matching
    /// forward; the tunnel is considered provided.
    External {
        /// The ssh process providing the tunnel.
        owner: PortOwnerDto,
    },
    /// The port is held by a process that does not provide the tunnel.
    Conflict {
        /// The process holding the port; absent when it cannot be determined,
        /// such as another user's socket on Linux.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner: Option<PortOwnerDto>,
    },
}

/// Whether the target of a tunnel answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TargetHealthDto {
    /// Never checked.
    Unknown,
    /// The last attempt reached it.
    Reachable,
    /// Attempts fail.
    Unreachable {
        /// When the first failure of the current streak happened.
        since: EpochMillis,
        /// The last error.
        error: String,
    },
}

/// What a tunnel carried since it was started.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsDto {
    /// The connections being forwarded right now.
    pub active_connections: u64,
    /// The connections accepted since the tunnel started.
    pub total_connections: u64,
    /// The bytes received from the target.
    pub bytes_in: u64,
    /// The bytes sent to the target.
    pub bytes_out: u64,
    /// When bytes last moved or a connection was accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<EpochMillis>,
}

/// A tunnel with its live state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelDto {
    /// The definition, flattened into the same JSON object.
    #[serde(flatten)]
    pub definition: TunnelDefinitionDto,
    /// Where the tunnel is in its life.
    pub state: TunnelStateDto,
    /// Whether its target answers.
    pub health: TargetHealthDto,
    /// What it carried.
    pub metrics: MetricsDto,
    /// Whether the tunnel was woken from standby and is serving: it can be
    /// sent back to standby.
    #[serde(default)]
    pub is_awake: bool,
    /// Whether the tunnel lives only in the daemon: it is never saved, and
    /// stopping it removes it.
    #[serde(default)]
    pub is_temporary: bool,
    /// What the tunnel opens with when nothing is asked: the choice saved in
    /// its `open.with`, else the opener suggested for its target port, else
    /// the browser.
    pub default_opener: DefaultOpenerDto,
}

/// How serious a log entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevelDto {
    /// Something expected happened.
    Info,
    /// Something went wrong but the tunnel carries on.
    Warn,
    /// Something went wrong and the tunnel stopped working.
    Error,
}

/// One line of a tunnel's log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntryDto {
    /// When it happened.
    pub at: EpochMillis,
    /// How serious it is.
    pub level: LogLevelDto,
    /// What happened.
    pub message: String,
}

/// The application settings. Durations are in milliseconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is a setting of the wire contract"
)]
pub struct SettingsDto {
    /// Bind address of local and dynamic tunnels without an override.
    pub default_bind_address: String,
    /// Bind address of remote tunnels without an override.
    pub default_remote_bind_address: String,
    /// Delay between keepalive probes; zero disables keepalive.
    pub keepalive_interval_ms: u64,
    /// Unanswered keepalive probes tolerated before the session is lost.
    pub keepalive_max: u32,
    /// How long an unused session stays open.
    pub session_idle_grace_ms: u64,
    /// Delay before the first reconnection attempt.
    pub reconnect_initial_delay_ms: u64,
    /// Upper bound of the reconnection backoff.
    pub reconnect_max_delay_ms: u64,
    /// Whether tunnels that were active at the last exit are restarted.
    pub restore_on_launch: bool,
    /// Tags whose tunnels start when the daemon launches.
    pub start_on_launch_tags: Vec<String>,
    /// Whether the daemon starts at login.
    pub launch_at_login: bool,
    /// Whether remembered passphrases are kept in the OS keyring.
    pub use_os_keyring: bool,
    /// The local X11 display; absent means `$DISPLAY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x11_display: Option<String>,
    /// Which loopback name opened tunnels use unless they say otherwise.
    #[serde(default)]
    pub open_host: OpenHostDto,
    /// Whether local and dynamic tunnels without an override wait in standby.
    #[serde(default)]
    pub standby: bool,
    /// Seconds without a connection before a standby tunnel releases its
    /// session.
    #[serde(default = "default_standby_idle_s")]
    pub standby_idle_s: u64,
    /// How a wake-up from standby, or its failure, is signalled.
    #[serde(default)]
    pub standby_notify: StandbyNotifyDto,
    /// The sound played when a wake-up, or its failure, is signalled by sound.
    #[serde(default)]
    pub standby_sound: StandbySoundDto,
    /// The language of the UI, the tray and the notifications.
    #[serde(default)]
    pub language: LanguageDto,
    /// Whether the UI looks for a newer release when it opens.
    #[serde(default = "default_check_for_updates")]
    pub check_for_updates: bool,
}

fn default_check_for_updates() -> bool {
    true
}

fn default_standby_idle_s() -> u64 {
    DEFAULT_STANDBY_IDLE_S
}

/// Whether the daemon shows its notification-area (tray) icon.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TrayStatusDto {
    /// The icon is shown.
    Active,
    /// The icon exists but no notification area hosts it (Linux: no
    /// `org.kde.StatusNotifierWatcher` on the session bus, e.g. GNOME without
    /// the `AppIndicator` extension).
    MissingHost,
    /// The icon could not be created.
    Failed {
        /// The explanation.
        message: String,
    },
    /// No icon on purpose: disabled by the user or unsupported on this platform.
    #[default]
    Disabled,
}

/// A port forward found on the command line of an ssh process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalForwardDto {
    /// The direction of the forward.
    pub kind: TunnelKindDto,
    /// The bind address, when the command line gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<String>,
    /// The port listened on.
    pub listen_port: u16,
    /// The host the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_port: Option<u16>,
}

/// An ssh process with forwards that match no configured tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalProcessDto {
    /// The process identifier.
    pub pid: u32,
    /// The process name, such as `ssh`.
    pub process: String,
    /// The host the process connects to, when the command line gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    /// The forwards found on its command line.
    pub forwards: Vec<ExternalForwardDto>,
}

/// Where the X11 anchor of a host is in its life.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum X11StateDto {
    /// Not running.
    Stopped,
    /// Connecting.
    Starting,
    /// Forwarding X11.
    Active {
        /// The remote `DISPLAY` other remote sessions can use.
        display: String,
    },
    /// The session was lost; trying again.
    Reconnecting {
        /// The number of the upcoming attempt, starting at 1.
        attempt: u32,
        /// When the next attempt is due.
        next_retry_at: EpochMillis,
    },
    /// Gave up.
    Failed {
        /// Why.
        message: String,
    },
}

/// The X11 anchor of a host alias: a session with X11 forwarding whose remote
/// `DISPLAY` other remote sessions can use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct X11AnchorDto {
    /// The `ssh_config` alias.
    pub host: String,
    /// Where the anchor is in its life.
    pub state: X11StateDto,
}

/// A saved command run with X11 on a host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct X11LauncherDto {
    /// The `ssh_config` alias the command runs on.
    pub host: String,
    /// Unique, human-readable name.
    pub name: String,
    /// The command line.
    pub command: String,
}

/// Everything a client needs to draw its first screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDto {
    /// Every tunnel with its live state.
    pub tunnels: Vec<TunnelDto>,
    /// The settings.
    pub settings: SettingsDto,
    /// Whether the daemon shows its tray icon.
    #[serde(default)]
    pub tray: TrayStatusDto,
    /// The ssh processes with forwards that match no configured tunnel.
    #[serde(default)]
    pub externals: Vec<ExternalProcessDto>,
    /// The X11 anchors that are not stopped.
    #[serde(default)]
    pub x11_anchors: Vec<X11AnchorDto>,
    /// The saved X11 launchers.
    #[serde(default)]
    pub x11_launchers: Vec<X11LauncherDto>,
    /// The local X display that a host alias overrides in `tunnels.toml`, by
    /// alias. Hosts without an override are absent.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub x11_host_displays: BTreeMap<String, String>,
    /// The openers that can be used: the ones of the user, then the detected
    /// presets they do not replace.
    #[serde(default)]
    pub openers: Vec<AvailableOpenerDto>,
}

/// One question of a keyboard-interactive exchange.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDto {
    /// The question to show.
    pub text: String,
    /// Whether the typed answer may be displayed.
    pub echo: bool,
}

/// Which secret the daemon asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CredentialRequestDto {
    /// The passphrase of an encrypted identity file.
    Passphrase {
        /// The host alias being connected to.
        alias: String,
        /// The encrypted identity file.
        key_path: String,
    },
    /// The account password.
    Password {
        /// The host alias being connected to.
        alias: String,
        /// The remote login name.
        user: String,
        /// The host name being connected to.
        host: String,
    },
    /// Answers to a keyboard-interactive round.
    KeyboardInteractive {
        /// The host alias being connected to.
        alias: String,
        /// The title sent by the server; may be empty.
        name: String,
        /// The instructions sent by the server; may be empty.
        instructions: String,
        /// The questions, answered in order.
        prompts: Vec<PromptDto>,
    },
}

/// What to do with an unknown host key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostKeyDecisionDto {
    /// Trust the key and record it in `known_hosts`.
    AcceptAndSave,
    /// Trust the key until the daemon exits.
    AcceptOnce,
    /// Refuse the key.
    Reject,
}

/// How long the daemon keeps a passphrase the user just typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RememberDto {
    /// Keep it in memory until the daemon stops.
    Session,
    /// Keep it in memory and in the OS keyring, when the keyring is enabled.
    Keyring,
}

/// Why a host key was refused without asking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum HostKeyAlertReasonDto {
    /// The host has a different key on record.
    Changed {
        /// The SHA-256 fingerprint on record.
        known_fingerprint: String,
        /// The `known_hosts` file holding it.
        path: String,
        /// The 1-based line of the recorded key.
        line: u64,
    },
    /// A `@revoked` line matches the key.
    Revoked {
        /// The `known_hosts` file holding the line.
        path: String,
        /// The 1-based line.
        line: u64,
    },
    /// The host is unknown and the configuration forbids asking.
    UnknownRejectedByPolicy,
}

/// One option of a resolved host, in `ssh -G` spelling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostOptionDto {
    /// The lowercase keyword.
    pub key: String,
    /// The value, as `ssh -G` prints it.
    pub value: String,
}

/// The effective configuration of a host alias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedHostDto {
    /// The alias.
    pub host: String,
    /// The options `ssh -G <host>` would report, in its order.
    pub options: Vec<HostOptionDto>,
    /// What deserved a warning while resolving, in English.
    pub warnings: Vec<String>,
}

/// One forward found in an import source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidateDto {
    /// The `ssh_config` alias the forward belongs to, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// The direction of the forward.
    pub kind: TunnelKindDto,
    /// The port to listen on.
    pub listen_port: u16,
    /// The bind address, when one was written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<String>,
    /// The host the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_port: Option<u16>,
    /// The comment written next to the port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// The 1-based line of the pasted text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// Something skipped while reading an import source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ImportWarningDto {
    /// A forward specification that cannot be parsed, or a template without a port list.
    MalformedForward {
        /// The 1-based line of the specification.
        line: u32,
        /// The specification as written.
        text: String,
    },
    /// A Unix socket forward or a remote SOCKS proxy.
    UnsupportedForward {
        /// The 1-based line of the specification.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        line: Option<u32>,
        /// The `ssh_config` alias declaring the forward.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        host: Option<String>,
        /// The specification as written.
        text: String,
    },
    /// An `ssh` destination that matches no `ssh_config` alias.
    UnknownDestination {
        /// The destination as written.
        destination: String,
    },
    /// An alias whose configuration could not be resolved.
    HostResolutionFailed {
        /// The alias.
        host: String,
        /// Why the resolution failed.
        message: String,
    },
}

/// The forwards found in an import source, and what was skipped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreviewDto {
    /// The forwards found, in source order.
    pub candidates: Vec<ImportCandidateDto>,
    /// What was skipped or could not be resolved.
    pub warnings: Vec<ImportWarningDto>,
}
