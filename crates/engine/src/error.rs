use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

use crate::authentication::AuthenticationMethod;
use crate::host_config::HostConfigError;
use crate::prompter::PromptError;

/// Why the host key was refused. There is no override for a changed or
/// revoked key.
#[derive(Debug, Error)]
pub enum HostKeyFailure {
    /// The host has a different key on record.
    #[error(
        "the host key changed: presented {presented_fingerprint}, recorded {known_fingerprint} ({}:{line})",
        path.display()
    )]
    Changed {
        /// The SHA-256 fingerprint the server presented.
        presented_fingerprint: String,
        /// The SHA-256 fingerprint on record.
        known_fingerprint: String,
        /// The `known_hosts` file holding the recorded key.
        path: PathBuf,
        /// The 1-based line of the recorded key.
        line: usize,
    },
    /// A `@revoked` line matches the key.
    #[error("the host key {presented_fingerprint} is revoked ({}:{line})", path.display())]
    Revoked {
        /// The SHA-256 fingerprint the server presented.
        presented_fingerprint: String,
        /// The `known_hosts` file holding the `@revoked` line.
        path: PathBuf,
        /// The 1-based line of the `@revoked` line.
        line: usize,
    },
    /// The host is unknown and `StrictHostKeyChecking` (or `BatchMode`)
    /// forbids asking.
    #[error("the host key {presented_fingerprint} is unknown and may not be accepted")]
    UnknownRejectedByPolicy {
        /// The SHA-256 fingerprint the server presented.
        presented_fingerprint: String,
    },
    /// The user declined the unknown host key.
    #[error("the host key {presented_fingerprint} was rejected by the user")]
    RejectedByUser {
        /// The SHA-256 fingerprint the server presented.
        presented_fingerprint: String,
    },
    /// The server presented a key the engine cannot check.
    #[error("unsupported host key: {reason}")]
    UnsupportedKey {
        /// Why the key cannot be checked.
        reason: String,
    },
    /// The accepted key could not be recorded.
    #[error("cannot record the host key: {source}")]
    NotRecorded {
        /// The underlying failure.
        source: known_hosts::Error,
    },
}

/// Why no TCP connection could be established.
#[derive(Debug, Error)]
pub enum TransportError {
    /// The host name did not resolve.
    #[error("cannot resolve {host}:{port}: {source}")]
    Resolve {
        /// The host name.
        host: String,
        /// The port.
        port: u16,
        /// The resolver failure.
        source: io::Error,
    },
    /// The host name resolved to no address of the requested family.
    #[error("{host}:{port} has no address of the requested family")]
    NoAddress {
        /// The host name.
        host: String,
        /// The port.
        port: u16,
    },
    /// The host is configured with a proxy this version cannot use, and
    /// connecting directly would bypass it.
    #[error("{directive} is not supported yet")]
    UnsupportedProxy {
        /// The directive that asked for a proxy.
        directive: &'static str,
    },
    /// The connection to an address failed.
    #[error("cannot connect to {address}: {source}")]
    Connect {
        /// The address that was tried last.
        address: SocketAddr,
        /// The socket failure.
        source: io::Error,
    },
    /// The connection to an address took longer than `ConnectTimeout`.
    #[error("connecting to {address} timed out after {timeout:?}")]
    Timeout {
        /// The address that was tried last.
        address: SocketAddr,
        /// The configured bound.
        timeout: Duration,
    },
}

/// Why an SSH session could not be opened.
#[derive(Debug, Error)]
pub enum ConnectError {
    /// The host alias could not be resolved.
    #[error(transparent)]
    Configuration(#[from] HostConfigError),
    /// The `known_hosts` files could not be read.
    #[error("cannot read the known hosts: {0}")]
    KnownHosts(#[from] known_hosts::Error),
    /// No TCP connection could be established.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The host key was refused.
    #[error("host key refused: {0}")]
    HostKey(HostKeyFailure),
    /// The SSH handshake did not complete within `ConnectTimeout`.
    #[error("the SSH handshake timed out after {timeout:?}")]
    HandshakeTimeout {
        /// The configured bound.
        timeout: Duration,
    },
    /// The SSH handshake failed.
    #[error("the SSH handshake failed: {0}")]
    Handshake(russh::Error),
    /// The server rejected every authentication method that was tried.
    #[error("authentication failed (tried: {})", describe_methods(tried))]
    Authentication {
        /// The methods that were attempted, in order.
        tried: Vec<AuthenticationMethod>,
    },
    /// A secret could not be asked.
    #[error(transparent)]
    Prompt(#[from] PromptError),
    /// The SSH connection broke during authentication.
    #[error("the SSH connection failed during authentication: {0}")]
    Protocol(#[from] russh::Error),
}

/// Why an operation on an open session failed.
#[derive(Debug, Error)]
pub enum SessionError {
    /// A `direct-tcpip` channel could not be opened.
    #[error("cannot open a channel to {host}:{port}: {source}")]
    OpenChannel {
        /// The target host.
        host: String,
        /// The target port.
        port: u16,
        /// The SSH failure.
        source: russh::Error,
    },
    /// The disconnect message could not be sent.
    #[error("cannot disconnect: {0}")]
    Disconnect(russh::Error),
}

fn describe_methods(methods: &[AuthenticationMethod]) -> String {
    if methods.is_empty() {
        return "no method".to_owned();
    }
    methods
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
