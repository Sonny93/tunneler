//! Systems without XDG desktops: there is no menu entry to manage.

use std::sync::Arc;

use super::{MenuEntry, MenuEntryError, MenuEntryReport};

struct UnsupportedMenuEntry;

impl MenuEntry for UnsupportedMenuEntry {
    fn is_installed(&self) -> bool {
        false
    }

    fn remove(&self) -> Result<(), MenuEntryError> {
        Ok(())
    }
}

/// The menu entry of the machine: none.
pub(crate) fn system_menu_entry() -> Arc<dyn MenuEntry> {
    Arc::new(UnsupportedMenuEntry)
}

/// Always fails: only Linux has this entry.
pub(crate) fn install_menu_entry() -> Result<MenuEntryReport, MenuEntryError> {
    Err(MenuEntryError::Unsupported)
}

/// Always fails: only Linux has this entry.
pub(crate) fn uninstall_menu_entry() -> Result<MenuEntryReport, MenuEntryError> {
    Err(MenuEntryError::Unsupported)
}

/// Does nothing: there is no entry to refresh.
pub(crate) fn refresh_menu_entry_at_start() {}
