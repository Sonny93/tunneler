use engine::EngineEvent;
use ipc::protocol::EventMessage;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::JoinHandle;

use super::mapping::event_to_message;

/// Forwards the events of the engine to the IPC server as wire events.
///
/// When the bridge falls behind, subscribers are told to read a fresh
/// snapshot, since they missed changes. The task ends with the engine's bus.
pub(crate) fn spawn_event_bridge(
    mut engine_events: broadcast::Receiver<EngineEvent>,
    outgoing: broadcast::Sender<EventMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match engine_events.recv().await {
                Ok(event) => {
                    if let Some(message) = event_to_message(&event) {
                        let _ = outgoing.send(message);
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "the engine events overran the bridge");
                    let _ = outgoing.send(EventMessage::ConfigReloaded {});
                }
                Err(RecvError::Closed) => return,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use engine::{TargetHealth, TunnelId, TunnelState};
    use ipc::protocol::TunnelChangedEvent;
    use tokio::time::timeout;

    use super::*;

    const WAIT: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn should_forward_an_engine_event_as_a_wire_event() {
        let (engine_sender, engine_receiver) = broadcast::channel(8);
        let (outgoing, mut wire_events) = broadcast::channel(8);
        spawn_event_bridge(engine_receiver, outgoing);
        let id = TunnelId::generate();

        engine_sender
            .send(EngineEvent::TunnelChanged {
                id,
                state: TunnelState::Active,
                health: TargetHealth::Unknown,
                is_awake: false,
            })
            .unwrap();

        let forwarded = timeout(WAIT, wire_events.recv()).await.unwrap().unwrap();
        assert!(matches!(
            forwarded,
            EventMessage::TunnelChanged(TunnelChangedEvent { id: forwarded_id, .. })
                if forwarded_id == id.to_string()
        ));
    }

    #[tokio::test]
    async fn should_ask_subscribers_to_resync_when_the_bridge_lags() {
        let (engine_sender, engine_receiver) = broadcast::channel(1);
        let (outgoing, mut wire_events) = broadcast::channel(8);
        for _ in 0..3 {
            engine_sender
                .send(EngineEvent::ConfigInvalid {
                    message: "bad".to_owned(),
                })
                .unwrap();
        }
        spawn_event_bridge(engine_receiver, outgoing);

        let first = timeout(WAIT, wire_events.recv()).await.unwrap().unwrap();

        assert_eq!(first, EventMessage::ConfigReloaded {});
    }

    #[tokio::test]
    async fn should_stop_when_the_engine_bus_closes() {
        let (engine_sender, engine_receiver) = broadcast::channel::<EngineEvent>(8);
        let (outgoing, _wire_events) = broadcast::channel(8);
        let bridge = spawn_event_bridge(engine_receiver, outgoing);

        drop(engine_sender);

        assert!(timeout(WAIT, bridge).await.unwrap().is_ok());
    }
}
