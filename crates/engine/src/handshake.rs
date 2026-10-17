use std::sync::Arc;
use std::time::Duration;

use russh::client::{Config, Handle, connect_stream};

use crate::bounded::bounded;
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
        Some(limit) => bounded(handshake, limit, state.deadline_pause())
            .await
            .map_err(|_| ConnectError::HandshakeTimeout { timeout: limit })?,
        None => handshake.await,
    };
    outcome.map_err(|error| match state.take_failure() {
        Some(failure) => ConnectError::HostKey(failure),
        None => ConnectError::Handshake(error),
    })
}
