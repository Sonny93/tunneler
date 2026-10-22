use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tracing::{error, info, warn};

use crate::clock::Clock;
use crate::runtime::events::{EngineEvent, EventBus};
use crate::runtime::log_buffer::{LogBuffer, LogLevel, TUNNEL_LOG_CAPACITY, TunnelLogEntry};
use crate::runtime::metrics::{MetricsCounters, TunnelMetrics};
use crate::runtime::state::{FailureReason, TargetHealth, TunnelState, TunnelStatus};
use crate::store::TunnelId;

/// What a tunnel publishes about itself: its status, metrics and log. The
/// tunnel task writes it; the engine reads it and the changes reach the
/// subscribers as events.
pub(crate) struct TunnelShared {
    id: TunnelId,
    bus: EventBus,
    clock: Arc<dyn Clock>,
    status: Mutex<TunnelStatus>,
    counters: Arc<MetricsCounters>,
    logs: Mutex<LogBuffer>,
}

impl TunnelShared {
    pub(crate) fn new(id: TunnelId, bus: EventBus, clock: Arc<dyn Clock>) -> Arc<Self> {
        Arc::new(Self {
            id,
            bus,
            counters: Arc::new(MetricsCounters::new(Arc::clone(&clock))),
            clock,
            status: Mutex::new(TunnelStatus::stopped()),
            logs: Mutex::new(LogBuffer::new(TUNNEL_LOG_CAPACITY)),
        })
    }

    pub(crate) fn status(&self) -> TunnelStatus {
        lock(&self.status).clone()
    }

    pub(crate) fn counters(&self) -> &Arc<MetricsCounters> {
        &self.counters
    }

    pub(crate) fn metrics(&self) -> TunnelMetrics {
        self.counters.snapshot()
    }

    pub(crate) fn logs(&self) -> Vec<TunnelLogEntry> {
        lock(&self.logs).entries()
    }

    pub(crate) fn now(&self) -> std::time::SystemTime {
        self.clock.now()
    }

    pub(crate) fn set_state(&self, state: TunnelState) {
        self.apply_state(state, false);
    }

    /// Marks the tunnel `Active`. `is_awake` tells that it was woken from
    /// standby, so that it can be sent back to it.
    pub(crate) fn set_active(&self, is_awake: bool) {
        self.apply_state(TunnelState::Active, is_awake);
    }

    fn apply_state(&self, state: TunnelState, is_awake: bool) {
        let mut status = lock(&self.status);
        if status.state == state && status.is_awake == is_awake {
            return;
        }
        let is_transition = !status.state.is_same_kind_as(&state);
        let level = level_of(&state);
        let message = format!("state: {state}");
        status.state = state;
        status.is_awake = is_awake;
        self.publish_status(&status);
        drop(status);
        if is_transition {
            self.log(level, message);
        }
    }

    pub(crate) fn set_health(&self, health: TargetHealth) {
        let mut status = lock(&self.status);
        if status.health == health {
            return;
        }
        let was_unreachable = matches!(status.health, TargetHealth::Unreachable { .. });
        let message = describe_health_change(&health, was_unreachable);
        status.health = health;
        self.publish_status(&status);
        drop(status);
        if let Some((level, message)) = message {
            self.log(level, message);
        }
    }

    pub(crate) fn observe_target_failure(&self, error: String) {
        let health = self.status().health.after_failure(self.clock.now(), error);
        self.set_health(health);
    }

    pub(crate) fn publish_woke(&self) {
        self.bus.publish(EngineEvent::TunnelWoke { id: self.id });
    }

    pub(crate) fn publish_wake_failed(&self, reason: FailureReason) {
        self.bus.publish(EngineEvent::TunnelWakeFailed {
            id: self.id,
            reason,
        });
    }

    pub(crate) fn publish_metrics(&self, metrics: TunnelMetrics) {
        self.bus.publish(EngineEvent::TunnelMetrics {
            id: self.id,
            metrics,
        });
    }

