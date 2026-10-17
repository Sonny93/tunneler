//! Platforms without a login entry (macOS): launch at login is not supported,
//! which is reported once.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{LoginLaunch, LoginLaunchError};

static HAS_WARNED: AtomicBool = AtomicBool::new(false);

pub(super) struct SystemLoginLaunch;

impl LoginLaunch for SystemLoginLaunch {
    fn apply(&self, is_enabled: bool) -> Result<(), LoginLaunchError> {
        if is_enabled && !HAS_WARNED.swap(true, Ordering::SeqCst) {
            tracing::warn!("launch at login is not supported on this platform");
        }
        Ok(())
    }
}
