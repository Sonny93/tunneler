use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::Ordering;

use tracing::warn;

use crate::blocking::run_blocking;
use crate::runtime::engine::EngineInner;
use crate::runtime::events::EngineEvent;
use crate::store::{LoadOutcome, StoreEvent, StoreWatcher};

/// Applies the external edits of the store file until the engine is gone.
pub(crate) async fn run_reload_loop(engine: Weak<EngineInner>, mut watcher: StoreWatcher) {
    while let Some(event) = watcher.next_event().await {
        let Some(inner) = engine.upgrade() else {
            return;
        };
        inner.handle_store_event(event).await;
    }
}

impl EngineInner {
    pub(crate) async fn handle_store_event(&self, event: StoreEvent) {
        match event {
            StoreEvent::Invalid(error) => {
                warn!(%error, "the tunnels file changed into something unusable");
                self.bus.publish(EngineEvent::ConfigInvalid {
                    message: error.to_string(),
                });
            }
            StoreEvent::Reloaded(outcome) => self.apply_reload(*outcome).await,
        }
    }

    /// Applies what the store holds when the lock is taken, which is the
    /// outcome that was queued or a newer state: a change that an apply of
    /// the engine already took in must not be undone by a reload queued
    /// before it.
    async fn apply_reload(&self, outcome: LoadOutcome) {
        let _serialized = self.mutation.lock().await;
        let outcome = LoadOutcome {
            document: self.store.document(),
            ..outcome
        };
        self.apply_reload_locked(outcome).await;
    }

    /// Applies an outcome read from the file; the caller holds `mutation`.
    pub(crate) async fn apply_reload_locked(&self, outcome: LoadOutcome) {
        if self.is_shut_down.load(Ordering::SeqCst) {
            return;
        }
        self.settings
            .send_if_modified(|settings| replace_if_different(settings, &outcome));
        let stopping = self.catalog().apply(&outcome.document.tunnels);
        for task in stopping {
            let _ = task.await;
        }
        self.bus.publish(EngineEvent::ConfigReloaded);
        if self.store.has_pending_normalization() {
            self.save_normalized(outcome).await;
        }
    }

    async fn save_normalized(&self, outcome: LoadOutcome) {
        let store = Arc::clone(&self.store);
        let document = outcome.document;
        if let Err(error) = run_blocking(move || store.save(document)).await {
            warn!(%error, "cannot save the generated tunnel ids back to the tunnels file");
        }
    }
}

fn replace_if_different(settings: &mut crate::store::Settings, outcome: &LoadOutcome) -> bool {
    if *settings == outcome.document.settings {
        return false;
    }
    settings.clone_from(&outcome.document.settings);
    true
}