    pub(crate) fn log(&self, level: LogLevel, message: String) {
        match level {
            LogLevel::Info => info!(tunnel = %self.id, "{message}"),
            LogLevel::Warn => warn!(tunnel = %self.id, "{message}"),
            LogLevel::Error => error!(tunnel = %self.id, "{message}"),
        }
        let entry = TunnelLogEntry {
            at: self.clock.now(),
            level,
            message,
        };
        lock(&self.logs).push(entry.clone());
        self.bus
            .publish(EngineEvent::TunnelLog { id: self.id, entry });
    }

    fn publish_status(&self, status: &TunnelStatus) {
        self.bus.publish(EngineEvent::TunnelChanged {
            id: self.id,
            state: status.state.clone(),
            health: status.health.clone(),
            is_awake: status.is_awake,
        });
    }
}

fn lock<Value>(mutex: &Mutex<Value>) -> MutexGuard<'_, Value> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn level_of(state: &TunnelState) -> LogLevel {
    match state {
        TunnelState::Failed { .. } => LogLevel::Error,
        TunnelState::Reconnecting { .. } | TunnelState::Conflict { .. } => LogLevel::Warn,
        TunnelState::Stopped
        | TunnelState::Standby { .. }
        | TunnelState::Starting
        | TunnelState::Active
        | TunnelState::External { .. } => LogLevel::Info,
    }
}

