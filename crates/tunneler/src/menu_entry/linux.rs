//! XDG desktops: the entry lives in `applications/` and the icons in
//! `icons/hicolor/` of the user data directory (`$XDG_DATA_HOME`, usually
//! `~/.local/share`), per the Desktop Entry and Icon Theme specifications.

use std::path::PathBuf;
use std::sync::Arc;

use directories::BaseDirs;

use super::files::MenuEntryFiles;
use super::programs::{expected_ui_program, find_ui_program, is_package_managed};
use super::{MenuEntry, MenuEntryError, MenuEntryReport};
use crate::daemon::login_launch::repoint_owned_entry;

struct SystemMenuEntry;

impl MenuEntry for SystemMenuEntry {
    fn is_installed(&self) -> bool {
        system_files().is_ok_and(|files| files.is_installed())
    }

    fn remove(&self) -> Result<(), MenuEntryError> {
        system_files()?.uninstall().map(drop)
    }
}

/// The menu entry of the machine.
pub(crate) fn system_menu_entry() -> Arc<dyn MenuEntry> {
    Arc::new(SystemMenuEntry)
}

/// Writes the entry and the icons for the `tunneler-ui` next to the running
/// executable, and points an existing launch-at-login entry that Tunneler
/// wrote at that executable.
///
/// # Errors
///
/// Fails, writing nothing, when the executable is managed by a package, when
/// `tunneler-ui` is not next to it, or when a `.desktop` file that Tunneler
/// did not write is in the way.
pub(crate) fn install_menu_entry() -> Result<MenuEntryReport, MenuEntryError> {
    let executable = current_executable()?;
    if is_package_managed(&executable) {
        return Err(MenuEntryError::PackageManaged { executable });
    }
    let ui_program =
        find_ui_program(&executable).ok_or_else(|| MenuEntryError::UiProgramMissing {
            expected: expected_ui_program(&executable),
        })?;
    let files = system_files()?.install(&ui_program)?;
    let repointed_autostart = repoint_owned_entry(&executable)?;
    Ok(MenuEntryReport {
        files,
        repointed_autostart,
    })
}

/// Removes the entry and the icons.
///
/// # Errors
///
/// Fails, removing nothing, when the `.desktop` file is not Tunneler's.
pub(crate) fn uninstall_menu_entry() -> Result<MenuEntryReport, MenuEntryError> {
    Ok(MenuEntryReport {
        files: system_files()?.uninstall()?,
        repointed_autostart: None,
    })
}

/// Points an installed entry at the `tunneler-ui` next to the running
/// executable. A failure is logged and never stops the daemon; nothing is done
/// for a package-managed executable or without a `tunneler-ui` next to it.
pub(crate) fn refresh_menu_entry_at_start() {
    if let Err(error) = refresh_menu_entry() {
        tracing::warn!(%error, "cannot refresh the application menu entry");
    }
}

fn refresh_menu_entry() -> Result<(), MenuEntryError> {
    let files = system_files()?;
    if !files.is_installed() {
        return Ok(());
    }
    let executable = current_executable()?;
    if is_package_managed(&executable) {
        return Ok(());
    }
    match find_ui_program(&executable) {
        Some(ui_program) => files.refresh(&ui_program),
        None => Ok(()),
    }
}

fn current_executable() -> Result<PathBuf, MenuEntryError> {
    std::env::current_exe().map_err(MenuEntryError::Executable)
}

fn system_files() -> Result<MenuEntryFiles, MenuEntryError> {
    let base_directories = BaseDirs::new().ok_or(MenuEntryError::NoDataDirectory)?;
    Ok(MenuEntryFiles::in_data_directory(
        base_directories.data_dir(),
    ))
}
