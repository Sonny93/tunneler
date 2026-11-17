use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard, PoisonError};

use engine::ProgramLocator;

/// A locator that finds the programs the test installed, and can change its
/// answers while the engine runs.
#[derive(Default)]
pub struct FakeProgramLocator {
    installed: Mutex<BTreeSet<String>>,
}

impl FakeProgramLocator {
    pub fn install(&self, program: &str) {
        self.installed().insert(program.to_owned());
    }

    pub fn uninstall(&self, program: &str) {
        self.installed().remove(program);
    }

    fn installed(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.installed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl ProgramLocator for FakeProgramLocator {
    fn is_available(&self, program: &str) -> bool {
        self.installed().contains(program)
    }
}