fn describe_health_change(
    health: &TargetHealth,
    was_unreachable: bool,
) -> Option<(LogLevel, String)> {
    match health {
        TargetHealth::Unreachable { error, .. } if !was_unreachable => {
            Some((LogLevel::Warn, format!("target unreachable: {error}")))
        }
        TargetHealth::Reachable if was_unreachable => {
            Some((LogLevel::Info, "target reachable again".to_owned()))
        }
        TargetHealth::Unknown | TargetHealth::Reachable | TargetHealth::Unreachable { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::broadcast::Receiver;

    use crate::clock::ManualClock;
    use crate::runtime::state::FailureReason;

    use super::*;

    struct Fixture {
        shared: Arc<TunnelShared>,
        events: Receiver<EngineEvent>,
        clock: Arc<ManualClock>,
    }

    fn fixture() -> Fixture {
        let bus = EventBus::new();
        let events = bus.subscribe();
        let clock = Arc::new(ManualClock::default());
        let shared = TunnelShared::new(TunnelId::generate(), bus, Arc::clone(&clock) as _);
        Fixture {
            shared,
            events,
            clock,
        }
    }

    fn drain(events: &mut Receiver<EngineEvent>) -> Vec<EngineEvent> {
        std::iter::from_fn(|| events.try_recv().ok()).collect()
    }

    fn changed_states(events: &mut Receiver<EngineEvent>) -> Vec<TunnelState> {
        drain(events)
            .into_iter()
            .filter_map(|event| match event {
                EngineEvent::TunnelChanged { state, .. } => Some(state),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn should_start_stopped_with_an_unknown_health() {
        let Fixture { shared, .. } = fixture();

        assert_eq!(shared.status(), TunnelStatus::stopped());
    }

    #[test]
    fn should_publish_the_new_state_when_it_changes() {
        let Fixture {
            shared, mut events, ..
        } = fixture();

        shared.set_state(TunnelState::Starting);

        assert_eq!(changed_states(&mut events), [TunnelState::Starting]);
    }

    #[test]
    fn should_publish_nothing_when_the_state_does_not_change() {
        let Fixture {
            shared, mut events, ..
        } = fixture();

        shared.set_state(TunnelState::Stopped);

        assert_eq!(drain(&mut events).len(), 0);
    }

    #[test]
    fn should_publish_once_when_the_active_state_is_set_twice() {
        let Fixture {
            shared, mut events, ..
        } = fixture();

        shared.set_state(TunnelState::Active);
        shared.set_state(TunnelState::Active);

        assert_eq!(changed_states(&mut events), [TunnelState::Active]);
    }

    #[test]
    fn should_publish_again_when_the_active_tunnel_wakes_from_standby() {
        let Fixture {
            shared, mut events, ..
        } = fixture();
        shared.set_active(false);

        shared.set_active(true);

        assert!(shared.status().is_awake);
        assert_eq!(changed_states(&mut events).len(), 2);
    }

    #[test]
    fn should_not_be_awake_once_the_state_leaves_active() {
        let Fixture { shared, .. } = fixture();
        shared.set_active(true);

        shared.set_state(TunnelState::Standby { last_failure: None });

        assert!(!shared.status().is_awake);
    }

    #[test]
    fn should_log_the_transition_when_the_kind_of_state_changes() {
        let Fixture { shared, .. } = fixture();

        shared.set_state(TunnelState::Starting);

        assert_eq!(shared.logs()[0].message, "state: starting");
    }

    #[test]
    fn should_not_log_again_when_only_the_reconnection_attempt_changes() {
        let Fixture { shared, .. } = fixture();
        let retry = |attempt| TunnelState::Reconnecting {
            attempt,
            next_retry_at: std::time::UNIX_EPOCH,
        };
        shared.set_state(retry(1));

        shared.set_state(retry(2));

        assert_eq!(shared.logs().len(), 1);
    }

    #[test]
    fn should_log_an_error_when_the_tunnel_fails() {
        let Fixture { shared, .. } = fixture();

        shared.set_state(TunnelState::Failed {
            reason: FailureReason::PortInUse {
                address: "127.0.0.1:1".to_owned(),
            },
        });

        assert_eq!(shared.logs()[0].level, LogLevel::Error);
    }

    #[test]
    fn should_also_emit_each_log_entry_as_an_event() {
        let Fixture {
            shared, mut events, ..
        } = fixture();

        shared.log(LogLevel::Warn, "careful".to_owned());

        assert!(matches!(
            drain(&mut events).as_slice(),
            [EngineEvent::TunnelLog { entry, .. }] if entry.message == "careful"
        ));
    }

    #[test]
    fn should_stamp_log_entries_with_the_clock_time() {
        let Fixture { shared, clock, .. } = fixture();
        let expected = clock.now();

        shared.log(LogLevel::Info, "hello".to_owned());

        assert_eq!(shared.logs()[0].at, expected);
    }

    #[test]
    fn should_mark_the_target_unreachable_since_the_clock_time_when_a_failure_is_observed() {
        let Fixture { shared, clock, .. } = fixture();
        let expected = clock.now();

        shared.observe_target_failure("refused".to_owned());

        assert_eq!(
            shared.status().health,
            TargetHealth::Unreachable {
                since: expected,
                error: "refused".to_owned()
            }
        );
    }

    #[test]
    fn should_keep_the_first_since_when_failures_keep_coming() {
        let Fixture { shared, clock, .. } = fixture();
        let first = clock.now();
        shared.observe_target_failure("refused".to_owned());
        clock.advance(std::time::Duration::from_secs(30));

        shared.observe_target_failure("refused again".to_owned());

        assert!(matches!(
            shared.status().health,
            TargetHealth::Unreachable { since, .. } if since == first
        ));
    }

    #[test]
    fn should_log_once_when_the_target_stays_unreachable() {
        let Fixture { shared, .. } = fixture();

        shared.observe_target_failure("refused".to_owned());
        shared.observe_target_failure("refused again".to_owned());

        assert_eq!(shared.logs().len(), 1);
    }

    #[test]
    fn should_log_when_the_target_becomes_reachable_again() {
        let Fixture { shared, .. } = fixture();
        shared.observe_target_failure("refused".to_owned());

        shared.set_health(TargetHealth::Reachable);

        assert_eq!(shared.logs()[1].message, "target reachable again");
    }

    #[test]
    fn should_not_log_when_the_target_is_reachable_for_the_first_time() {
        let Fixture { shared, .. } = fixture();

        shared.set_health(TargetHealth::Reachable);

        assert_eq!(shared.logs().len(), 0);
    }

    #[test]
    fn should_publish_the_health_with_the_current_state_when_the_health_changes() {
        let Fixture {
            shared, mut events, ..
        } = fixture();

        shared.set_health(TargetHealth::Reachable);

        assert!(matches!(
            drain(&mut events).as_slice(),
            [EngineEvent::TunnelChanged {
                state: TunnelState::Stopped,
                health: TargetHealth::Reachable,
                ..
            }]
        ));
    }
}
