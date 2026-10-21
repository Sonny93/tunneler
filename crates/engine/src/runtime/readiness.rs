use std::time::Duration;

use tokio::sync::broadcast::{Receiver, error::RecvError};
use tokio::time::Instant;

use crate::runtime::engine::EngineError;
use crate::runtime::events::EngineEvent;
use crate::runtime::state::TunnelState;
use crate::store::TunnelId;

/// What a tunnel state means for someone who wants to use the tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Readiness {
    /// The listening port answers, or something else provides it.
    Ready,
    /// The tunnel is not started yet or is still starting.
    NotReady,
    /// Another process holds the listening port: waiting will not help.
    PortConflict,
}

/// Whether `state` can be used, must be waited for, or is a dead end.
/// `Stopped` and `Failed` count as not ready: the caller starts the tunnel.
pub(crate) fn assess_start(state: &TunnelState) -> Readiness {
    match state {
        TunnelState::Active | TunnelState::Reconnecting { .. } | TunnelState::External { .. } => {
            Readiness::Ready
        }
        TunnelState::Conflict { .. } => Readiness::PortConflict,
        TunnelState::Stopped | TunnelState::Starting | TunnelState::Failed { .. } => {
            Readiness::NotReady
        }
    }
}

/// Like [`assess_start`], for a tunnel that was just started: a failure, a
/// stop or a retry after the first connection failed is final.
fn assess_wait(state: &TunnelState) -> WaitStep {
    match state {
        TunnelState::Failed { reason } => {
            WaitStep::Done(Err(EngineError::TunnelStartFailed(reason.clone())))
        }
        TunnelState::Stopped => WaitStep::Done(Err(EngineError::TunnelStoppedWhileStarting)),
        TunnelState::Reconnecting { .. } => WaitStep::Done(Err(EngineError::TunnelCannotConnect)),
        other => match assess_start(other) {
            Readiness::Ready => WaitStep::Done(Ok(())),
            Readiness::PortConflict => WaitStep::Done(Err(EngineError::TunnelPortConflict)),
            Readiness::NotReady => WaitStep::KeepWaiting,
        },
    }
}

enum WaitStep {
    Done(Result<(), EngineError>),
    KeepWaiting,
}

