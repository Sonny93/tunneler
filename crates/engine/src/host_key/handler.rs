use russh::client::{DisconnectReason, Handler};
use russh::keys::PublicKeyOrCertificate;
use tokio::sync::watch;

use super::checker::HostKeyChecker;

/// The russh client handler: it decides about the server key, leaving the
/// reason of a refusal in the shared handshake state, and signals when the
/// connection ends.
pub(crate) struct ClientHandler {
    checker: HostKeyChecker,
    closed_signal: watch::Sender<bool>,
}

impl ClientHandler {
    pub(crate) fn new(checker: HostKeyChecker, closed_signal: watch::Sender<bool>) -> Self {
        Self {
            checker,
            closed_signal,
        }
    }
}

impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        match self.checker.check(server_public_key).await {
            Ok(()) => Ok(true),
            Err(failure) => {
                self.checker.record_failure(failure);
                Ok(false)
            }
        }
    }

    #[allow(
        clippy::unused_async_trait_impl,
        reason = "the russh handler trait requires an async method even when nothing awaits"
    )]
    async fn disconnected(
        &mut self,
        reason: DisconnectReason<Self::Error>,
    ) -> Result<(), Self::Error> {
        self.closed_signal.send_replace(true);
        match reason {
            DisconnectReason::ReceivedDisconnect(_) => Ok(()),
            DisconnectReason::Error(error) => Err(error),
        }
    }
}
