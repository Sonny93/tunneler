use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

use engine::ProgramLocator;

/// A locator that finds the programs the test installed, and can change its
/// answers while the engine runs. It can also hang, like a stale network
/// mount in the `PATH`, until the test releases it.
#[derive(Default)]
pub struct FakeProgramLocator {
    installed: Mutex<BTreeSet<String>>,
    is_hanging: Mutex<bool>,
    hang_ended: Condvar,
    lookups: AtomicUsize,
}

impl FakeProgramLocator {
    pub fn install(&self, program: &str) {
        self.installed().insert(program.to_owned());
    }

    pub fn uninstall(&self, program: &str) {
        self.installed().remove(program);
    }

    /// Makes every lookup wait until [`release`](Self::release).
    pub fn hang(&self) {
        *self.hanging() = true;
    }

    pub fn release(&self) {
        *self.hanging() = false;
        self.hang_ended.notify_all();
    }

    /// How many programs were looked for so far.
    pub fn lookup_count(&self) -> usize {
        self.lookups.load(Ordering::SeqCst)
    }

    fn installed(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.installed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn hanging(&self) -> MutexGuard<'_, bool> {
        self.is_hanging
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl ProgramLocator for FakeProgramLocator {
    fn is_available(&self, program: &str) -> bool {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        let mut is_hanging = self.hanging();
        while *is_hanging {
            is_hanging = self
                .hang_ended
                .wait(is_hanging)
                .unwrap_or_else(PoisonError::into_inner);
        }
        drop(is_hanging);
        self.installed().contains(program)
    }
}
