//! Conversions between the engine types and the wire types of the IPC protocol.
//!
//! The wire types are the daemon's public contract, so none of the engine types
//! leak through: every conversion is explicit and covered by a test.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use engine::{
    AvailableOpener, DefaultOpener, EngineEvent, EngineSnapshot, FailureReason, HostKeyDecision,
    ImportCandidate, ImportPreview, ImportWarning, InvalidTunnelId, LogLevel, NewTunnel, OpenHost,
    OpenOptions, OpenScheme, Opener, OpenerSource, PortOwner, RejectedHostKey, RejectionReason,
    Retention, Settings, SshForward, StandbyNotify, StandbySound, TargetHealth, TunnelDefinition,
    TunnelId, TunnelKind, TunnelLogEntry, TunnelMetrics, TunnelSnapshot, TunnelState,
    UnmanagedProcess, X11AnchorSnapshot, X11AnchorState, X11Launcher, default_opener,
};
use ipc::protocol::{
    AvailableOpenerDto, ConfigInvalidEvent, DefaultOpenerDto, EpochMillis, EventMessage,
    ExternalForwardDto, ExternalProcessDto, ExternalsChangedEvent, FailureReasonDto,
    HostKeyAlertReasonDto, HostKeyDecisionDto, HostKeyRequestEvent, HostOptionDto,
    ImportCandidateDto, ImportPreviewDto, ImportWarningDto, LogEntryDto, LogLevelDto, MetricsDto,
    OpenHostDto, OpenOptionsDto, OpenSchemeDto, OpenerSourceDto, OpenerSpecDto, PortOwnerDto,
    RememberDto, ResolvedHostDto, SettingsDto, SnapshotDto, SoundEventDto, StandbyNotifyDto,
    StandbySoundDto, TargetHealthDto, TemporaryTunnelSpecDto, TrayStatusDto, TunnelChangedEvent,
    TunnelDefinitionDto, TunnelDto, TunnelKindDto, TunnelLogEvent, TunnelMetricsEvent,
    TunnelSpecDto, TunnelStateDto, X11AnchorDto, X11ChangedEvent, X11LauncherDto, X11StateDto,
};

use ssh_config::Resolution;

use super::standby_signal::SoundEvent;
use super::tray::TrayStatus;

pub(crate) fn to_epoch_millis(time: SystemTime) -> EpochMillis {
    let elapsed = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    saturating_u64(elapsed.as_millis())
}

