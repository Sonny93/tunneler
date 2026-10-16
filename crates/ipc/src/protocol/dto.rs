use serde::{Deserialize, Serialize};

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

/// The editable part of a tunnel definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelSpecDto {
    /// Unique, human-readable name.
    pub name: String,
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
    /// This kind of tunnel is not implemented.
    UnsupportedKind {
        /// The unsupported kind.
        tunnel_kind: TunnelKindDto,
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
    /// Whether the target of a tunnel is probed periodically.
    pub health_probe: bool,
    /// Whether tunnels that were active at the last exit are restarted.
    pub restore_on_launch: bool,
    /// Tags whose tunnels start when the daemon launches.
    pub start_on_launch_tags: Vec<String>,
    /// Whether the daemon starts at login.
    pub launch_at_login: bool,
    /// The local X11 display; absent means `$DISPLAY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x11_display: Option<String>,
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
