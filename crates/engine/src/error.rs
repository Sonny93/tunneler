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
    /// The `ProxyCommand` could not be started.
    #[error("cannot start the ProxyCommand \"{command}\": {source}")]
    ProxyCommandSpawn {
        /// The expanded command.
        command: String,
        /// The process failure.
        source: io::Error,
    },
    /// The `ProxyCommand` process has no standard input or output to talk to.
    #[error("the ProxyCommand \"{command}\" has no standard input or output")]
    ProxyCommandStdio {
        /// The expanded command.
        command: String,
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

/// Why a `ProxyJump` chain cannot be followed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JumpChainError {
    /// A host is reached through itself.
    #[error("ProxyJump cycle: {}", chain.join(" -> "))]
    Cycle {
        /// The aliases being connected, outermost first, ending with the repeated one.
        chain: Vec<String>,
    },
    /// The chain of jump hosts, each with its own `ProxyJump`, is too long.
    #[error("ProxyJump chain deeper than {maximum}: {}", chain.join(" -> "))]
    TooDeep {
        /// The aliases being connected, outermost first.
        chain: Vec<String>,
        /// The depth limit.
        maximum: usize,
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
    /// A jump host could not be reached or authenticated.
    #[error("jump host \"{hop}\": {source}")]
    Jump {
        /// The alias of the jump host that failed.
        hop: String,
        /// Why it failed.
        source: Box<ConnectError>,
    },
    /// The `ProxyJump` chain loops or is too deep.
    #[error(transparent)]
    JumpChain(#[from] JumpChainError),
    /// A secret could not be asked.
    #[error(transparent)]
    Prompt(#[from] PromptError),
    /// The channel to the next jump host or the target could not be opened.
    #[error("cannot tunnel to the next host: {0}")]
    Tunnel(#[from] SessionError),
    /// The SSH agent failed to sign during authentication.
    #[error("the SSH agent failed: {0}")]
    Agent(Box<russh::keys::Error>),
    /// The server went silent during authentication.
    #[error("the server did not answer during authentication within {timeout:?}")]
    AuthenticationTimeout {
        /// The bound that elapsed.
        timeout: Duration,
    },
    /// The SSH agent did not answer a request, typically because it waits for
    /// the user to confirm a signature.
    #[error("the SSH agent did not answer within {timeout:?}; is it waiting for a confirmation?")]
    AgentUnresponsive {
        /// The bound that elapsed.
        timeout: Duration,
    },
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
        source: Box<russh::Error>,
    },
    /// The server refused to listen for a remote forward.
    #[error("the server refused to listen on {address}:{port}")]
    ForwardRefused {
        /// The address the server was asked to listen on.
        address: String,
        /// The port the server was asked to listen on.
        port: u16,
    },
    /// A remote forward could not be requested.
    #[error("cannot request a forward on {address}:{port}: {source}")]
    RequestForward {
        /// The address the server was asked to listen on.
        address: String,
        /// The port the server was asked to listen on.
        port: u16,
        /// The SSH failure.
        source: Box<russh::Error>,
    },
    /// A remote forward could not be cancelled.
    #[error("cannot cancel the forward on {address}:{port}: {source}")]
    CancelForward {
        /// The address the server was listening on.
        address: String,
        /// The port the server was listening on.
        port: u16,
        /// The SSH failure.
        source: Box<russh::Error>,
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
