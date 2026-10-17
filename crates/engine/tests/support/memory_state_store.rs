use std::sync::Mutex;

use engine::{AppState, StateStore, StoreError};

/// A state store that keeps the state in memory and counts the writes.
#[derive(Default)]
pub struct MemoryStateStore {
    state: Mutex<AppState>,
    save_count: Mutex<usize>,
}

impl MemoryStateStore {
    pub fn holding(state: AppState) -> Self {
        Self {
            state: Mutex::new(state),
            save_count: Mutex::new(0),
        }
    }

    pub fn state(&self) -> AppState {
        self.state.lock().unwrap().clone()
    }

    pub fn save_count(&self) -> usize {
        *self.save_count.lock().unwrap()
    }
}

impl StateStore for MemoryStateStore {
    fn load(&self) -> Result<AppState, StoreError> {
        Ok(self.state())
    }

    fn save(&self, state: &AppState) -> Result<(), StoreError> {
        *self.state.lock().unwrap() = state.clone();
        *self.save_count.lock().unwrap() += 1;
        Ok(())
    }
}
