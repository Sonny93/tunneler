//! Finding the ports of the devslot instances of a host.
//!
//! The host runs `ds ls --json`; this module builds that command, reads and
//! cleans its output, and compares the listing with the tunnels of the host
//! to tell which listed ports are already tunneled and which devslot tunnels
//! no longer match. Running the command and removing tunnels belong to the
//! engine runtime.

mod analysis;
mod command;
mod discovery;
mod host_name;
mod listing;
mod local_ports;
mod origin;
mod parse;
mod project_tag;
mod resync_plan;
mod sanitize;
mod up_outcome;
mod up_plan;

pub use analysis::{DevslotStaleAction, DevslotStaleReason, DevslotStaleTunnel};
pub(crate) use analysis::{KnownTunnel, assign_port_statuses, find_stale_tunnels};
pub(crate) use command::{DEFAULT_PROGRAM, build_listing_command};
pub use discovery::{DevslotDiscovery, DevslotError, DevslotResync, DevslotResyncTunnel};
pub use listing::{DevslotInstance, DevslotPort, DevslotPortStatus, DevslotWarning};
pub(crate) use local_ports::{BindProbe, assign_local_ports};
pub use origin::DevslotOrigin;
pub(crate) use parse::parse_listing;
pub use parse::{MAX_DEVSLOT_INSTANCES, MAX_DEVSLOT_PORTS, MAX_DEVSLOT_PORTS_PER_INSTANCE};
pub use project_tag::devslot_project_tag;
pub(crate) use resync_plan::plan_resync;
pub(crate) use sanitize::{
    MAX_PATH_BYTES, MAX_TEXT_BYTES, contains_forbidden_character, sanitize_stderr, sanitize_text,
};
pub use up_outcome::{
    DevslotFailedPort, DevslotRemovedRoute, DevslotSkipReason, DevslotSkippedPort, DevslotUp,
    DevslotUpPort,
};
pub(crate) use up_plan::{
    DevslotUpPlan, PlannedTunnel, UpPlanInput, UpTunnelKind, plan_devslot_up,
};