/// Waits on `events` until the tunnel `id` is ready, failed or stopped, for at
/// most `timeout`. When the receiver lags, `current_state` is read again
/// instead of trusting events that may have been lost.
pub(crate) async fn wait_until_ready(
    id: TunnelId,
    events: &mut Receiver<EngineEvent>,
    timeout: Duration,
    current_state: impl Fn() -> Option<TunnelState>,
) -> Result<(), EngineError> {
    let deadline = Instant::now() + timeout;
    loop {
        let event = tokio::time::timeout_at(deadline, events.recv())
            .await
            .map_err(|_| EngineError::TunnelStartTimedOut(timeout))?;
        let state = match event {
            Ok(EngineEvent::TunnelChanged {
                id: changed, state, ..
            }) if changed == id => state,
            Ok(_) => continue,
            Err(RecvError::Lagged(_)) => current_state().ok_or(EngineError::UnknownTunnel(id))?,
            Err(RecvError::Closed) => return Err(EngineError::ShutDown),
        };
        if let WaitStep::Done(outcome) = assess_wait(&state) {
            return outcome;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tokio::sync::broadcast;

    use super::*;
    use crate::external::PortOwner;
    use crate::runtime::state::{FailureReason, TargetHealth};

    fn owner() -> PortOwner {
        PortOwner {
            pid: 7,
            process: "ssh".to_owned(),
        }
    }

    fn change(id: TunnelId, state: TunnelState) -> EngineEvent {
        EngineEvent::TunnelChanged {
            id,
            state,
            health: TargetHealth::Unknown,
        }
    }

    #[test]
    fn should_be_ready_when_the_tunnel_is_active() {
        assert_eq!(assess_start(&TunnelState::Active), Readiness::Ready);
    }

    #[test]
    fn should_be_ready_when_the_tunnel_is_reconnecting() {
        let state = TunnelState::Reconnecting {
            attempt: 1,
            next_retry_at: SystemTime::UNIX_EPOCH,
        };

        assert_eq!(assess_start(&state), Readiness::Ready);
    }

    #[test]
    fn should_be_ready_when_an_external_process_provides_the_tunnel() {
        let state = TunnelState::External { owner: owner() };

        assert_eq!(assess_start(&state), Readiness::Ready);
    }

    #[test]
    fn should_not_be_ready_when_the_tunnel_is_stopped() {
        assert_eq!(assess_start(&TunnelState::Stopped), Readiness::NotReady);
    }

    #[test]
    fn should_not_be_ready_when_the_tunnel_failed_but_can_be_started_again() {
        let state = TunnelState::Failed {
            reason: FailureReason::Connect {
                message: "refused".to_owned(),
            },
        };

        assert_eq!(assess_start(&state), Readiness::NotReady);
    }

    #[test]
    fn should_be_unusable_when_another_process_holds_the_port() {
        let state = TunnelState::Conflict { owner: None };

        assert_eq!(assess_start(&state), Readiness::PortConflict);
    }

    #[tokio::test]
    async fn should_resolve_when_the_tunnel_becomes_active() {
        let id = TunnelId::generate();
        let (sender, mut events) = broadcast::channel(8);
        sender.send(change(id, TunnelState::Starting)).unwrap();
        sender.send(change(id, TunnelState::Active)).unwrap();

        let outcome = wait_until_ready(id, &mut events, Duration::from_secs(5), || None).await;

        assert!(outcome.is_ok());
    }

    #[tokio::test]
    async fn should_ignore_the_events_of_other_tunnels() {
        let id = TunnelId::generate();
        let (sender, mut events) = broadcast::channel(8);
        sender
            .send(change(TunnelId::generate(), TunnelState::Active))
            .unwrap();
        sender.send(change(id, TunnelState::Active)).unwrap();

        let outcome = wait_until_ready(id, &mut events, Duration::from_secs(5), || None).await;

        assert!(outcome.is_ok());
    }

    #[tokio::test]
    async fn should_fail_with_the_reason_when_the_tunnel_fails() {
        let id = TunnelId::generate();
        let (sender, mut events) = broadcast::channel(8);
        let reason = FailureReason::Connect {
            message: "refused".to_owned(),
        };
        sender
            .send(change(
                id,
                TunnelState::Failed {
                    reason: reason.clone(),
                },
            ))
            .unwrap();

        let outcome = wait_until_ready(id, &mut events, Duration::from_secs(5), || None).await;

        assert!(matches!(outcome, Err(EngineError::TunnelStartFailed(found)) if found == reason));
    }

    #[tokio::test]
    async fn should_fail_when_the_first_connection_fails_and_the_tunnel_retries() {
        let id = TunnelId::generate();
        let (sender, mut events) = broadcast::channel(8);
        sender
            .send(change(
                id,
                TunnelState::Reconnecting {
                    attempt: 1,
                    next_retry_at: SystemTime::UNIX_EPOCH,
                },
            ))
            .unwrap();

        let outcome = wait_until_ready(id, &mut events, Duration::from_secs(5), || None).await;

        assert!(matches!(outcome, Err(EngineError::TunnelCannotConnect)));
    }

    #[tokio::test]
    async fn should_fail_when_another_process_takes_the_port() {
        let id = TunnelId::generate();
        let (sender, mut events) = broadcast::channel(8);
        sender
            .send(change(id, TunnelState::Conflict { owner: None }))
            .unwrap();

        let outcome = wait_until_ready(id, &mut events, Duration::from_secs(5), || None).await;

        assert!(matches!(outcome, Err(EngineError::TunnelPortConflict)));
    }

    #[tokio::test]
    async fn should_fail_when_the_tunnel_is_stopped_meanwhile() {
        let id = TunnelId::generate();
        let (sender, mut events) = broadcast::channel(8);
        sender.send(change(id, TunnelState::Stopped)).unwrap();

        let outcome = wait_until_ready(id, &mut events, Duration::from_secs(5), || None).await;

        assert!(matches!(
            outcome,
            Err(EngineError::TunnelStoppedWhileStarting)
        ));
    }

    #[tokio::test]
    async fn should_time_out_when_nothing_happens() {
        let id = TunnelId::generate();
        let (_sender, mut events) = broadcast::channel(8);

        let outcome = wait_until_ready(id, &mut events, Duration::from_millis(20), || None).await;

        assert!(matches!(outcome, Err(EngineError::TunnelStartTimedOut(_))));
    }

    #[tokio::test]
    async fn should_read_the_current_state_when_events_were_lost() {
        let id = TunnelId::generate();
        let (sender, mut events) = broadcast::channel(1);
        sender.send(change(id, TunnelState::Starting)).unwrap();
        sender.send(change(id, TunnelState::Starting)).unwrap();

        let outcome = wait_until_ready(id, &mut events, Duration::from_secs(5), || {
            Some(TunnelState::Active)
        })
        .await;

        assert!(outcome.is_ok());
    }
}
