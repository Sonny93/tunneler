use std::ffi::OsString;
use std::sync::Arc;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use tokio::sync::mpsc;

use super::atomic_file::{create_private_directories, parent_directory};
use super::error::StoreError;
use super::file::{SharedStore, Store};
use super::format::LoadOutcome;

/// Events closer together than this are folded into one reload.
const RELOAD_DEBOUNCE: Duration = Duration::from_millis(200);

/// What happened to the store file behind the application's back.
#[derive(Debug)]
pub enum StoreEvent {
    /// The file changed and is valid; it is now the store's document. The
    /// outcome says whether the file still lacks ids the store generated.
    Reloaded(LoadOutcome),
    /// The file changed but cannot be used (or could not be read); the last
    /// good document stays the store's state.
    Invalid(StoreError),
}

/// Delivers [`StoreEvent`]s while it is alive; dropping it stops the watch.
pub struct StoreWatcher {
    events: mpsc::UnboundedReceiver<StoreEvent>,
    _debouncer: Debouncer<RecommendedWatcher>,
}

impl StoreWatcher {
    /// Waits for the next event; `None` once the watch has stopped.
    pub async fn next_event(&mut self) -> Option<StoreEvent> {
        self.events.recv().await
    }
}

impl Store {
    /// Watches the store file for external changes.
    ///
    /// The parent directory is watched rather than the file, because editors
    /// and [`save`](Store::save) replace the file with a rename. Bursts of
    /// events are debounced, the file is re-read and validated, and writes
    /// made through this store are recognized by their content and ignored.
    /// A deleted file is ignored.
    pub fn watch(&self) -> Result<StoreWatcher, StoreError> {
        let shared = Arc::clone(self.shared());
        let directory = parent_directory(shared.path()).to_path_buf();
        create_private_directories(&directory)?;
        let (sender, events) = mpsc::unbounded_channel();
        let handler = reload_handler(shared, sender);
        let watch_error = |source| StoreError::Watch {
            path: directory.clone(),
            source,
        };
        let mut debouncer = new_debouncer(RELOAD_DEBOUNCE, handler).map_err(watch_error)?;
        debouncer
            .watcher()
            .watch(&directory, RecursiveMode::NonRecursive)
            .map_err(watch_error)?;
        Ok(StoreWatcher {
            events,
            _debouncer: debouncer,
        })
    }
}

