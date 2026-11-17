//! The application menu entry of the Linux archive: a `.desktop` file and its
//! icons under the user's XDG data directory, so that the desktop lists
//! Tunneler like the entry the `.deb` ships.
//!
//! `tunneler desktop-entry install` writes it, `uninstall` and `purge` remove
//! it, and the daemon points an existing entry at the current `tunneler-ui`
//! when it starts: a per-version install folder changes the path of the
//! program at every update.

#[cfg(all(unix, not(target_os = "macos")))]
mod entry_text;
#[cfg(all(unix, not(target_os = "macos")))]
mod files;
#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
mod programs;
#[cfg(not(all(unix, not(target_os = "macos"))))]
mod unsupported;

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) use linux::{
    install_menu_entry, refresh_menu_entry_at_start, system_menu_entry, uninstall_menu_entry,
};
#[cfg(not(all(unix, not(target_os = "macos"))))]
pub(crate) use unsupported::{
    install_menu_entry, refresh_menu_entry_at_start, system_menu_entry, uninstall_menu_entry,
};

use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::daemon::login_launch::LoginLaunchError;

/// Why the menu entry could not be installed, removed or refreshed.
#[derive(Debug, Error)]
#[allow(
    dead_code,
    reason = "each platform builds its own subset of the variants"
)]
pub(crate) enum MenuEntryError {
    #[error("the application menu entry is only available on Linux")]
    Unsupported,
    #[error("the platform has no user data directory")]
    NoDataDirectory,
    #[error("cannot find the running executable: {0}")]
    Executable(io::Error),
    #[error("{} not found: it must sit next to the tunneler program", expected.display())]
    UiProgramMissing { expected: PathBuf },
    #[error("{} contains a control character, such as a line break, which a menu entry cannot hold", path.display())]
    UnsafePath { path: PathBuf },
    #[error("{} is managed by a package, which already ships the menu entry", executable.display())]
    PackageManaged { executable: PathBuf },
    #[error("{} exists and was not written by Tunneler, leaving it alone", path.display())]
    ForeignEntry { path: PathBuf },
    #[error("cannot access {}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
    #[error("cannot update the launch-at-login entry: {0}")]
    Autostart(#[from] LoginLaunchError),
}

/// What an install or an uninstall did to the files.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct MenuEntryReport {
    /// The files written or removed, the entry first.
    pub(crate) files: Vec<PathBuf>,
    /// The launch-at-login entry that was pointed at the running executable.
    pub(crate) repointed_autostart: Option<PathBuf>,
}

/// The menu entry of the machine, as far as `purge` is concerned.
pub(crate) trait MenuEntry: Send + Sync {
    /// Whether an entry that Tunneler wrote is installed.
    fn is_installed(&self) -> bool;

    /// Removes the entry and its icons. Removing what is gone succeeds.
    fn remove(&self) -> Result<(), MenuEntryError>;
}
