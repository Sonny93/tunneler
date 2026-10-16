use std::io;

use engine::{EngineError, StoreError};
use ipc::{BindError, EndpointError};
use ssh_config::EnvironmentError;
use thiserror::Error;

/// Why the daemon could not start or keep running.
#[derive(Debug, Error)]
pub enum DaemonError {
    /// Another daemon answers on the endpoint.
    #[error("Tunneler is already running")]
    AlreadyRunning,
    /// The endpoint is held by something that is not a responding daemon and
    /// could not be reclaimed.
    #[error("the endpoint is in use by another process that is not a Tunneler daemon")]
    EndpointInUse,
    /// The endpoint could not be determined.
    #[error(transparent)]
    Endpoint(#[from] EndpointError),
    /// The endpoint could not be bound.
    #[error(transparent)]
    Bind(#[from] BindError),
    /// A socket left behind by a crashed daemon could not be removed.
    #[error("cannot remove the stale socket: {0}")]
    ReclaimStale(#[source] io::Error),
    /// The tunnels file could not be read or written.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The tunnel engine failed to start.
    #[error(transparent)]
    Engine(#[from] EngineError),
    /// The local machine could not be described to the `ssh_config` resolver.
    #[error("cannot read the local environment: {0}")]
    Environment(#[from] EnvironmentError),
}
