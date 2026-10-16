use std::sync::Arc;
use std::time::Duration;

use russh::client::{Config, Handle, connect_stream};
use tokio::time::{Instant, sleep_until};

use crate::error::ConnectError;
use crate::host_key::{ClientHandler, HandshakeState};
use crate::transport::BoxedTransport;

/// Runs the SSH handshake, bounded by `ConnectTimeout` when it is set.
///
/// The bound is measured without the time the user spends deciding about an
/// unknown host key.
pub(crate) async fn perform_handshake(
    config: Arc<Config>,
    transport: BoxedTransport,
    handler: ClientHandler,
    state: &HandshakeState,
    connect_timeout: Option<Duration>,
) -> Result<Handle<ClientHandler>, ConnectError> {
    let handshake = connect_stream(config, transport, handler);
    let outcome = match connect_timeout {
        Some(limit) => bounded(handshake, limit, state).await?,
        None => handshake.await,
    };
    outcome.map_err(|error| match state.take_failure() {
        Some(failure) => ConnectError::HostKey(failure),
        None => ConnectError::Handshake(error),
    })
}

async fn bounded<Outcome>(
    handshake: impl Future<Output = Outcome>,
    limit: Duration,
    state: &HandshakeState,
) -> Result<Outcome, ConnectError> {
    tokio::pin!(handshake);
    let mut deadline = Instant::now() + limit;
    loop {
        tokio::select! {
            outcome = &mut handshake => return Ok(outcome),
            () = sleep_until(deadline) => {
                if !state.is_awaiting_user() {
                    return Err(ConnectError::HandshakeTimeout { timeout: limit });
                }
                deadline = Instant::now() + limit;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;

    use super::*;

    const SHORT_LIMIT: Duration = Duration::from_millis(50);

    #[tokio::test]
    async fn should_time_out_when_the_handshake_never_completes() {
        let state = HandshakeState::default();
        let outcome = bounded(pending::<()>(), SHORT_LIMIT, &state).await;
        assert!(matches!(
            outcome,
            Err(ConnectError::HandshakeTimeout { .. })
        ));
    }

    #[tokio::test]
    async fn should_not_time_out_when_the_handshake_completes_in_time() {
        let state = HandshakeState::default();
        let outcome = bounded(async { 7 }, SHORT_LIMIT, &state).await;
        assert_eq!(outcome.unwrap(), 7);
    }

    #[tokio::test]
    async fn should_not_time_out_when_the_user_is_deciding_past_the_limit() {
        let state = HandshakeState::default();
        state.begin_awaiting_user();
        let slow_decision = async {
            tokio::time::sleep(SHORT_LIMIT * 2).await;
            state.end_awaiting_user();
            7
        };
        let outcome = bounded(slow_decision, SHORT_LIMIT, &state).await;
        assert_eq!(outcome.unwrap(), 7);
    }
}
