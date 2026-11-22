//! What opening one devslot instance reports.

use crate::store::TunnelId;

/// A port of the instance that has a tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevslotUpPort {
    /// The port on the host.
    pub remote_port: u16,
    /// The port the tunnel listens on, on this machine.
    pub listen_port: u16,
    /// The tunnel.
    pub tunnel_id: TunnelId,
}

/// Why a port of the instance was not tunneled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevslotSkipReason {
    /// Something holds the port on this machine and no other port is free.
    LocalPortBusy,
    /// A stale tunnel of the port waits for the user's decision.
    NeedsReview,
    /// The port is below 1024: it is left to the import panel.
    PrivilegedPort,
    /// A tunnel the user configured forwards the port: it is not started
    /// nor duplicated.
    ExistingTunnel,
}

/// A port of the instance that was left alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevslotSkippedPort {
    /// The port on the host.
    pub remote_port: u16,
    /// Why it was left alone.
    pub reason: DevslotSkipReason,
}

/// A port of the instance whose tunnel could not be created or started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevslotFailedPort {
    /// The port on the host.
    pub remote_port: u16,
    /// What went wrong.
    pub message: String,
}

/// A stale route that was removed before the instance was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevslotRemovedRoute {
    /// The port the removed tunnel listened on.
    pub listen_port: u16,
    /// The port on the host it forwarded to.
    pub remote_port: u16,
    /// Whether it lived only in the daemon.
    pub is_temporary: bool,
}

/// What opening one devslot instance did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevslotUp {
    /// The host alias that was asked.
    pub host: String,
    /// The instance that was asked.
    pub instance: String,
    /// The ports whose tunnel was created now.
    pub started: Vec<DevslotUpPort>,
    /// The ports that already had a tunnel, started when it was stopped.
    pub existing: Vec<DevslotUpPort>,
    /// The ports that were left alone.
    pub skipped: Vec<DevslotSkippedPort>,
    /// The ports whose tunnel could not be created or started.
    pub failed: Vec<DevslotFailedPort>,
    /// The stale routes that were removed first.
    pub removed: Vec<DevslotRemovedRoute>,
    /// How many stale tunnels were kept for the user to decide.
    pub needs_review: usize,
    /// The tunnel of the application port, when it is up: the one to open.
    pub app_tunnel: Option<TunnelId>,
    /// The URL of the application, as the host lists it.
    pub app_url: Option<String>,
    /// Whether the port of the application is listed by several instances:
    /// the browser may then reach another one.
    pub is_app_port_shared: bool,
}
