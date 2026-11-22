//! The wire types of the devslot discovery: `devslot.discover` and
//! `devslot.resync`.

use serde::{Deserialize, Serialize};

use crate::protocol::dto::TunnelDefinitionDto;

/// Marks a temporary tunnel as started from a devslot listing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotOriginDto {
    /// The tag of the project, as the discovery returned it: it starts with
    /// `ds-`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_tag: Option<String>,
}

/// Whether a listed port can be tunneled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DevslotPortStatusDto {
    /// Nothing forwards to the port yet.
    New,
    /// A tunnel of the host already forwards to the port; `tunnelId` names it.
    AlreadyTunneled,
    /// Several instances list the port.
    Ambiguous,
}

/// A port of a devslot instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotPortDto {
    /// The environment variable that names the port.
    pub var: String,
    /// The port on the host.
    pub port: u16,
    /// What the port serves; devslot says `web` or `service`.
    pub kind: String,
    /// The compose service of a service port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// The port inside the service container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<u16>,
    /// The URL of a web port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The `<label>.localhost` name of the URL, when it is a valid one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_name: Option<String>,
    /// Whether the port is forced: it may be shared with another instance.
    #[serde(default)]
    pub forced: bool,
    /// Whether the port can be tunneled.
    pub status: DevslotPortStatusDto,
    /// The tunnel that already forwards to the port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tunnel_id: Option<String>,
}

/// A devslot instance of a host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotInstanceDto {
    /// The unique name of the instance.
    pub name: String,
    /// The name of the repository; not unique.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// The tag of the project, computed by the daemon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_tag: Option<String>,
    /// The path of the checkout on the host.
    pub path: String,
    /// Whether the checkout is a linked git worktree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<bool>,
    /// The slot the instance holds.
    pub slot: u32,
    /// The URL of the application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_url: Option<String>,
    /// Whether the compose services run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub services_running: Option<bool>,
    /// The ports of the instance.
    pub ports: Vec<DevslotPortDto>,
}

/// Why a devslot tunnel no longer matches the listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DevslotStaleReasonDto {
    /// No instance lists its port.
    PortGone,
    /// Its port now belongs to another project or host name.
    Reassigned,
}

/// What to do with a stale devslot tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DevslotStaleActionDto {
    /// Remove it without asking.
    Remove,
    /// Ask the user.
    Ask,
}

/// A devslot tunnel that no longer matches the listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotStaleDto {
    /// The tunnel.
    pub tunnel: TunnelDefinitionDto,
    /// Whether it lives only in the daemon.
    pub is_temporary: bool,
    /// Why it is stale.
    pub reason: DevslotStaleReasonDto,
    /// What to do with it.
    pub action: DevslotStaleActionDto,
}

/// Something the listing had that was left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DevslotWarningDto {
    /// A port with a number outside 1 to 65535 was skipped.
    InvalidPort {
        /// The instance that lists it.
        instance: String,
        /// The variable that names it.
        var: String,
    },
    /// An instance has a project but no tag could be made of it.
    ProjectWithoutTag {
        /// The instance.
        instance: String,
    },
}

/// The result of `devslot.discover`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotDiscoveryDto {
    /// The host alias that was asked.
    pub host: String,
    /// Whether discovered tunnels are saved by default.
    pub persist_default: bool,
    /// The instances the host lists.
    pub instances: Vec<DevslotInstanceDto>,
    /// The devslot tunnels of the host that no longer match the listing.
    pub stale: Vec<DevslotStaleDto>,
    /// What the listing had that was left out.
    pub warnings: Vec<DevslotWarningDto>,
}

/// A stale tunnel to remove, as `devslot.discover` returned it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotResyncTunnelDto {
    /// The tunnel identifier.
    pub id: String,
    /// The definition the discovery returned.
    pub definition: TunnelDefinitionDto,
    /// Whether the tunnel lived only in the daemon.
    pub is_temporary: bool,
}

/// The parameters of `devslot.resync`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotResyncParams {
    /// The `ssh_config` alias.
    pub host: String,
    /// The tunnels to remove.
    pub tunnels: Vec<DevslotResyncTunnelDto>,
}

/// The result of `devslot.resync`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevslotResyncResultDto {
    /// The tunnels that are gone: removed now, or already absent.
    pub removed: Vec<String>,
    /// The tunnels left alone because they changed since the discovery.
    pub skipped: Vec<String>,
}