fn reload_handler(
    shared: Arc<SharedStore>,
    sender: mpsc::UnboundedSender<StoreEvent>,
) -> impl FnMut(DebounceEventResult) + Send + 'static {
    let file_name: Option<OsString> = shared.path().file_name().map(OsString::from);
    move |result| {
        let event = match result {
            Ok(events) => {
                let touches_store_file = events
                    .iter()
                    .any(|event| event.path.file_name() == file_name.as_deref());
                if !touches_store_file {
                    return;
                }
                shared.reload_if_changed()
            }
            Err(source) => Some(StoreEvent::Invalid(StoreError::Watch {
                path: shared.path().to_path_buf(),
                source,
            })),
        };
        if let Some(event) = event {
            let _ = sender.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::model::{StoreDocument, TunnelKind};
    use crate::store::test_support::named_tunnel;
    use std::fs;
    use std::path::PathBuf;
    use tokio::time::timeout;

    const EVENT_TIMEOUT: Duration = Duration::from_secs(5);
    const QUIET_PERIOD: Duration = Duration::from_millis(1000);
    const VALID_FILE: &str = "[[tunnels]]\nid = \"01JA0000000000000000000000\"\nname = \"Web\"\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 8080\n";
    const EDITED_FILE: &str = "[[tunnels]]\nid = \"01JA0000000000000000000000\"\nname = \"Web\"\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 9090\n";

    struct Fixture {
        _directory: tempfile::TempDir,
        path: PathBuf,
        store: Store,
        watcher: StoreWatcher,
    }

    fn fixture() -> Fixture {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tunnels.toml");
        fs::write(&path, VALID_FILE).unwrap();
        let (store, _) = Store::open(path.clone()).unwrap();
        let watcher = store.watch().unwrap();
        Fixture {
            _directory: directory,
            path,
            store,
            watcher,
        }
    }

    async fn next_event(watcher: &mut StoreWatcher) -> Option<StoreEvent> {
        timeout(EVENT_TIMEOUT, watcher.next_event())
            .await
            .ok()
            .flatten()
    }

    #[tokio::test]
    async fn should_emit_reloaded_when_the_file_is_edited_externally() {
        let mut fixture = fixture();

        fs::write(&fixture.path, EDITED_FILE).unwrap();

        let event = next_event(&mut fixture.watcher).await;
        assert!(
            matches!(&event, Some(StoreEvent::Reloaded(outcome)) if outcome.document.tunnels[0].listen_port == 9090),
            "{event:?}"
        );
    }

    #[tokio::test]
    async fn should_update_the_store_document_when_the_file_is_edited_externally() {
        let mut fixture = fixture();

        fs::write(&fixture.path, EDITED_FILE).unwrap();
        next_event(&mut fixture.watcher).await;

        assert_eq!(fixture.store.document().tunnels[0].listen_port, 9090);
    }

    #[tokio::test]
    async fn should_emit_invalid_and_keep_the_last_good_document_when_the_edit_is_broken() {
        let mut fixture = fixture();
        let last_good = fixture.store.document();

        fs::write(&fixture.path, "[[tunnels]\n").unwrap();

        let event = next_event(&mut fixture.watcher).await;
        assert!(matches!(event, Some(StoreEvent::Invalid(_))), "{event:?}");
        assert_eq!(fixture.store.document(), last_good);
    }

    #[tokio::test]
    async fn should_emit_invalid_when_the_edit_breaks_a_validation_rule() {
        let mut fixture = fixture();

        fs::write(&fixture.path, VALID_FILE.replace("8080", "0")).unwrap();

        let event = next_event(&mut fixture.watcher).await;
        assert!(
            matches!(event, Some(StoreEvent::Invalid(StoreError::Validation(_)))),
            "{event:?}"
        );
    }

    #[tokio::test]
    async fn should_emit_nothing_when_the_store_saves_its_own_document() {
        let mut fixture = fixture();
        let document = StoreDocument {
            tunnels: vec![named_tunnel(TunnelKind::Dynamic, "Socks", 1080)],
            ..StoreDocument::default()
        };

        fixture.store.save(document).unwrap();

        let event = timeout(QUIET_PERIOD, fixture.watcher.next_event()).await;
        assert!(event.is_err(), "{event:?}");
    }

    #[tokio::test]
    async fn should_detect_an_editor_replacing_the_file_through_a_rename() {
        let mut fixture = fixture();
        let temporary = fixture.path.with_file_name("tunnels.toml.swp");

        fs::write(&temporary, EDITED_FILE).unwrap();
        fs::rename(&temporary, &fixture.path).unwrap();

        let event = next_event(&mut fixture.watcher).await;
        assert!(matches!(event, Some(StoreEvent::Reloaded(_))), "{event:?}");
    }

    #[tokio::test]
    async fn should_ignore_changes_to_other_files_in_the_directory() {
        let mut fixture = fixture();

        fs::write(fixture.path.with_file_name("other.txt"), "noise").unwrap();

        let event = timeout(QUIET_PERIOD, fixture.watcher.next_event()).await;
        assert!(event.is_err(), "{event:?}");
    }

    #[tokio::test]
    async fn should_report_the_edit_after_a_save_when_the_content_really_changed() {
        let mut fixture = fixture();
        fixture.store.save(fixture.store.document()).unwrap();

        fs::write(&fixture.path, EDITED_FILE).unwrap();

        let event = next_event(&mut fixture.watcher).await;
        assert!(matches!(event, Some(StoreEvent::Reloaded(_))), "{event:?}");
    }

    #[tokio::test]
    async fn should_stop_the_event_stream_when_the_watcher_is_dropped() {
        let Fixture { mut watcher, .. } = fixture();
        let (_, placeholder) = mpsc::unbounded_channel();
        let mut events = std::mem::replace(&mut watcher.events, placeholder);
        drop(watcher);

        let closed = timeout(EVENT_TIMEOUT, events.recv()).await;

        assert!(matches!(closed, Ok(None)), "{closed:?}");
    }
}
