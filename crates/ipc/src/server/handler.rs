use std::fmt;

use async_trait::async_trait;

use crate::protocol::{ClientKind, IpcError, Request, ResponsePayload};

/// Identifies a connection for as long as the daemon runs. Never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectionId(pub(crate) u64);

impl fmt::Display for ConnectionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

/// Who is on the other end of a socket, as the operating system reports it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PeerInfo {
    /// The process id, when the platform tells.
    pub process_id: Option<u32>,
    /// The effective user id, on Unix.
    pub user_id: Option<u32>,
}

/// What the daemon knows about a connection once its handshake is done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    /// The connection's identifier.
    pub id: ConnectionId,
    /// What kind of client it is.
    pub client_kind: ClientKind,
    /// The version of the client program.
    pub client_version: String,
    /// Whether events are pushed to it.
    pub subscribes_to_events: bool,
    /// Whether it answers credential and host key prompts.
    pub can_prompt: bool,
    /// The operating system's view of the peer.
    pub peer: PeerInfo,
}

/// What the daemon implements: one answer per request.
///
/// The server answers `hello` itself, so a handler never receives
/// [`Request::Hello`]. Requests of a connection are handled concurrently, so a
/// slow handler does not delay the others.
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Answers one request.
    async fn handle(
        &self,
        connection: &ConnectionInfo,
        request: Request,
    ) -> Result<ResponsePayload, IpcError>;
}
