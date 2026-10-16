//! Conversions between the engine types and the wire types of the IPC protocol.
//!
//! The wire types are the daemon's public contract, so none of the engine types
//! leak through: every conversion is explicit and covered by a test.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use engine::{
    EngineEvent, EngineSnapshot, FailureReason, HostKeyDecision, InvalidTunnelId, LogLevel,
    NewTunnel, RejectedHostKey, RejectionReason, Settings, TargetHealth, TunnelDefinition,
    TunnelId, TunnelKind, TunnelLogEntry, TunnelMetrics, TunnelSnapshot, TunnelState,
};
use ipc::protocol::{
    ConfigInvalidEvent, EpochMillis, EventMessage, FailureReasonDto, HostKeyAlertReasonDto,
    HostKeyDecisionDto, HostKeyRequestEvent, LogEntryDto, LogLevelDto, MetricsDto, SettingsDto,
    SnapshotDto, TargetHealthDto, TunnelChangedEvent, TunnelDefinitionDto, TunnelDto,
    TunnelKindDto, TunnelLogEvent, TunnelMetricsEvent, TunnelSpecDto, TunnelStateDto,
};

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
            name: definition.name.clone(),
            host: definition.host.clone(),
            kind: kind_to_dto(definition.kind),
            listen_port: definition.listen_port,
            listen_address: definition.listen_address.clone(),
            target_host: definition.target_host.clone(),
            target_port: definition.target_port,
            tags: definition.tags.clone(),
        },
    }
}

pub(crate) fn definition_from_dto(
    dto: TunnelDefinitionDto,
) -> Result<TunnelDefinition, InvalidTunnelId> {
    let id: TunnelId = dto.id.parse()?;
    let spec = dto.spec;
    Ok(TunnelDefinition {
        id,
        name: spec.name,
        host: spec.host,
        kind: kind_from_dto(spec.kind),
        listen_port: spec.listen_port,
        listen_address: spec.listen_address,
        target_host: spec.target_host,
        target_port: spec.target_port,
        tags: spec.tags,
    })
}

pub(crate) fn new_tunnel_from_spec(spec: TunnelSpecDto) -> NewTunnel {
    NewTunnel {
        name: spec.name,
        host: spec.host,
        kind: kind_from_dto(spec.kind),
        listen_port: spec.listen_port,
        listen_address: spec.listen_address,
        target_host: spec.target_host,
        target_port: spec.target_port,
        tags: spec.tags,
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
        FailureReason::UnsupportedKind(kind) => FailureReasonDto::UnsupportedKind {
            tunnel_kind: kind_to_dto(*kind),
        },
    }
}

pub(crate) fn state_to_dto(state: &TunnelState) -> TunnelStateDto {
    match state {
        TunnelState::Stopped => TunnelStateDto::Stopped,
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

pub(crate) fn tunnel_to_dto(snapshot: &TunnelSnapshot) -> TunnelDto {
    TunnelDto {
        definition: definition_to_dto(&snapshot.definition),
        state: state_to_dto(&snapshot.state),
        health: health_to_dto(&snapshot.health),
        metrics: metrics_to_dto(&snapshot.metrics),
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
        health_probe: settings.health_probe,
        restore_on_launch: settings.restore_on_launch,
        start_on_launch_tags: settings.start_on_launch_tags.clone(),
        launch_at_login: settings.launch_at_login,
        x11_display: settings.x11_display.clone(),
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
        health_probe: dto.health_probe,
        restore_on_launch: dto.restore_on_launch,
        start_on_launch_tags: dto.start_on_launch_tags,
        launch_at_login: dto.launch_at_login,
        x11_display: dto.x11_display,
    }
}

pub(crate) fn snapshot_to_dto(snapshot: &EngineSnapshot) -> SnapshotDto {
    SnapshotDto {
        tunnels: snapshot.tunnels.iter().map(tunnel_to_dto).collect(),
        settings: settings_to_dto(&snapshot.settings),
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

pub(crate) fn event_to_message(event: &EngineEvent) -> EventMessage {
    match event {
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
    }
}

pub(crate) fn host_key_decision_from_dto(decision: HostKeyDecisionDto) -> HostKeyDecision {
    match decision {
        HostKeyDecisionDto::AcceptAndSave => HostKeyDecision::AcceptAndSave,
        HostKeyDecisionDto::AcceptOnce => HostKeyDecision::AcceptOnce,
        HostKeyDecisionDto::Reject => HostKeyDecision::Reject,
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

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn sample_definition() -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            name: "Adonis".to_owned(),
            host: "nova".to_owned(),
            kind: TunnelKind::Local,
            listen_port: 3333,
            listen_address: Some("0.0.0.0".to_owned()),
            target_host: Some("db.internal".to_owned()),
            target_port: Some(5432),
            tags: vec!["dev".to_owned(), "adonis".to_owned()],
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
            (new_tunnel.name, new_tunnel.target_host, new_tunnel.tags),
            (spec.name, spec.target_host, spec.tags)
        );
    }

    #[test]
    fn should_round_trip_the_settings_through_their_dto() {
        let settings = Settings {
            start_on_launch_tags: vec!["dev".to_owned()],
            x11_display: Some("host:0.0".to_owned()),
            ..Settings::default()
        };

        assert_eq!(settings_from_dto(settings_to_dto(&settings)), settings);
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
            event_to_message(&event),
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
            event_to_message(&event),
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
            event_to_message(&event),
            EventMessage::ConfigInvalid(ConfigInvalidEvent {
                messages: vec!["bad file".to_owned()]
            })
        );
    }

    #[test]
    fn should_convert_a_config_reloaded_event() {
        assert_eq!(
            event_to_message(&EngineEvent::ConfigReloaded),
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
