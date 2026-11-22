//! The wire types of `devslot.up` and of the link a window relays.

use serde::{Deserialize, Serialize};

use crate::protocol::devslot_names::{DevslotHostName, DevslotInstanceName};

/// The parameters of `devslot.up`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotUpParams {
    /// The `ssh_config` alias, within the bounds of the devslot contract.
    pub host: DevslotHostName,
    /// The `name` of the instance in `ds ls --json`, within the bounds of the
    /// devslot contract.
    pub instance: DevslotInstanceName,
    /// Whether the daemon opens the application in the browser once the
    /// tunnels are up.
    pub open: bool,
}

/// A port of the instance that has a tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotUpPortDto {
    /// The port on the host.
    pub remote_port: u16,
    /// The port the tunnel listens on, on this machine.
    pub listen_port: u16,
    /// The tunnel.
    pub tunnel_id: String,
}

/// Why `devslot.up` did not tunnel a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DevslotSkipReasonDto {
    /// Something holds the port on this machine and no other port is free.
    LocalPortBusy,
    /// A stale tunnel of the port waits for the user's decision.
    NeedsReview,
    /// The port is below 1024: it is left to the import panel.
    PrivilegedPort,
    /// A tunnel the user configured already forwards the port.
    ExistingTunnel,
}

/// A port of the instance that `devslot.up` left alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotSkippedPortDto {
    /// The port on the host.
    pub remote_port: u16,
    /// Why it was left alone.
    pub reason: DevslotSkipReasonDto,
}

/// A port of the instance whose tunnel could not be created or started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotFailedPortDto {
    /// The port on the host.
    pub remote_port: u16,
    /// What went wrong.
    pub message: String,
}

/// A stale route that `devslot.up` removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotRemovedRouteDto {
    /// The port the removed tunnel listened on.
    pub listen_port: u16,
    /// The port on the host it forwarded to.
    pub remote_port: u16,
    /// Whether it lived only in the daemon.
    pub is_temporary: bool,
}

/// The result of `devslot.up`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotUpResultDto {
    /// The host alias that was asked.
    pub host: String,
    /// The instance that was asked.
    pub instance: String,
    /// The ports whose tunnel was created now.
    pub started: Vec<DevslotUpPortDto>,
    /// The ports that already had a tunnel, started when it was stopped.
    pub existing: Vec<DevslotUpPortDto>,
    /// The ports that were left alone.
    pub skipped: Vec<DevslotSkippedPortDto>,
    /// The ports whose tunnel could not be created or started.
    pub failed: Vec<DevslotFailedPortDto>,
    /// The stale routes that were removed first.
    pub removed: Vec<DevslotRemovedRouteDto>,
    /// How many stale tunnels were kept for the user to decide.
    pub needs_review: u32,
    /// The URL of the application, as the host lists it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_url: Option<String>,
    /// Whether the port of the application is listed by several instances: the
    /// browser may then reach another one.
    pub app_port_shared: bool,
    /// The address that was opened in the browser.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened: Option<String>,
    /// Why nothing was opened, when opening was asked and failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_error: Option<String>,
}

/// What a window relays from a `tunneler://devslot/...` link it was started
/// with, so that the window that stays open can ask the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DevslotLinkDto {
    /// A link to open one instance.
    DevslotUp {
        /// The host alias.
        host: DevslotHostName,
        /// The instance name.
        instance: DevslotInstanceName,
    },
    /// A link that does not follow the contract; its text is not relayed.
    Invalid,
}
