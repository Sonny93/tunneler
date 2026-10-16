use russh::client::Handler;
use russh::keys::PublicKeyOrCertificate;

use super::checker::HostKeyChecker;

/// The russh client handler: it only decides about the server key and leaves
/// the reason of a refusal in the shared handshake state.
pub(crate) struct ClientHandler {
    checker: HostKeyChecker,
}

impl ClientHandler {
    pub(crate) fn new(checker: HostKeyChecker) -> Self {
        Self { checker }
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
}
