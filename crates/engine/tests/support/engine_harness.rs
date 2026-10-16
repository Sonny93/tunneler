use std::time::Duration;

use engine::{EngineEvent, TunnelId, TunnelState};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::{Instant, sleep, timeout};

const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Reads events until `select` picks one, or panics after `limit`, listing
/// what was seen. Events skipped because the receiver lagged are not an
/// error.
pub async fn wait_for_event<Picked>(
    events: &mut Receiver<EngineEvent>,
    limit: Duration,
    mut select: impl FnMut(&EngineEvent) -> Option<Picked>,
) -> Picked {
    let mut seen = Vec::new();
    let outcome = timeout(limit, async {
        loop {
            match events.recv().await {
                Ok(event) => {
                    if let Some(picked) = select(&event) {
                        return picked;
                    }
                    seen.push(event);
                }
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => panic!("the event bus closed; saw {seen:#?}"),
            }
        }
    })
    .await;
    outcome.unwrap_or_else(|_| panic!("no matching event within {limit:?}; saw {seen:#?}"))
}

/// Waits until the tunnel reports a state the predicate accepts.
pub async fn wait_for_state(
    events: &mut Receiver<EngineEvent>,
    id: TunnelId,
    predicate: impl Fn(&TunnelState) -> bool,
    limit: Duration,
) -> TunnelState {
    wait_for_event(events, limit, |event| match event {
        EngineEvent::TunnelChanged {
            id: changed, state, ..
        } if *changed == id && predicate(state) => Some(state.clone()),
        _ => None,
    })
    .await
}

/// Polls a condition that raises no event, panicking after `limit`.
pub async fn wait_until(limit: Duration, description: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + limit;
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "timed out after {limit:?} waiting for {description}"
        );
        sleep(POLL_INTERVAL).await;
    }
}
