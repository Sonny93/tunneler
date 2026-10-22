//! IPC between the daemon and its clients: the UI and the CLI.
//!
//! The wire format is JSON lines over a local socket (a Unix socket, or a named
//! pipe on Windows), restricted to the current user. The [`protocol`] module
//! defines the messages, [`IpcServer`] is the daemon side and [`IpcClient`] the
//! client side.
//!
//! [`detect_portable_layout`] tells where the files are when the program runs
//! in portable mode, for the daemon and the UI alike.
//!
//! This crate does not depend on the engine: the wire types are its own, which
//! is what keeps the protocol stable while the engine evolves.
//!
//! # Known gaps
//!
//! - The named pipe restriction on Windows (owner and `SYSTEM` only) and the
//!   Windows endpoint naming have never been compiled or run.
//! - Bytes of secrets sit in the JSON parser's buffers before they are wrapped
//!   in a wiping [`protocol::Secret`].
#![warn(missing_docs)]

mod client;
mod endpoint;
mod error;
mod launcher;
mod portable;
pub mod protocol;
mod server;

pub use client::{EventReceiver, IpcClient};
pub use endpoint::{Endpoint, default_endpoint};
pub use error::{BindError, ClientError, EndpointError, LaunchError, SendError};
pub use launcher::{DEFAULT_DAEMON_START_TIMEOUT, ensure_daemon_running, locate_companion_binary};
pub use portable::{
    PORTABLE_MARKER_FILE_NAME, PortableLayout, detect_portable_layout, portable_layout,
};
pub use server::{
    ConnectionId, ConnectionInfo, ConnectionNotification, IpcServer, OUTBOUND_QUEUE_CAPACITY,
    PeerInfo, RequestHandler, ServerHandle, ServerOptions,
};
