//! The persistent store: tunnels, settings, host overrides and X11 launchers
//! kept in a hand-editable `tunnels.toml`.
//!
//! [`Store::open`] reads and validates the file; [`Store::save`] validates a
//! [`StoreDocument`] and writes it atomically, updating the existing TOML in
//! place so comments and formatting written by hand survive; [`Store::watch`]
//! reports external edits.
//!
//! The [`Store`] is synchronous: its file operations are short and blocking.
//! Callers in async code run them through `spawn_blocking`.

mod atomic_file;
mod document_update;
mod error;
mod file;
mod format;
mod migration;
mod model;
mod state;
#[cfg(test)]
pub(crate) mod test_support;
mod validation;
mod watch;

pub(crate) use document_update::export_tunnel_table;
pub use error::{SourceLocation, StoreError};
pub use file::{Store, default_store_path};
pub use format::LoadOutcome;
pub(crate) use format::{StoredTunnel, locate};
pub(crate) use migration::{CURRENT_VERSION, migrate};
pub use model::{
    BROWSER_OPENER_ID, DevslotStale, HostOverrides, InvalidTunnelId, Language, MAX_STANDBY_VOLUME,
    MIN_STANDBY_VOLUME, OpenDefault, OpenHost, OpenOptions, OpenScheme, Opener, Settings,
    StandbyNotify, StandbySound, StoreDocument, TunnelDefinition, TunnelId, TunnelKind,
    X11Launcher, X11ServerStart,
};
pub use state::{AppState, FileStateStore, StateStore, default_state_path};
#[cfg(test)]
pub(crate) use validation::validate;
pub use validation::{
    DevslotPathProblem, HostNameProblem, OpenPathProblem, OpenerIdProblem, SshHostProblem,
    TagProblem, TunnelReference, ValidationError,
};
pub(crate) use validation::{
    ListenEndpoint, effective_target, find_host_name_problem, find_open_path_problem,
    find_ssh_host_problem, find_tag_problem, validate_tunnel_definition, validate_tunnel_host,
};
pub use watch::{StoreEvent, StoreWatcher};
