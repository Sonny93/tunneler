use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::runtime::Handle;
use tracing::warn;

use crate::blocking::run_blocking;
use crate::store::{AppState, StateStore, StoreError};

/// Keeps the [`AppState`] in memory and writes every change to its store off
/// the async executor.
///
/// A failed write is logged and never reaches the caller: losing the state
/// must not fail a user command. Writes always carry the latest state, so a
/// late write cannot bring an older one back.
pub(crate) struct StateTracker {
    store: Arc<dyn StateStore>,
    runtime: Handle,
    current: Mutex<AppState>,
    last_written: Mutex<Option<AppState>>,
}

impl StateTracker {
    /// Reads the state from `store`. A state that cannot be read is logged
    /// and replaced by an empty one, which the next change overwrites.
    /// Must be called inside a Tokio runtime.
    pub(crate) fn load(store: Arc<dyn StateStore>) -> Arc<Self> {
        let loaded = store.load();
        let last_written = loaded.as_ref().ok().cloned();
        if let Err(error) = &loaded {
            warn!(%error, "cannot read the saved state, starting with an empty one");
        }
        Arc::new(Self {
            store,
            runtime: Handle::current(),
            current: Mutex::new(loaded.unwrap_or_default()),
            last_written: Mutex::new(last_written),
        })
    }

    /// A tracker that keeps its state in memory only.
    pub(crate) fn detached() -> Arc<Self> {
        Self::load(Arc::new(DetachedStateStore))
    }

    pub(crate) fn snapshot(&self) -> AppState {
        lock(&self.current).clone()
    }

    /// Applies `change` and writes the state when it differs afterwards.
    pub(crate) fn update(self: &Arc<Self>, change: impl FnOnce(&mut AppState)) {
        let is_changed = {
            let mut current = lock(&self.current);
            let before = current.clone();
            change(&mut current);
            *current != before
        };
        if is_changed {
            let tracker = Arc::clone(self);
            drop(self.runtime.spawn_blocking(move || tracker.write_latest()));
        }
    }

    /// Waits until the latest state is written.
    pub(crate) async fn flush(self: &Arc<Self>) {
        let tracker = Arc::clone(self);
        run_blocking(move || tracker.write_latest()).await;
    }

    fn write_latest(&self) {
        let mut last_written = lock(&self.last_written);
        let latest = self.snapshot();
        if last_written.as_ref() == Some(&latest) {
            return;
        }
        match self.store.save(&latest) {
            Ok(()) => *last_written = Some(latest),
            Err(error) => warn!(%error, "cannot save the state"),
        }
    }
}

fn lock<Value>(mutex: &Mutex<Value>) -> MutexGuard<'_, Value> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

struct DetachedStateStore;

impl StateStore for DetachedStateStore {
    fn load(&self) -> Result<AppState, StoreError> {
        Ok(AppState::default())
    }

    fn save(&self, _state: &AppState) -> Result<(), StoreError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use pretty_assertions::assert_eq;

    use super::*;
    use crate::store::TunnelId;

    #[derive(Default)]
    struct RecordingStateStore {
        initial: Mutex<Option<AppState>>,
        saved: Mutex<Vec<AppState>>,
        save_attempts: AtomicUsize,
        is_failing: bool,
    }

    impl StateStore for RecordingStateStore {
        fn load(&self) -> Result<AppState, StoreError> {
            Ok(lock(&self.initial).clone().unwrap_or_default())
        }

        fn save(&self, state: &AppState) -> Result<(), StoreError> {
            self.save_attempts.fetch_add(1, Ordering::SeqCst);
            if self.is_failing {
                return Err(StoreError::ConfigDirectoryUnavailable);
            }
            lock(&self.saved).push(state.clone());
            Ok(())
        }
    }

    fn running_change(id: TunnelId) -> impl FnOnce(&mut AppState) {
        move |state| {
            state.running.insert(id);
        }
    }

    #[tokio::test]
    async fn should_write_the_state_when_it_changes() {
        let store = Arc::new(RecordingStateStore::default());
        let tracker = StateTracker::load(store.clone());
        let id = TunnelId::generate();

        tracker.update(running_change(id));
        tracker.flush().await;

        let saved = lock(&store.saved).last().cloned().unwrap();
        assert_eq!(saved.running.into_iter().collect::<Vec<_>>(), vec![id]);
    }

    #[tokio::test]
    async fn should_not_write_when_the_change_leaves_the_state_as_it_was() {
        let store = Arc::new(RecordingStateStore::default());
        let tracker = StateTracker::load(store.clone());

        tracker.update(|_state| {});
        tracker.flush().await;

        assert_eq!(store.save_attempts.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_start_from_the_stored_state_when_the_store_has_one() {
        let id = TunnelId::generate();
        let store = RecordingStateStore::default();
        *lock(&store.initial) = Some(AppState {
            running: [id].into(),
            ..AppState::default()
        });

        let tracker = StateTracker::load(Arc::new(store));

        assert_eq!(
            tracker.snapshot().running.into_iter().collect::<Vec<_>>(),
            vec![id]
        );
    }

    #[tokio::test]
    async fn should_keep_the_change_in_memory_when_the_write_fails() {
        let store = Arc::new(RecordingStateStore {
            is_failing: true,
            ..RecordingStateStore::default()
        });
        let tracker = StateTracker::load(store);
        let id = TunnelId::generate();

        tracker.update(running_change(id));
        tracker.flush().await;

        assert!(tracker.snapshot().running.contains(&id));
    }

    #[tokio::test]
    async fn should_start_empty_when_the_stored_state_cannot_be_read() {
        struct UnreadableStore;
        impl StateStore for UnreadableStore {
            fn load(&self) -> Result<AppState, StoreError> {
                Err(StoreError::ConfigDirectoryUnavailable)
            }
            fn save(&self, _state: &AppState) -> Result<(), StoreError> {
                Ok(())
            }
        }

        let tracker = StateTracker::load(Arc::new(UnreadableStore));

        assert_eq!(tracker.snapshot(), AppState::default());
    }
}
