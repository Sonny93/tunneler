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

pub use error::{SourceLocation, StoreError};
pub use file::{Store, default_store_path};
pub use format::LoadOutcome;
pub use model::{
    BROWSER_OPENER_ID, HostOverrides, InvalidTunnelId, OpenHost, OpenOptions, OpenScheme, Opener,
    Settings, StoreDocument, TunnelDefinition, TunnelId, TunnelKind, X11Launcher,
};
pub use state::{AppState, FileStateStore, StateStore, default_state_path};
pub use validation::{
    OpenPathProblem, OpenerIdProblem, TagProblem, TunnelReference, ValidationError,
};
pub(crate) use validation::{find_open_path_problem, find_tag_problem, validate_tunnel_definition};
pub use watch::{StoreEvent, StoreWatcher};
