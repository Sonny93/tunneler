//! What discovering and resynchronising the devslot tunnels of a host
//! reports.

use std::sync::Arc;

use thiserror::Error;

use super::analysis::DevslotStaleTunnel;
use super::listing::{DevslotInstance, DevslotWarning};
use crate::error::ConnectError;
use crate::store::{TunnelDefinition, TunnelId};

/// The devslot instances of a host, compared with its tunnels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevslotDiscovery {
    /// The host alias that was asked.
    pub host: String,
    /// Whether the discovered tunnels are saved by default: the host
    /// override, else the setting.
    pub persist_default: bool,
    /// The instances the host lists, each port with its status.
    pub instances: Vec<DevslotInstance>,
    /// The devslot tunnels of the host that no longer match the listing.
    pub stale: Vec<DevslotStaleTunnel>,
    /// What the listing had that was left out.
    pub warnings: Vec<DevslotWarning>,
}

/// A stale tunnel a client asks to remove, as the discovery returned it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevslotResyncTunnel {
    /// The tunnel as the discovery returned it.
    pub definition: TunnelDefinition,
    /// Whether the tunnel lived only in the daemon.
    pub is_temporary: bool,
}

/// What a resync did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DevslotResync {
    /// The tunnels that are gone: removed now, or already absent.
    pub removed: Vec<TunnelId>,
    /// The tunnels left alone because they changed since the discovery.
    pub skipped: Vec<TunnelId>,
}

/// Why the instances of a host could not be listed.
#[derive(Debug, Error)]
pub enum DevslotError {
    /// The SSH session could not be opened.
    #[error("{source}")]
    Connect {
        /// The host alias.
        host: String,
        /// Why the session could not be opened.
        source: Arc<ConnectError>,
    },
    /// The session manager shut down.
    #[error("the engine is shutting down")]
    ShuttingDown,
    /// The `ds` program does not exist on the host.
    #[error("\"{path}\" was not found on host {host}")]
    NotFound {
        /// The host alias.
        host: String,
        /// The program that was run.
        path: String,
    },
    /// The command failed or did not report how it ended.
    #[error("the devslot listing failed on host {host} ({}): {stderr}", describe_status(*status))]
    Failed {
        /// The host alias.
        host: String,
        /// The exit status; `None` when the command was killed by a signal,
        /// refused, or the channel closed without one.
        status: Option<u32>,
        /// The end of the standard error, cleaned.
        stderr: String,
    },
    /// The command succeeded but its output cannot be used.
    #[error("the devslot listing of host {host} is not usable: {reason}")]
    InvalidOutput {
        /// The host alias.
        host: String,
        /// What is wrong with the output.
        reason: String,
    },
    /// The command did not end in time.
    #[error("the devslot listing of host {host} did not end within {seconds} seconds")]
    Timeout {
        /// The host alias.
        host: String,
        /// The time limit, in seconds.
        seconds: u64,
    },
    /// The host lists no instance with this name.
    #[error("host {host} does not list the instance \"{instance}\"")]
    InstanceNotFound {
        /// The host alias.
        host: String,
        /// The instance name that was asked for.
        instance: String,
    },
}

fn describe_status(status: Option<u32>) -> String {
    status.map_or_else(
        || "no exit status".to_owned(),
        |status| format!("exit status {status}"),
    )
}