fn saturating_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn saturating_usize(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

pub(crate) fn kind_to_dto(kind: TunnelKind) -> TunnelKindDto {
    match kind {
        TunnelKind::Local => TunnelKindDto::Local,
        TunnelKind::Remote => TunnelKindDto::Remote,
        TunnelKind::Dynamic => TunnelKindDto::Dynamic,
    }
}

pub(crate) fn kind_from_dto(kind: TunnelKindDto) -> TunnelKind {
    match kind {
        TunnelKindDto::Local => TunnelKind::Local,
        TunnelKindDto::Remote => TunnelKind::Remote,
        TunnelKindDto::Dynamic => TunnelKind::Dynamic,
    }
}

pub(crate) fn definition_to_dto(definition: &TunnelDefinition) -> TunnelDefinitionDto {
    TunnelDefinitionDto {
        id: definition.id.to_string(),
        spec: TunnelSpecDto {
            host: definition.host.clone(),
            kind: kind_to_dto(definition.kind),
            listen_port: definition.listen_port,
            listen_address: definition.listen_address.clone(),
            target_host: definition.target_host.clone(),
            target_port: definition.target_port,
            tags: definition.tags.clone(),
            open: open_options_to_dto(&definition.open),
            standby: definition.standby,
        },
    }
}

fn open_options_to_dto(options: &OpenOptions) -> OpenOptionsDto {
    OpenOptionsDto {
        scheme: options.scheme.map(open_scheme_to_dto),
        path: options.path.clone(),
        host: options.host.map(open_host_to_dto),
        with: options.with.clone(),
    }
}

fn open_options_from_dto(dto: OpenOptionsDto) -> OpenOptions {
    OpenOptions {
        scheme: dto.scheme.map(open_scheme_from_dto),
        path: dto.path,
        host: dto.host.map(open_host_from_dto),
        with: dto.with,
    }
}

fn open_scheme_to_dto(scheme: OpenScheme) -> OpenSchemeDto {
    match scheme {
        OpenScheme::Http => OpenSchemeDto::Http,
        OpenScheme::Https => OpenSchemeDto::Https,
    }
}

fn open_scheme_from_dto(scheme: OpenSchemeDto) -> OpenScheme {
    match scheme {
        OpenSchemeDto::Http => OpenScheme::Http,
        OpenSchemeDto::Https => OpenScheme::Https,
    }
}

fn standby_notify_to_dto(mode: StandbyNotify) -> StandbyNotifyDto {
    match mode {
        StandbyNotify::None => StandbyNotifyDto::None,
        StandbyNotify::Visual => StandbyNotifyDto::Visual,
        StandbyNotify::Sound => StandbyNotifyDto::Sound,
        StandbyNotify::Both => StandbyNotifyDto::Both,
    }
}

fn standby_notify_from_dto(mode: StandbyNotifyDto) -> StandbyNotify {
    match mode {
        StandbyNotifyDto::None => StandbyNotify::None,
        StandbyNotifyDto::Visual => StandbyNotify::Visual,
        StandbyNotifyDto::Sound => StandbyNotify::Sound,
        StandbyNotifyDto::Both => StandbyNotify::Both,
    }
}

fn standby_sound_to_dto(sound: StandbySound) -> StandbySoundDto {
    match sound {
        StandbySound::System => StandbySoundDto::System,
        StandbySound::Chime => StandbySoundDto::Chime,
        StandbySound::Marimba => StandbySoundDto::Marimba,
        StandbySound::Pluck => StandbySoundDto::Pluck,
    }
}

pub(crate) fn standby_sound_from_dto(sound: StandbySoundDto) -> StandbySound {
    match sound {
        StandbySoundDto::System => StandbySound::System,
        StandbySoundDto::Chime => StandbySound::Chime,
        StandbySoundDto::Marimba => StandbySound::Marimba,
        StandbySoundDto::Pluck => StandbySound::Pluck,
    }
}

pub(crate) fn sound_event_from_dto(event: SoundEventDto) -> SoundEvent {
    match event {
        SoundEventDto::Woke => SoundEvent::Woke,
        SoundEventDto::WakeFailed => SoundEvent::WakeFailed,
    }
}

fn open_host_to_dto(host: OpenHost) -> OpenHostDto {
    match host {
        OpenHost::Localhost => OpenHostDto::Localhost,
        OpenHost::Ip => OpenHostDto::Ip,
    }
}

fn open_host_from_dto(host: OpenHostDto) -> OpenHost {
    match host {
        OpenHostDto::Localhost => OpenHost::Localhost,
        OpenHostDto::Ip => OpenHost::Ip,
    }
}

pub(crate) fn definition_from_dto(
    dto: TunnelDefinitionDto,
) -> Result<TunnelDefinition, InvalidTunnelId> {
    let id: TunnelId = dto.id.parse()?;
    let spec = dto.spec;
    Ok(TunnelDefinition {
        id,
        host: spec.host,
        kind: kind_from_dto(spec.kind),
        listen_port: spec.listen_port,
        listen_address: spec.listen_address,
        target_host: spec.target_host,
        target_port: spec.target_port,
        tags: spec.tags,
        open: open_options_from_dto(spec.open),
        standby: spec.standby,
    })
}

pub(crate) fn new_tunnel_from_spec(spec: TunnelSpecDto) -> NewTunnel {
    NewTunnel {
        host: spec.host,
        kind: kind_from_dto(spec.kind),
        listen_port: spec.listen_port,
        listen_address: spec.listen_address,
        target_host: spec.target_host,
        target_port: spec.target_port,
        tags: spec.tags,
        open: open_options_from_dto(spec.open),
        standby: spec.standby,
    }
}

pub(crate) fn new_temporary_tunnel_from_spec(spec: TemporaryTunnelSpecDto) -> NewTunnel {
    NewTunnel {
        host: spec.host,
        kind: kind_from_dto(spec.kind),
        listen_port: spec.listen_port,
        listen_address: spec.listen_address,
        target_host: spec.target_host,
        target_port: spec.target_port,
        tags: Vec::new(),
        open: OpenOptions::default(),
        standby: None,
    }
}

fn failure_to_dto(reason: &FailureReason) -> FailureReasonDto {
    match reason {
        FailureReason::PortInUse { address } => FailureReasonDto::PortInUse {
            address: address.clone(),
        },
        FailureReason::ListenFailed { address, message } => FailureReasonDto::ListenFailed {
            address: address.clone(),
            message: message.clone(),
        },
        FailureReason::Connect { message } => FailureReasonDto::Connect {
            message: message.clone(),
        },
        FailureReason::AgentUnresponsive { timeout } => FailureReasonDto::AgentUnresponsive {
            timeout_ms: saturating_u64(timeout.as_millis()),
        },
        FailureReason::RemoteForwardRefused { address } => FailureReasonDto::RemoteForwardRefused {
            address: address.clone(),
        },
    }
}

pub(crate) fn state_to_dto(state: &TunnelState) -> TunnelStateDto {
    match state {
        TunnelState::Stopped => TunnelStateDto::Stopped,
        TunnelState::Standby { last_failure } => TunnelStateDto::Standby {
            last_failure: last_failure.as_ref().map(failure_to_dto),
        },
        TunnelState::Starting => TunnelStateDto::Starting,
        TunnelState::Active => TunnelStateDto::Active,
        TunnelState::Reconnecting {
            attempt,
            next_retry_at,
        } => TunnelStateDto::Reconnecting {
            attempt: *attempt,
            next_retry_at: to_epoch_millis(*next_retry_at),
        },
        TunnelState::Failed { reason } => TunnelStateDto::Failed {
            reason: failure_to_dto(reason),
        },
        TunnelState::External { owner } => TunnelStateDto::External {
            owner: owner_to_dto(owner),
        },
        TunnelState::Conflict { owner } => TunnelStateDto::Conflict {
            owner: owner.as_ref().map(owner_to_dto),
        },
    }
}

fn owner_to_dto(owner: &PortOwner) -> PortOwnerDto {
    PortOwnerDto {
        pid: owner.pid,
        process: owner.process.clone(),
    }
}

fn forward_to_dto(forward: &SshForward) -> ExternalForwardDto {
    ExternalForwardDto {
        kind: kind_to_dto(forward.kind),
        listen_address: forward.listen_address.clone(),
        listen_port: forward.listen_port,
        target_host: forward.target_host.clone(),
        target_port: forward.target_port,
    }
}

pub(crate) fn external_to_dto(process: &UnmanagedProcess) -> ExternalProcessDto {
    ExternalProcessDto {
        pid: process.pid,
        process: process.process.clone(),
        destination: process.destination.clone(),
        forwards: process.forwards.iter().map(forward_to_dto).collect(),
    }
}

pub(crate) fn health_to_dto(health: &TargetHealth) -> TargetHealthDto {
    match health {
        TargetHealth::Unknown => TargetHealthDto::Unknown,
        TargetHealth::Reachable => TargetHealthDto::Reachable,
        TargetHealth::Unreachable { since, error } => TargetHealthDto::Unreachable {
            since: to_epoch_millis(*since),
            error: error.clone(),
        },
    }
}

pub(crate) fn metrics_to_dto(metrics: &TunnelMetrics) -> MetricsDto {
    MetricsDto {
        active_connections: saturating_usize(metrics.active_connections),
        total_connections: metrics.total_connections,
        bytes_in: metrics.bytes_in,
        bytes_out: metrics.bytes_out,
        last_activity: metrics.last_activity.map(to_epoch_millis),
    }
}

pub(crate) fn opener_to_spec_dto(opener: &Opener) -> OpenerSpecDto {
    OpenerSpecDto {
        id: opener.id.clone(),
        name: opener.name.clone(),
        command: opener.command.clone(),
        args: opener.args.clone(),
        ports: opener.ports.clone(),
    }
}

pub(crate) fn opener_from_spec_dto(dto: OpenerSpecDto) -> Opener {
    Opener {
        id: dto.id,
        name: dto.name,
        command: dto.command,
        args: dto.args,
        ports: dto.ports,
    }
}

pub(crate) fn available_opener_to_dto(available: &AvailableOpener) -> AvailableOpenerDto {
    AvailableOpenerDto {
        spec: opener_to_spec_dto(&available.opener),
        source: match available.source {
            OpenerSource::Detected => OpenerSourceDto::Detected,
            OpenerSource::User => OpenerSourceDto::User,
        },
    }
}

fn default_opener_to_dto(default: DefaultOpener) -> DefaultOpenerDto {
    DefaultOpenerDto {
        id: default.id,
        name: default.name,
    }
}

pub(crate) fn tunnel_to_dto(snapshot: &TunnelSnapshot, openers: &[AvailableOpener]) -> TunnelDto {
    TunnelDto {
        definition: definition_to_dto(&snapshot.definition),
        state: state_to_dto(&snapshot.state),
        health: health_to_dto(&snapshot.health),
        metrics: metrics_to_dto(&snapshot.metrics),
        is_temporary: snapshot.is_temporary,
        default_opener: default_opener_to_dto(default_opener(&snapshot.definition, openers)),
    }
}

pub(crate) fn settings_to_dto(settings: &Settings) -> SettingsDto {
    SettingsDto {
        default_bind_address: settings.default_bind_address.clone(),
        default_remote_bind_address: settings.default_remote_bind_address.clone(),
        keepalive_interval_ms: saturating_u64(settings.keepalive_interval.as_millis()),
        keepalive_max: settings.keepalive_max,
        session_idle_grace_ms: saturating_u64(settings.session_idle_grace.as_millis()),
        reconnect_initial_delay_ms: saturating_u64(settings.reconnect_initial_delay.as_millis()),
        reconnect_max_delay_ms: saturating_u64(settings.reconnect_max_delay.as_millis()),
        restore_on_launch: settings.restore_on_launch,
        start_on_launch_tags: settings.start_on_launch_tags.clone(),
        launch_at_login: settings.launch_at_login,
        use_os_keyring: settings.use_os_keyring,
        x11_display: settings.x11_display.clone(),
        open_host: open_host_to_dto(settings.open_host),
        standby: settings.standby,
        standby_idle_s: settings.standby_idle.as_secs(),
        standby_notify: standby_notify_to_dto(settings.standby_notify),
        standby_sound: standby_sound_to_dto(settings.standby_sound),
        check_for_updates: settings.check_for_updates,
    }
}

pub(crate) fn settings_from_dto(dto: SettingsDto) -> Settings {
    Settings {
        default_bind_address: dto.default_bind_address,
        default_remote_bind_address: dto.default_remote_bind_address,
        keepalive_interval: Duration::from_millis(dto.keepalive_interval_ms),
        keepalive_max: dto.keepalive_max,
        session_idle_grace: Duration::from_millis(dto.session_idle_grace_ms),
        reconnect_initial_delay: Duration::from_millis(dto.reconnect_initial_delay_ms),
        reconnect_max_delay: Duration::from_millis(dto.reconnect_max_delay_ms),
        restore_on_launch: dto.restore_on_launch,
        start_on_launch_tags: dto.start_on_launch_tags,
        launch_at_login: dto.launch_at_login,
        use_os_keyring: dto.use_os_keyring,
        x11_display: dto.x11_display,
        open_host: open_host_from_dto(dto.open_host),
        standby: dto.standby,
        standby_idle: Duration::from_secs(dto.standby_idle_s),
        standby_notify: standby_notify_from_dto(dto.standby_notify),
        standby_sound: standby_sound_from_dto(dto.standby_sound),
        check_for_updates: dto.check_for_updates,
    }
}

pub(crate) fn tray_status_to_dto(status: &TrayStatus) -> TrayStatusDto {
    match status {
        TrayStatus::Active => TrayStatusDto::Active,
        TrayStatus::MissingHost => TrayStatusDto::MissingHost,
        TrayStatus::Failed { message } => TrayStatusDto::Failed {
            message: message.clone(),
        },
        TrayStatus::Disabled => TrayStatusDto::Disabled,
    }
}

pub(crate) fn snapshot_to_dto(snapshot: &EngineSnapshot, tray: &TrayStatus) -> SnapshotDto {
    SnapshotDto {
        tunnels: snapshot
            .tunnels
            .iter()
            .map(|tunnel| tunnel_to_dto(tunnel, &snapshot.openers))
            .collect(),
        settings: settings_to_dto(&snapshot.settings),
        tray: tray_status_to_dto(tray),
        externals: snapshot.externals.iter().map(external_to_dto).collect(),
        x11_anchors: snapshot.x11_anchors.iter().map(x11_anchor_to_dto).collect(),
        x11_launchers: snapshot
            .x11_launchers
            .iter()
            .map(x11_launcher_to_dto)
            .collect(),
        openers: snapshot
            .openers
            .iter()
            .map(available_opener_to_dto)
            .collect(),
    }
}

pub(crate) fn x11_state_to_dto(state: &X11AnchorState) -> X11StateDto {
    match state {
        X11AnchorState::Stopped => X11StateDto::Stopped,
        X11AnchorState::Starting => X11StateDto::Starting,
        X11AnchorState::Active { display } => X11StateDto::Active {
            display: display.clone(),
        },
        X11AnchorState::Reconnecting {
            attempt,
            next_retry_at,
        } => X11StateDto::Reconnecting {
            attempt: *attempt,
            next_retry_at: to_epoch_millis(*next_retry_at),
        },
        X11AnchorState::Failed { message } => X11StateDto::Failed {
            message: message.clone(),
        },
    }
}

fn x11_anchor_to_dto(anchor: &X11AnchorSnapshot) -> X11AnchorDto {
    X11AnchorDto {
        host: anchor.host.clone(),
        state: x11_state_to_dto(&anchor.state),
    }
}

fn x11_launcher_to_dto(launcher: &X11Launcher) -> X11LauncherDto {
    X11LauncherDto {
        host: launcher.host.clone(),
        name: launcher.name.clone(),
        command: launcher.command.clone(),
    }
}

fn log_level_to_dto(level: LogLevel) -> LogLevelDto {
    match level {
        LogLevel::Info => LogLevelDto::Info,
        LogLevel::Warn => LogLevelDto::Warn,
        LogLevel::Error => LogLevelDto::Error,
    }
}

pub(crate) fn log_entry_to_dto(entry: &TunnelLogEntry) -> LogEntryDto {
    LogEntryDto {
        at: to_epoch_millis(entry.at),
        level: log_level_to_dto(entry.level),
        message: entry.message.clone(),
    }
}

pub(crate) fn resolved_host_to_dto(alias: &str, resolution: &Resolution) -> ResolvedHostDto {
    ResolvedHostDto {
        host: alias.to_owned(),
        options: resolution
            .config
            .ssh_g_options()
            .into_iter()
            .map(|(key, value)| HostOptionDto { key, value })
            .collect(),
        warnings: resolution
            .warnings
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

pub(crate) fn import_preview_to_dto(preview: &ImportPreview) -> ImportPreviewDto {
    ImportPreviewDto {
        candidates: preview
            .candidates
            .iter()
            .map(import_candidate_to_dto)
            .collect(),
        warnings: preview.warnings.iter().map(import_warning_to_dto).collect(),
    }
}

fn import_candidate_to_dto(candidate: &ImportCandidate) -> ImportCandidateDto {
    ImportCandidateDto {
        host: candidate.host.clone(),
        kind: kind_to_dto(candidate.kind),
        listen_port: candidate.listen_port,
        listen_address: candidate.listen_address.clone(),
        target_host: candidate.target_host.clone(),
        target_port: candidate.target_port,
        comment: candidate.comment.clone(),
        line: candidate.line,
    }
}

fn import_warning_to_dto(warning: &ImportWarning) -> ImportWarningDto {
    match warning {
        ImportWarning::MalformedForward { line, text } => ImportWarningDto::MalformedForward {
            line: *line,
            text: text.clone(),
        },
        ImportWarning::UnsupportedForward { line, host, text } => {
            ImportWarningDto::UnsupportedForward {
                line: *line,
                host: host.clone(),
                text: text.clone(),
            }
        }
        ImportWarning::UnknownDestination { destination } => ImportWarningDto::UnknownDestination {
            destination: destination.clone(),
        },
        ImportWarning::HostResolutionFailed { host, message } => {
            ImportWarningDto::HostResolutionFailed {
                host: host.clone(),
                message: message.clone(),
            }
        }
    }
}

/// The wire form of an engine event; `None` for the events that stay in the
/// daemon.
pub(crate) fn event_to_message(event: &EngineEvent) -> Option<EventMessage> {
    Some(match event {
        EngineEvent::TunnelChanged { id, state, health } => {
            EventMessage::TunnelChanged(TunnelChangedEvent {
                id: id.to_string(),
                state: state_to_dto(state),
                health: health_to_dto(health),
            })
        }
        EngineEvent::TunnelMetrics { id, metrics } => {
            EventMessage::TunnelMetrics(TunnelMetricsEvent {
                id: id.to_string(),
                metrics: metrics_to_dto(metrics),
            })
        }
        EngineEvent::TunnelLog { id, entry } => EventMessage::TunnelLog(TunnelLogEvent {
            id: id.to_string(),
            entry: log_entry_to_dto(entry),
        }),
        EngineEvent::ConfigReloaded => EventMessage::ConfigReloaded {},
        EngineEvent::ConfigInvalid { message } => EventMessage::ConfigInvalid(ConfigInvalidEvent {
            messages: vec![message.clone()],
        }),
        EngineEvent::ExternalsChanged { processes } => {
            EventMessage::ExternalsChanged(ExternalsChangedEvent {
                processes: processes.iter().map(external_to_dto).collect(),
            })
        }
        EngineEvent::X11Changed { host, state } => EventMessage::X11Changed(X11ChangedEvent {
            host: host.clone(),
            state: x11_state_to_dto(state),
        }),
        EngineEvent::TunnelWoke { .. } | EngineEvent::TunnelWakeFailed { .. } => return None,
    })
}

pub(crate) fn host_key_decision_from_dto(decision: HostKeyDecisionDto) -> HostKeyDecision {
    match decision {
        HostKeyDecisionDto::AcceptAndSave => HostKeyDecision::AcceptAndSave,
        HostKeyDecisionDto::AcceptOnce => HostKeyDecision::AcceptOnce,
        HostKeyDecisionDto::Reject => HostKeyDecision::Reject,
    }
}

/// How long the engine keeps a passphrase, from what the client asked.
pub(crate) fn retention_from_dto(remember: Option<RememberDto>) -> Retention {
    match remember {
        None => Retention::Never,
        Some(RememberDto::Session) => Retention::Session,
        Some(RememberDto::Keyring) => Retention::Keyring,
    }
}

fn rejection_reason_to_dto(reason: &RejectionReason) -> HostKeyAlertReasonDto {
    match reason {
        RejectionReason::Changed {
            known_fingerprint,
            path,
            line,
        } => HostKeyAlertReasonDto::Changed {
            known_fingerprint: known_fingerprint.clone(),
            path: path.display().to_string(),
            line: saturating_usize(*line),
        },
        RejectionReason::Revoked { path, line } => HostKeyAlertReasonDto::Revoked {
            path: path.display().to_string(),
            line: saturating_usize(*line),
        },
        RejectionReason::UnknownRejectedByPolicy => HostKeyAlertReasonDto::UnknownRejectedByPolicy,
    }
}

pub(crate) fn rejection_to_alert(rejection: &RejectedHostKey) -> HostKeyRequestEvent {
    HostKeyRequestEvent::Alert {
        alias: rejection.alias.clone(),
        reason: rejection_reason_to_dto(&rejection.reason),
        presented_fingerprint: rejection.presented_fingerprint.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;

    use super::*;

    const SECONDS_AFTER_EPOCH: u64 = 1_700_000_000;

    fn candidate() -> ImportCandidate {
        ImportCandidate {
            host: Some("nova".to_owned()),
            kind: TunnelKind::Remote,
            listen_address: Some("0.0.0.0".to_owned()),
            listen_port: 9000,
            target_host: Some("localhost".to_owned()),
            target_port: Some(80),
            comment: Some("web".to_owned()),
            line: Some(3),
        }
    }

    #[test]
    fn should_map_every_field_of_an_import_candidate() {
        let preview = ImportPreview {
            candidates: vec![candidate()],
            warnings: Vec::new(),
        };

        assert_eq!(
            import_preview_to_dto(&preview).candidates,
            vec![ImportCandidateDto {
                host: Some("nova".to_owned()),
                kind: TunnelKindDto::Remote,
                listen_port: 9000,
                listen_address: Some("0.0.0.0".to_owned()),
                target_host: Some("localhost".to_owned()),
                target_port: Some(80),
                comment: Some("web".to_owned()),
                line: Some(3),
            }]
        );
    }

    #[test]
    fn should_map_every_import_warning_to_its_wire_warning() {
        let warnings = vec![
            ImportWarning::MalformedForward {
                line: 2,
                text: "x".to_owned(),
            },
            ImportWarning::UnsupportedForward {
                line: None,
                host: Some("sock".to_owned()),
                text: "8080 /run/app.sock".to_owned(),
            },
            ImportWarning::UnknownDestination {
                destination: "me@host".to_owned(),
            },
            ImportWarning::HostResolutionFailed {
                host: "broken".to_owned(),
                message: "bad port".to_owned(),
            },
        ];
        let preview = ImportPreview {
            candidates: Vec::new(),
            warnings,
        };

        assert_eq!(
            import_preview_to_dto(&preview).warnings,
            vec![
                ImportWarningDto::MalformedForward {
                    line: 2,
                    text: "x".to_owned()
                },
                ImportWarningDto::UnsupportedForward {
                    line: None,
                    host: Some("sock".to_owned()),
                    text: "8080 /run/app.sock".to_owned()
                },
                ImportWarningDto::UnknownDestination {
                    destination: "me@host".to_owned()
                },
                ImportWarningDto::HostResolutionFailed {
                    host: "broken".to_owned(),
                    message: "bad port".to_owned()
                },
            ]
        );
    }

    #[test]
    fn should_map_every_tray_status_to_its_wire_status() {
        let statuses = [
            (TrayStatus::Active, TrayStatusDto::Active),
            (TrayStatus::MissingHost, TrayStatusDto::MissingHost),
            (
                TrayStatus::Failed {
                    message: "no bus".to_owned(),
                },
                TrayStatusDto::Failed {
                    message: "no bus".to_owned(),
                },
            ),
            (TrayStatus::Disabled, TrayStatusDto::Disabled),
        ];

        for (status, expected) in statuses {
            assert_eq!(tray_status_to_dto(&status), expected);
        }
    }

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn sample_definition() -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            host: "nova".to_owned(),
            kind: TunnelKind::Local,
            listen_port: 3333,
            listen_address: Some("0.0.0.0".to_owned()),
            target_host: Some("db.internal".to_owned()),
            target_port: Some(5432),
            tags: vec!["dev".to_owned(), "adonis".to_owned()],
            open: OpenOptions {
                scheme: Some(OpenScheme::Https),
                path: Some("/admin".to_owned()),
                host: Some(OpenHost::Ip),
                with: Some("remmina-vnc".to_owned()),
            },
            standby: Some(true),
        }
    }

    #[test]
    fn should_round_trip_a_definition_through_its_dto() {
        let definition = sample_definition();

        let restored = definition_from_dto(definition_to_dto(&definition)).unwrap();

        assert_eq!(restored, definition);
    }

    #[test]
    fn should_reject_a_definition_dto_when_the_id_is_not_a_ulid() {
        let mut dto = definition_to_dto(&sample_definition());
        dto.id = "not-an-id".to_owned();

        assert!(definition_from_dto(dto).is_err());
    }

    fn sample_opener() -> Opener {
        Opener {
            id: "viewer".to_owned(),
            name: "Viewer".to_owned(),
            command: "viewer".to_owned(),
            args: vec!["{host}".to_owned()],
            ports: vec![5900],
        }
    }

    #[test]
    fn should_round_trip_an_opener_through_its_dto() {
        let opener = sample_opener();

        assert_eq!(opener_from_spec_dto(opener_to_spec_dto(&opener)), opener);
    }

    #[test]
    fn should_mark_the_source_of_an_available_opener() {
        let detected = AvailableOpener {
            opener: sample_opener(),
            source: OpenerSource::Detected,
        };

        assert_eq!(
            available_opener_to_dto(&detected).source,
            OpenerSourceDto::Detected
        );
    }

    #[test]
    fn should_carry_the_default_opener_when_converting_a_tunnel() {
        let openers = vec![AvailableOpener {
            opener: sample_opener(),
            source: OpenerSource::User,
        }];
        let definition = TunnelDefinition {
            target_port: Some(5900),
            open: OpenOptions::default(),
            standby: None,
            ..sample_definition()
        };
        let snapshot = TunnelSnapshot {
            definition,
            state: TunnelState::Stopped,
            health: TargetHealth::Unknown,
            metrics: TunnelMetrics::default(),
            is_temporary: false,
        };

        assert_eq!(
            tunnel_to_dto(&snapshot, &openers).default_opener,
            DefaultOpenerDto {
                id: "viewer".to_owned(),
                name: "Viewer".to_owned()
            }
        );
    }

    #[test]
    fn should_round_trip_every_kind_through_its_dto() {
        for kind in [TunnelKind::Local, TunnelKind::Remote, TunnelKind::Dynamic] {
            assert_eq!(kind_from_dto(kind_to_dto(kind)), kind);
        }
    }

    #[test]
    fn should_carry_every_field_when_converting_a_spec_to_a_new_tunnel() {
        let spec = definition_to_dto(&sample_definition()).spec;

        let new_tunnel = new_tunnel_from_spec(spec.clone());

        assert_eq!(
            (new_tunnel.host, new_tunnel.target_host, new_tunnel.tags),
            (spec.host, spec.target_host, spec.tags)
        );
    }

    #[test]
    fn should_carry_the_open_options_when_converting_a_spec_to_a_new_tunnel() {
        let definition = sample_definition();
        let spec = definition_to_dto(&definition).spec;

        assert_eq!(new_tunnel_from_spec(spec).open, definition.open);
    }

    #[test]
    fn should_round_trip_the_settings_through_their_dto() {
        let settings = Settings {
            start_on_launch_tags: vec!["dev".to_owned()],
            x11_display: Some("host:0.0".to_owned()),
            open_host: OpenHost::Ip,
            standby: true,
            standby_idle: Duration::from_secs(90),
            standby_notify: StandbyNotify::Both,
            standby_sound: StandbySound::Marimba,
            check_for_updates: false,
            ..Settings::default()
        };

        assert_eq!(settings_from_dto(settings_to_dto(&settings)), settings);
    }

    #[test]
    fn should_map_the_sound_events_of_the_wire_to_the_daemon_ones() {
        assert_eq!(
            [SoundEventDto::Woke, SoundEventDto::WakeFailed].map(sound_event_from_dto),
            [SoundEvent::Woke, SoundEvent::WakeFailed]
        );
    }

    #[test]
    fn should_express_the_durations_of_the_settings_in_milliseconds() {
        let dto = settings_to_dto(&Settings::default());

        assert_eq!(
            (dto.keepalive_interval_ms, dto.reconnect_max_delay_ms),
            (15_000, 60_000)
        );
    }

    #[test]
    fn should_convert_a_reconnecting_state_with_its_retry_time_in_milliseconds() {
        let state = TunnelState::Reconnecting {
            attempt: 2,
            next_retry_at: at(SECONDS_AFTER_EPOCH),
        };

        assert_eq!(
            state_to_dto(&state),
            TunnelStateDto::Reconnecting {
                attempt: 2,
                next_retry_at: SECONDS_AFTER_EPOCH * 1000
            }
        );
    }

    #[test]
    fn should_convert_a_standby_state_with_its_last_failure() {
        let state = TunnelState::Standby {
            last_failure: Some(FailureReason::Connect {
                message: "refused".to_owned(),
            }),
        };

        assert_eq!(
            state_to_dto(&state),
            TunnelStateDto::Standby {
                last_failure: Some(FailureReasonDto::Connect {
                    message: "refused".to_owned()
                })
            }
        );
    }

    #[test]
    fn should_convert_a_standby_state_without_failure() {
        assert_eq!(
            state_to_dto(&TunnelState::Standby { last_failure: None }),
            TunnelStateDto::Standby { last_failure: None }
        );
    }

    #[test]
    fn should_express_the_standby_idle_of_the_settings_in_seconds() {
        assert_eq!(settings_to_dto(&Settings::default()).standby_idle_s, 300);
    }

    #[test]
    fn should_not_forward_the_standby_events_over_the_wire() {
        let id = TunnelId::generate();

        assert_eq!(event_to_message(&EngineEvent::TunnelWoke { id }), None);
    }

    #[test]
    fn should_convert_a_failed_state_with_its_reason() {
        let state = TunnelState::Failed {
            reason: FailureReason::PortInUse {
                address: "127.0.0.1:3333".to_owned(),
            },
        };

        assert_eq!(
            state_to_dto(&state),
            TunnelStateDto::Failed {
                reason: FailureReasonDto::PortInUse {
                    address: "127.0.0.1:3333".to_owned()
                }
            }
        );
    }

    #[test]
    fn should_convert_an_unresponsive_agent_failure_with_its_timeout_in_milliseconds() {
        let state = TunnelState::Failed {
            reason: FailureReason::AgentUnresponsive {
                timeout: Duration::from_secs(30),
            },
        };

        assert_eq!(
            state_to_dto(&state),
            TunnelStateDto::Failed {
                reason: FailureReasonDto::AgentUnresponsive { timeout_ms: 30_000 }
            }
        );
    }

    fn owner() -> PortOwner {
        PortOwner {
            pid: 4242,
            process: "ssh".to_owned(),
        }
    }

    fn owner_dto() -> PortOwnerDto {
        PortOwnerDto {
            pid: 4242,
            process: "ssh".to_owned(),
        }
    }

    fn unmanaged() -> UnmanagedProcess {
        UnmanagedProcess {
            pid: 1234,
            process: "ssh".to_owned(),
            destination: Some("nova".to_owned()),
            forwards: vec![SshForward {
                kind: TunnelKind::Local,
                listen_address: Some("0.0.0.0".to_owned()),
                listen_port: 3333,
                target_host: Some("localhost".to_owned()),
                target_port: Some(3333),
            }],
        }
    }

    fn unmanaged_dto() -> ExternalProcessDto {
        ExternalProcessDto {
            pid: 1234,
            process: "ssh".to_owned(),
            destination: Some("nova".to_owned()),
            forwards: vec![ExternalForwardDto {
                kind: TunnelKindDto::Local,
                listen_address: Some("0.0.0.0".to_owned()),
                listen_port: 3333,
                target_host: Some("localhost".to_owned()),
                target_port: Some(3333),
            }],
        }
    }

    #[test]
    fn should_convert_an_external_state_with_its_owner() {
        assert_eq!(
            state_to_dto(&TunnelState::External { owner: owner() }),
            TunnelStateDto::External { owner: owner_dto() }
        );
    }

    #[test]
    fn should_convert_a_conflict_state_with_its_owner() {
        assert_eq!(
            state_to_dto(&TunnelState::Conflict {
                owner: Some(owner())
            }),
            TunnelStateDto::Conflict {
                owner: Some(owner_dto())
            }
        );
    }

    #[test]
    fn should_convert_a_conflict_state_without_owner() {
        assert_eq!(
            state_to_dto(&TunnelState::Conflict { owner: None }),
            TunnelStateDto::Conflict { owner: None }
        );
    }

    #[test]
    fn should_convert_an_unmanaged_process_with_its_forwards() {
        assert_eq!(external_to_dto(&unmanaged()), unmanaged_dto());
    }

    #[test]
    fn should_include_the_externals_when_converting_a_snapshot() {
        let snapshot = EngineSnapshot {
            tunnels: Vec::new(),
            settings: Settings::default(),
            externals: vec![unmanaged()],
            x11_anchors: Vec::new(),
            x11_launchers: Vec::new(),
            openers: Vec::new(),
        };

        assert_eq!(
            snapshot_to_dto(&snapshot, &TrayStatus::Disabled).externals,
            vec![unmanaged_dto()]
        );
    }

    fn x11_snapshot() -> EngineSnapshot {
        EngineSnapshot {
            tunnels: Vec::new(),
            settings: Settings::default(),
            externals: Vec::new(),
            x11_anchors: vec![X11AnchorSnapshot {
                host: "nova".to_owned(),
                state: X11AnchorState::Starting,
            }],
            x11_launchers: vec![X11Launcher {
                host: "nova".to_owned(),
                name: "Editor".to_owned(),
                command: "code --wait".to_owned(),
            }],
            openers: Vec::new(),
        }
    }

    #[test]
    fn should_include_the_x11_anchors_when_converting_a_snapshot() {
        assert_eq!(
            snapshot_to_dto(&x11_snapshot(), &TrayStatus::Disabled).x11_anchors,
            vec![X11AnchorDto {
                host: "nova".to_owned(),
                state: X11StateDto::Starting
            }]
        );
    }

    #[test]
    fn should_include_the_x11_launchers_when_converting_a_snapshot() {
        assert_eq!(
            snapshot_to_dto(&x11_snapshot(), &TrayStatus::Disabled).x11_launchers,
            vec![X11LauncherDto {
                host: "nova".to_owned(),
                name: "Editor".to_owned(),
                command: "code --wait".to_owned()
            }]
        );
    }

    #[test]
    fn should_convert_an_active_x11_state_with_its_display() {
        let state = X11AnchorState::Active {
            display: "localhost:10.0".to_owned(),
        };

        assert_eq!(
            x11_state_to_dto(&state),
            X11StateDto::Active {
                display: "localhost:10.0".to_owned()
            }
        );
    }

    #[test]
    fn should_convert_a_reconnecting_x11_state_with_the_retry_time_in_milliseconds() {
        let state = X11AnchorState::Reconnecting {
            attempt: 2,
            next_retry_at: at(5),
        };

        assert_eq!(
            x11_state_to_dto(&state),
            X11StateDto::Reconnecting {
                attempt: 2,
                next_retry_at: 5_000
            }
        );
    }

    #[test]
    fn should_convert_a_failed_x11_state_with_its_message() {
        let state = X11AnchorState::Failed {
            message: "refused".to_owned(),
        };

        assert_eq!(
            x11_state_to_dto(&state),
            X11StateDto::Failed {
                message: "refused".to_owned()
            }
        );
    }

    #[test]
    fn should_convert_an_x11_changed_event() {
        let event = EngineEvent::X11Changed {
            host: "nova".to_owned(),
            state: X11AnchorState::Stopped,
        };

        assert_eq!(
            event_to_message(&event).unwrap(),
            EventMessage::X11Changed(X11ChangedEvent {
                host: "nova".to_owned(),
                state: X11StateDto::Stopped
            })
        );
    }

    #[test]
    fn should_convert_an_externals_changed_event() {
        let event = EngineEvent::ExternalsChanged {
            processes: vec![unmanaged()],
        };

        assert_eq!(
            event_to_message(&event).unwrap(),
            EventMessage::ExternalsChanged(ExternalsChangedEvent {
                processes: vec![unmanaged_dto()]
            })
        );
    }

    #[test]
    fn should_convert_an_unreachable_health_with_its_start_time() {
        let health = TargetHealth::Unreachable {
            since: at(10),
            error: "refused".to_owned(),
        };

        assert_eq!(
            health_to_dto(&health),
            TargetHealthDto::Unreachable {
                since: 10_000,
                error: "refused".to_owned()
            }
        );
    }

    #[test]
    fn should_convert_metrics_with_the_last_activity_in_milliseconds() {
        let metrics = TunnelMetrics {
            active_connections: 2,
            total_connections: 5,
            bytes_in: 7,
            bytes_out: 9,
            last_activity: Some(at(3)),
        };

        assert_eq!(
            metrics_to_dto(&metrics),
            MetricsDto {
                active_connections: 2,
                total_connections: 5,
                bytes_in: 7,
                bytes_out: 9,
                last_activity: Some(3000)
            }
        );
    }

    #[test]
    fn should_map_a_time_before_the_epoch_to_zero() {
        assert_eq!(to_epoch_millis(UNIX_EPOCH - Duration::from_secs(1)), 0);
    }

    #[test]
    fn should_convert_a_tunnel_changed_event_with_the_id_as_text() {
        let id = TunnelId::generate();
        let event = EngineEvent::TunnelChanged {
            id,
            state: TunnelState::Active,
            health: TargetHealth::Reachable,
        };

        assert_eq!(
            event_to_message(&event).unwrap(),
            EventMessage::TunnelChanged(TunnelChangedEvent {
                id: id.to_string(),
                state: TunnelStateDto::Active,
                health: TargetHealthDto::Reachable
            })
        );
    }

    #[test]
    fn should_convert_a_log_event_with_its_level_and_message() {
        let id = TunnelId::generate();
        let event = EngineEvent::TunnelLog {
            id,
            entry: TunnelLogEntry {
                at: at(1),
                level: LogLevel::Warn,
                message: "slow".to_owned(),
            },
        };

        assert_eq!(
            event_to_message(&event).unwrap(),
            EventMessage::TunnelLog(TunnelLogEvent {
                id: id.to_string(),
                entry: LogEntryDto {
                    at: 1000,
                    level: LogLevelDto::Warn,
                    message: "slow".to_owned()
                }
            })
        );
    }

    #[test]
    fn should_wrap_the_message_in_a_list_when_converting_a_config_invalid_event() {
        let event = EngineEvent::ConfigInvalid {
            message: "bad file".to_owned(),
        };

        assert_eq!(
            event_to_message(&event).unwrap(),
            EventMessage::ConfigInvalid(ConfigInvalidEvent {
                messages: vec!["bad file".to_owned()]
            })
        );
    }

    #[test]
    fn should_convert_a_config_reloaded_event() {
        assert_eq!(
            event_to_message(&EngineEvent::ConfigReloaded).unwrap(),
            EventMessage::ConfigReloaded {}
        );
    }

    #[test]
    fn should_convert_every_host_key_decision_from_its_dto() {
        assert_eq!(
            [
                HostKeyDecisionDto::AcceptAndSave,
                HostKeyDecisionDto::AcceptOnce,
                HostKeyDecisionDto::Reject
            ]
            .map(host_key_decision_from_dto),
            [
                HostKeyDecision::AcceptAndSave,
                HostKeyDecision::AcceptOnce,
                HostKeyDecision::Reject
            ]
        );
    }

    #[test]
    fn should_map_the_remember_choice_to_the_retention() {
        assert_eq!(
            [None, Some(RememberDto::Session), Some(RememberDto::Keyring)].map(retention_from_dto),
            [Retention::Never, Retention::Session, Retention::Keyring]
        );
    }

    #[test]
    fn should_convert_a_changed_host_key_rejection_into_an_alert() {
        let rejection = RejectedHostKey {
            alias: "nova".to_owned(),
            reason: RejectionReason::Changed {
                known_fingerprint: "SHA256:old".to_owned(),
                path: PathBuf::from("/home/me/.ssh/known_hosts"),
                line: 4,
            },
            presented_fingerprint: "SHA256:new".to_owned(),
        };

        assert_eq!(
            rejection_to_alert(&rejection),
            HostKeyRequestEvent::Alert {
                alias: "nova".to_owned(),
                reason: HostKeyAlertReasonDto::Changed {
                    known_fingerprint: "SHA256:old".to_owned(),
                    path: "/home/me/.ssh/known_hosts".to_owned(),
                    line: 4
                },
                presented_fingerprint: "SHA256:new".to_owned()
            }
        );
    }

    #[test]
    fn should_convert_a_policy_rejection_into_an_alert() {
        let rejection = RejectedHostKey {
            alias: "nova".to_owned(),
            reason: RejectionReason::UnknownRejectedByPolicy,
            presented_fingerprint: "SHA256:new".to_owned(),
        };

        assert!(matches!(
            rejection_to_alert(&rejection),
            HostKeyRequestEvent::Alert {
                reason: HostKeyAlertReasonDto::UnknownRejectedByPolicy,
                ..
            }
        ));
    }
}
