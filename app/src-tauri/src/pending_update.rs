//! The newer version found by the last check, kept for its installation and
//! its release page: the front end never names a version or a URL itself.

use std::sync::{Mutex, PoisonError};

use tauri_plugin_updater::Update;

/// The update the last successful check found, if any.
#[derive(Default)]
pub struct PendingUpdate {
    update: Mutex<Option<Update>>,
}

impl PendingUpdate {
    pub fn replace(&self, update: Option<Update>) {
        *self.update.lock().unwrap_or_else(PoisonError::into_inner) = update;
    }

    pub fn current(&self) -> Option<Update> {
        self.update
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}
