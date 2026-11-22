//! What `ds ls --json` lists, once parsed and cleaned.

use crate::store::TunnelId;

/// Whether a listed port can be tunneled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevslotPortStatus {
    /// No tunnel of the host forwards to the port and only this instance
    /// lists it.
    New,
    /// A tunnel of the host already forwards to the port.
    AlreadyTunneled {
        /// The tunnel that does.
        tunnel_id: TunnelId,
    },
    /// Several instances list the port (a forced port), so it does not
    /// identify one of them.
    Ambiguous,
}

/// A port of an instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevslotPort {
    /// The environment variable that names the port.
    pub var: String,
    /// The effective port, between 1 and 65535.
    pub port: u16,
    /// What the port serves: devslot says `web` or `service`.
    pub kind: String,
    /// The URL of a web port.
    pub url: Option<String>,
    /// The `<label>.localhost` name of the URL, when it is a valid open host
    /// name.
    pub host_name: Option<String>,
    /// The compose service of a service port.
    pub service: Option<String>,
    /// The port inside the service container.
    pub target: Option<u16>,
    /// Whether the port is forced by the instance: it may be shared with
    /// another instance.
    pub is_forced: bool,
    /// Whether the port can be tunneled.
    pub status: DevslotPortStatus,
    /// Whether something else already holds this port on the loopback of
    /// this machine, so a tunnel cannot listen on it.
    pub is_local_port_busy: bool,
    /// A free local port to listen on instead, when the port is busy.
    pub suggested_local_port: Option<u16>,
}

/// A devslot instance of a host: a checkout with its own ports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevslotInstance {
    /// The unique name of the instance.
    pub name: String,
    /// The name of the repository; not unique, never a key.
    pub project: Option<String>,
    /// The tag of the project, see [`devslot_project_tag`](crate::devslot_project_tag).
    pub project_tag: Option<String>,
    /// The path of the checkout on the host.
    pub path: String,
    /// Whether the checkout is a linked git worktree.
    pub is_worktree: Option<bool>,
    /// The slot the instance holds.
    pub slot: u32,
    /// The URL of the application.
    pub app_url: Option<String>,
    /// Whether the compose services run; `None` when devslot could not tell.
    pub services_running: Option<bool>,
    /// The ports of the instance.
    pub ports: Vec<DevslotPort>,
}

/// Something the listing had that was left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevslotWarning {
    /// A port was skipped because its number is not between 1 and 65535.
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
