use std::sync::Arc;

use russh::Channel;
use russh::ChannelOpenFailure;
use russh::client::{ChannelOpenHandle, DisconnectReason, Handler, Msg, Session};
use russh::keys::PublicKeyOrCertificate;
use tokio::sync::watch;
use tracing::warn;

use super::checker::HostKeyChecker;
use crate::remote_forwards::{ForwardedConnection, RemoteForwardRegistry};

/// The russh client handler: it decides about the server key, leaving the
/// reason of a refusal in the shared handshake state, hands the connections
/// of remote forwards to their tunnel, and signals when the connection ends.
pub(crate) struct ClientHandler {
    checker: HostKeyChecker,
    closed_signal: watch::Sender<bool>,
    remote_forwards: Arc<RemoteForwardRegistry>,
}

impl ClientHandler {
    pub(crate) fn new(
        checker: HostKeyChecker,
        closed_signal: watch::Sender<bool>,
        remote_forwards: Arc<RemoteForwardRegistry>,
    ) -> Self {
        Self {
            checker,
            closed_signal,
            remote_forwards,
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

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let Some(sender) = self
            .remote_forwards
            .route(connected_address, connected_port)
        else {
            warn!(
                connected_address,
                connected_port, "no remote forward listens there; connection refused"
            );
            reply
                .reject(ChannelOpenFailure::AdministrativelyProhibited)
                .await;
            return Ok(());
        };
        let connection = ForwardedConnection {
            channel,
            reply,
            originator: format!("{originator_address}:{originator_port}"),
        };
        if let Err(unsent) = sender.send(connection) {
            warn!(
                connected_address,
                connected_port, "the remote forward is gone; connection refused"
            );
            unsent
                .0
                .reply
                .reject(ChannelOpenFailure::AdministrativelyProhibited)
                .await;
        }
        Ok(())
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
