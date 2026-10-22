use serde::{Deserialize, Serialize};

use crate::protocol::dto::{
    CredentialRequestDto, ExternalProcessDto, HostKeyAlertReasonDto, LogEntryDto, MetricsDto,
    TargetHealthDto, TrayStatusDto, TunnelStateDto, X11StateDto,
};

/// The state or the target health of a tunnel changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelChangedEvent {
    /// The tunnel.
    pub id: String,
    /// Its new state.
    pub state: TunnelStateDto,
    /// Its new target health.
    pub health: TargetHealthDto,
    /// Whether it is now woken from standby and serving.
    #[serde(default)]
    pub is_awake: bool,
}

/// The metrics of a tunnel changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelMetricsEvent {
    /// The tunnel.
    pub id: String,
    /// Its current metrics.
    pub metrics: MetricsDto,
}

/// A tunnel logged something.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelLogEvent {
    /// The tunnel.
    pub id: String,
    /// The log entry.
    pub entry: LogEntryDto,
}

/// The configuration file could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigInvalidEvent {
    /// What is wrong with the file; the previous configuration stays active.
    pub messages: Vec<String>,
}

/// The daemon needs a secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRequestEvent {
    /// Correlates the answer with this request.
    pub prompt_id: String,
    /// Which secret is needed, flattened into the same JSON object.
    #[serde(flatten)]
    pub request: CredentialRequestDto,
}

/// The daemon asks about, or reports, a host key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum HostKeyRequestEvent {
    /// An unknown key awaits a decision.
    Prompt {
        /// Correlates the answer with this request.
        prompt_id: String,
        /// The host alias being connected to.
        alias: String,
        /// The name the key would be recorded under.
        lookup_name: String,
        /// The key algorithm, such as `ssh-ed25519`.
        algorithm: String,
        /// The OpenSSH SHA-256 fingerprint.
        fingerprint_sha256: String,
    },
    /// A key was refused; no answer is expected.
    Alert {
        /// The host alias being connected to.
        alias: String,
        /// Why the key was refused.
        reason: HostKeyAlertReasonDto,
        /// The SHA-256 fingerprint the server presented.
        presented_fingerprint: String,
    },
}

/// A prompt was answered, possibly by another client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptResolvedEvent {
    /// The prompt that is no longer pending.
    pub prompt_id: String,
}

/// The tray icon status changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayChangedEvent {
    /// The new status.
    pub status: TrayStatusDto,
}

/// The ssh processes with unmatched forwards changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalsChangedEvent {
    /// Every such process now, replacing the previous list.
    pub processes: Vec<ExternalProcessDto>,
}

/// The state of the X11 anchor of a host changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct X11ChangedEvent {
    /// The host alias.
    pub host: String,
    /// The new state of its anchor.
    pub state: X11StateDto,
}

/// What the daemon pushes: `{"event": "...", "data": {...}}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum EventMessage {
    /// The state or health of a tunnel changed.
    #[serde(rename = "tunnel.changed")]
    TunnelChanged(TunnelChangedEvent),
    /// The metrics of a tunnel changed.
    #[serde(rename = "tunnel.metrics")]
    TunnelMetrics(TunnelMetricsEvent),
    /// A tunnel logged something.
    #[serde(rename = "tunnel.log")]
    TunnelLog(TunnelLogEvent),
    /// The definitions or the settings changed; read a new snapshot.
    #[serde(rename = "config.reloaded")]
    ConfigReloaded {},
    /// The configuration file is invalid.
    #[serde(rename = "config.invalid")]
    ConfigInvalid(ConfigInvalidEvent),
    /// The daemon needs a secret.
    #[serde(rename = "credential.request")]
    CredentialRequest(CredentialRequestEvent),
    /// The daemon asks about, or reports, a host key.
    #[serde(rename = "hostkey.request")]
    HostKeyRequest(HostKeyRequestEvent),
    /// Somebody asked the daemon to quit; the UI should confirm.
    #[serde(rename = "quit.requested")]
    QuitRequested {},
    /// Somebody asked to send the awake tunnels back to standby; the UI
    /// should confirm.
    #[serde(rename = "standby.requested")]
    StandbyRequested {},
    /// The tray icon status changed.
    #[serde(rename = "tray.changed")]
    TrayChanged(TrayChangedEvent),
    /// Somebody asked to see the UI; a connected UI should bring its window to
    /// the front.
    #[serde(rename = "show.requested")]
    ShowRequested {},
    /// A prompt was answered elsewhere; close its dialog.
    #[serde(rename = "prompt.resolved")]
    PromptResolved(PromptResolvedEvent),
    /// The ssh processes with unmatched forwards changed.
    #[serde(rename = "externals.changed")]
    ExternalsChanged(ExternalsChangedEvent),
    /// The state of the X11 anchor of a host changed.
    #[serde(rename = "x11.changed")]
    X11Changed(X11ChangedEvent),
}
