use tokio::sync::broadcast;

use crate::external::UnmanagedProcess;
use crate::runtime::log_buffer::TunnelLogEntry;
use crate::runtime::metrics::TunnelMetrics;
use crate::runtime::state::{FailureReason, TargetHealth, TunnelState};
use crate::runtime::x11_state::X11AnchorState;
use crate::store::TunnelId;

/// How many events a slow subscriber may lag behind before it misses some.
const EVENT_BUS_CAPACITY: usize = 1024;

/// What the engine tells its subscribers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    /// The state or the target health of a tunnel changed.
    TunnelChanged {
        /// The tunnel.
        id: TunnelId,
        /// Its new state.
        state: TunnelState,
        /// Its new target health.
        health: TargetHealth,
    },
    /// A tunnel left standby: its first connection opened the SSH session.
    TunnelWoke {
        /// The tunnel.
        id: TunnelId,
    },
    /// A tunnel could not leave standby; it went back to it and keeps the
    /// reason.
    TunnelWakeFailed {
        /// The tunnel.
        id: TunnelId,
        /// Why the SSH session could not be opened.
        reason: FailureReason,
    },
    /// The metrics of a tunnel changed; sent at most once a second per tunnel.
    TunnelMetrics {
        /// The tunnel.
        id: TunnelId,
        /// Its current metrics.
        metrics: TunnelMetrics,
    },
    /// A tunnel logged something.
    TunnelLog {
        /// The tunnel.
        id: TunnelId,
        /// The log entry.
        entry: TunnelLogEntry,
    },
    /// The tunnel definitions or the settings changed: the store file was
    /// edited, or a tunnel was created, updated or deleted. Subscribers
    /// should read a new snapshot.
    ConfigReloaded,
    /// The `ssh` processes forwarding ports that match no configured tunnel
    /// changed.
    ExternalsChanged {
        /// The processes now reported.
        processes: Vec<UnmanagedProcess>,
    },
    /// The state of the X11 anchor of a host changed.
    X11Changed {
        /// The host alias.
        host: String,
        /// The new state of its anchor.
        state: X11AnchorState,
    },
    /// The store file was edited into something unusable; the previous
    /// configuration keeps running.
    ConfigInvalid {
        /// What is wrong, ready to be shown to the user.
        message: String,
    },
}

/// Fans [`EngineEvent`]s out to the subscribers.
#[derive(Debug, Clone)]
pub(crate) struct EventBus {
    sender: broadcast::Sender<EngineEvent>,
}

impl EventBus {
    pub(crate) fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_BUS_CAPACITY);
        Self { sender }
    }

    pub(crate) fn publish(&self, event: EngineEvent) {
        let _ = self.sender.send(event);
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<EngineEvent> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_deliver_the_event_when_a_subscriber_is_listening() {
        let bus = EventBus::new();
        let mut events = bus.subscribe();

        bus.publish(EngineEvent::ConfigReloaded);

        assert_eq!(events.try_recv().unwrap(), EngineEvent::ConfigReloaded);
    }

    #[test]
    fn should_not_fail_when_nobody_is_listening() {
        EventBus::new().publish(EngineEvent::ConfigReloaded);
    }

    #[test]
    fn should_deliver_to_every_subscriber_when_there_are_several() {
        let bus = EventBus::new();
        let mut first = bus.subscribe();
        let mut second = bus.subscribe();

        bus.publish(EngineEvent::ConfigReloaded);

        assert_eq!(first.try_recv().unwrap(), second.try_recv().unwrap());
    }
}
