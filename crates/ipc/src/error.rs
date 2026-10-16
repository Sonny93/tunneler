use std::io;
use std::path::PathBuf;

use serde_json::Value;
use thiserror::Error;

use crate::protocol::{ErrorCode, IpcError, MalformedFrame};

/// Why the default endpoint could not be determined.
#[derive(Debug, Error)]
pub enum EndpointError {
    /// The name of the current user is unknown, so the pipe cannot be named.
    #[error("cannot determine the current user name: {0}")]
    UnknownUser(String),
}

/// Why a server could not bind its endpoint.
#[derive(Debug, Error)]
pub enum BindError {
    /// Something already listens there, or a socket file was left behind.
    ///
    /// Use [`Endpoint::is_daemon_alive`](crate::Endpoint::is_daemon_alive) to
    /// tell the two apart.
    #[error("the endpoint is already in use")]
    AddressInUse,
    /// The directory holding the socket could be used by other users.
    #[error("{} must be a directory owned by you and closed to others", .path.display())]
    InsecureDirectory {
        /// The offending directory.
        path: PathBuf,
    },
    /// Binding failed for another reason.
    #[error("cannot bind the endpoint: {0}")]
    Io(#[from] io::Error),
}

/// Why a message could not be sent to a connection.
#[derive(Debug, Error)]
pub enum SendError {
    /// The connection is closed or was never opened.
    #[error("the connection is not open")]
    NotConnected,
    /// The client did not keep up and was disconnected.
    #[error("the client was too slow and was disconnected")]
    QueueFull,
    /// The event cannot be represented as JSON.
    #[error("cannot encode the event: {0}")]
    Encoding(#[from] serde_json::Error),
}

/// Why a client call failed.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Nothing listens on the endpoint.
    #[error("the daemon is not running")]
    DaemonNotRunning,
    /// The daemon speaks another protocol version.
    #[error("the daemon speaks protocol {daemon}, this client protocol {client}")]
    ProtocolMismatch {
        /// The version the daemon speaks.
        daemon: u32,
        /// The version this client speaks.
        client: u32,
    },
    /// The daemon answered with an error.
    #[error("{code}: {message}")]
    Remote {
        /// The error code.
        code: ErrorCode,
        /// The explanation.
        message: String,
        /// Machine-readable details, whose shape depends on the code.
        data: Option<Value>,
    },
    /// The connection ended before the answer arrived.
    #[error("the connection to the daemon was closed")]
    ConnectionClosed,
    /// Reading or writing the socket failed.
    #[error("communication with the daemon failed: {0}")]
    Io(#[from] io::Error),
    /// The daemon sent something this client cannot read.
    #[error("{0}")]
    InvalidResponse(String),
}

impl From<IpcError> for ClientError {
    fn from(error: IpcError) -> Self {
        Self::Remote {
            code: error.code,
            message: error.message,
            data: error.data,
        }
    }
}

impl From<MalformedFrame> for ClientError {
    fn from(error: MalformedFrame) -> Self {
        Self::InvalidResponse(error.to_string())
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidResponse(format!("unexpected result shape: {error}"))
    }
}
