//! A launcher that starts nothing, for the unit tests.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{UiLaunchError, UiLauncher};

/// Counts the launches instead of starting a program.
pub(crate) struct FakeLauncher {
    launches: AtomicUsize,
    is_failing: bool,
}

impl FakeLauncher {
    pub(crate) fn succeeding() -> Arc<Self> {
        Arc::new(Self {
            launches: AtomicUsize::new(0),
            is_failing: false,
        })
    }

    pub(crate) fn failing() -> Arc<Self> {
        Arc::new(Self {
            launches: AtomicUsize::new(0),
            is_failing: true,
        })
    }

    pub(crate) fn launch_count(&self) -> usize {
        self.launches.load(Ordering::SeqCst)
    }
}

impl UiLauncher for FakeLauncher {
    fn launch(&self) -> Result<(), UiLaunchError> {
        self.launches.fetch_add(1, Ordering::SeqCst);
        if self.is_failing {
            return Err(UiLaunchError::Spawn {
                program: PathBuf::from("tunneler-ui"),
                source: io::Error::from(io::ErrorKind::NotFound),
            });
        }
        Ok(())
    }
}
