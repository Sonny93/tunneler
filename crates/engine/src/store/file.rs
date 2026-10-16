use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use directories::BaseDirs;

use super::atomic_file::{read_if_present, replace_contents};
use super::error::StoreError;
use super::format::{LoadOutcome, parse_document, render_document};
use super::model::StoreDocument;
use super::validation::validate;
use super::watch::StoreEvent;

const APP_DIRECTORY_NAME: &str = "tunneler";
const STORE_FILE_NAME: &str = "tunnels.toml";

/// The default location of the store file.
///
/// This is `tunneler/tunnels.toml` inside the user configuration directory
/// reported by [`directories::BaseDirs::config_dir`]:
///
/// - Linux: `$XDG_CONFIG_HOME/tunneler/tunnels.toml`, usually
///   `~/.config/tunneler/tunnels.toml`
/// - Windows: `%APPDATA%\tunneler\tunnels.toml` (the roaming `AppData`
///   folder)
/// - macOS: `~/Library/Application Support/tunneler/tunnels.toml`
pub fn default_store_path() -> Result<PathBuf, StoreError> {
    let base_directories = BaseDirs::new().ok_or(StoreError::ConfigDirectoryUnavailable)?;
    Ok(base_directories
        .config_dir()
        .join(APP_DIRECTORY_NAME)
        .join(STORE_FILE_NAME))
}

/// A hash of the bytes of the store file, used to recognize content the store
/// itself wrote or already read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentFingerprint(u64);

impl ContentFingerprint {
    fn of(content: &[u8]) -> Self {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Self(hasher.finish())
    }
}

struct StoreState {
    document: StoreDocument,
    fingerprint: Option<ContentFingerprint>,
    is_normalization_pending: bool,
}

/// The state shared between a [`Store`] and its watcher.
pub(crate) struct SharedStore {
    path: PathBuf,
    state: Mutex<StoreState>,
}

impl SharedStore {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    fn lock(&self) -> MutexGuard<'_, StoreState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Re-reads the file when its content differs from what the store last
    /// wrote or read. A valid file replaces the store's document; an invalid
    /// one is reported and the last good document stays. A missing file is
    /// ignored.
    pub(crate) fn reload_if_changed(&self) -> Option<StoreEvent> {
        let mut state = self.lock();
        let content = match read_if_present(&self.path) {
            Ok(Some(content)) => content,
            Ok(None) => return None,
            Err(error) => return Some(StoreEvent::Invalid(error)),
        };
        let fingerprint = Some(ContentFingerprint::of(&content));
        if fingerprint == state.fingerprint {
            return None;
        }
        state.fingerprint = fingerprint;
        match parse_content(&self.path, &content) {
            Ok(outcome) => {
                state.document = outcome.document.clone();
                state.is_normalization_pending = outcome.was_normalized;
                Some(StoreEvent::Reloaded(outcome))
            }
            Err(error) => Some(StoreEvent::Invalid(error)),
        }
    }
}

/// The persistent store backed by a TOML file.
///
/// All operations are synchronous and block on file IO; they are short, and
/// the watcher thread needs them synchronous anyway. Async callers run them
/// through `spawn_blocking`.
pub struct Store {
    shared: Arc<SharedStore>,
}

impl Store {
    /// Opens the store at `path`, reading and validating the file.
    ///
    /// A missing file yields the default document and is not created until
    /// the first [`save`](Self::save). The outcome says whether the file
    /// needs rewriting ([`LoadOutcome::was_normalized`]); that decision is
    /// left to the caller.
    pub fn open(path: PathBuf) -> Result<(Self, LoadOutcome), StoreError> {
        let content = read_if_present(&path)?;
        let outcome = match &content {
            Some(bytes) => parse_content(&path, bytes)?,
            None => LoadOutcome {
                document: StoreDocument::default(),
                was_normalized: false,
            },
        };
        let state = StoreState {
            document: outcome.document.clone(),
            fingerprint: content.as_deref().map(ContentFingerprint::of),
            is_normalization_pending: outcome.was_normalized,
        };
        let store = Self {
            shared: Arc::new(SharedStore {
                path,
                state: Mutex::new(state),
            }),
        };
        Ok((store, outcome))
    }

    /// The path of the store file.
    pub fn path(&self) -> &Path {
        self.shared.path()
    }

    /// The current document: the last one read from or written to the file.
    pub fn document(&self) -> StoreDocument {
        self.shared.lock().document.clone()
    }

    /// Whether the last document read from the file had gaps filled in (ids
    /// generated for hand-written tunnels) that the file does not have yet.
    /// A successful [`save`](Self::save) clears it.
    pub fn has_pending_normalization(&self) -> bool {
        self.shared.lock().is_normalization_pending
    }

    /// Validates `document` and writes it to the file.
    ///
    /// The existing file is updated in place, keeping comments and formatting;
    /// the write is atomic. A file written by a newer version of the app, or
    /// with broken TOML, is refused and left untouched.
    pub fn save(&self, document: StoreDocument) -> Result<(), StoreError> {
        validate(&document).map_err(StoreError::Validation)?;
        let mut state = self.shared.lock();
        let existing = read_if_present(self.path())?;
        let existing_text = existing
            .as_deref()
            .map(|bytes| decode(self.path(), bytes))
            .transpose()?;
        let text = render_document(self.path(), existing_text, &document)?;
        replace_contents(self.path(), text.as_bytes())?;
        state.fingerprint = Some(ContentFingerprint::of(text.as_bytes()));
        state.document = document;
        state.is_normalization_pending = false;
        Ok(())
    }

    pub(crate) fn shared(&self) -> &Arc<SharedStore> {
        &self.shared
    }
}

fn parse_content(path: &Path, content: &[u8]) -> Result<LoadOutcome, StoreError> {
    parse_document(path, decode(path, content)?)
}

fn decode<'a>(path: &Path, content: &'a [u8]) -> Result<&'a str, StoreError> {
    std::str::from_utf8(content).map_err(|error| StoreError::Parse {
        path: path.to_path_buf(),
        location: None,
        message: format!("the file is not valid UTF-8: {error}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::model::TunnelKind;
    use crate::store::test_support::named_tunnel;
    use pretty_assertions::assert_eq;
    use std::fs;

    const HAND_WRITTEN: &str = "# my tunnels\nversion = 1\n\n[[tunnels]]\n# the web app\nname = \"Web\"\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 8080\n";

    fn store_path(directory: &tempfile::TempDir) -> PathBuf {
        directory.path().join("tunneler").join("tunnels.toml")
    }

    fn write_store_file(directory: &tempfile::TempDir, text: &str) -> PathBuf {
        let path = store_path(directory);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }

    fn document_with_one_tunnel() -> StoreDocument {
        StoreDocument {
            tunnels: vec![named_tunnel(TunnelKind::Local, "Adonis", 3333)],
            ..StoreDocument::default()
        }
    }

    #[test]
    fn should_open_with_the_default_document_when_the_file_is_missing() {
        let directory = tempfile::tempdir().unwrap();

        let (store, outcome) = Store::open(store_path(&directory)).unwrap();

        assert_eq!(
            (store.document(), outcome.was_normalized),
            (StoreDocument::default(), false)
        );
    }

    #[test]
    fn should_not_create_the_file_when_opening_a_missing_one() {
        let directory = tempfile::tempdir().unwrap();
        let path = store_path(&directory);

        Store::open(path.clone()).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn should_open_the_document_when_the_file_exists() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, HAND_WRITTEN);

        let (store, _) = Store::open(path).unwrap();

        assert_eq!(store.document().tunnels[0].name, "Web");
    }

    #[test]
    fn should_report_normalization_when_a_tunnel_has_no_id() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, HAND_WRITTEN);

        let (_, outcome) = Store::open(path).unwrap();

        assert!(outcome.was_normalized);
    }

    #[test]
    fn should_not_rewrite_the_file_when_opening_a_file_that_needs_normalization() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, HAND_WRITTEN);

        Store::open(path.clone()).unwrap();

        assert_eq!(fs::read_to_string(path).unwrap(), HAND_WRITTEN);
    }

    #[test]
    fn should_keep_the_generated_id_when_the_normalized_document_is_saved() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, HAND_WRITTEN);
        let (store, outcome) = Store::open(path.clone()).unwrap();
        store.save(outcome.document.clone()).unwrap();

        let (_, reopened) = Store::open(path).unwrap();

        assert_eq!(
            (reopened.document, reopened.was_normalized),
            (outcome.document, false)
        );
    }

    #[test]
    fn should_keep_hand_written_comments_when_the_normalized_document_is_saved() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, HAND_WRITTEN);
        let (store, outcome) = Store::open(path.clone()).unwrap();
        store.save(outcome.document).unwrap();

        let text = fs::read_to_string(path).unwrap();

        assert!(
            text.starts_with("# my tunnels\nversion = 1\n") && text.contains("# the web app\nname"),
            "{text}"
        );
    }

    #[test]
    fn should_create_directory_and_file_when_saving_for_the_first_time() {
        let directory = tempfile::tempdir().unwrap();
        let path = store_path(&directory);
        let (store, _) = Store::open(path.clone()).unwrap();

        store.save(document_with_one_tunnel()).unwrap();

        assert!(path.is_file());
    }

    #[test]
    fn should_expose_the_saved_document_when_saving_succeeds() {
        let directory = tempfile::tempdir().unwrap();
        let (store, _) = Store::open(store_path(&directory)).unwrap();
        let document = document_with_one_tunnel();

        store.save(document.clone()).unwrap();

        assert_eq!(store.document(), document);
    }

    #[test]
    fn should_read_back_the_saved_document_when_reopening() {
        let directory = tempfile::tempdir().unwrap();
        let path = store_path(&directory);
        let (store, _) = Store::open(path.clone()).unwrap();
        let document = document_with_one_tunnel();
        store.save(document.clone()).unwrap();

        let (_, reopened) = Store::open(path).unwrap();

        assert_eq!(reopened.document, document);
    }

    #[test]
    fn should_refuse_to_save_and_write_nothing_when_the_document_is_invalid() {
        let directory = tempfile::tempdir().unwrap();
        let path = store_path(&directory);
        let (store, _) = Store::open(path.clone()).unwrap();
        let invalid = StoreDocument {
            tunnels: vec![named_tunnel(TunnelKind::Local, "Adonis", 0)],
            ..StoreDocument::default()
        };

        let outcome = store.save(invalid);

        assert!(matches!(outcome, Err(StoreError::Validation(_))) && !path.exists());
    }

    #[test]
    fn should_keep_the_previous_document_when_a_save_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let (store, _) = Store::open(store_path(&directory)).unwrap();
        let invalid = StoreDocument {
            tunnels: vec![named_tunnel(TunnelKind::Local, "Adonis", 0)],
            ..StoreDocument::default()
        };

        let _ = store.save(invalid);

        assert_eq!(store.document(), StoreDocument::default());
    }

    #[test]
    fn should_refuse_to_open_a_file_written_by_a_newer_version() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, "version = 2\n");

        assert!(matches!(
            Store::open(path),
            Err(StoreError::UnsupportedVersion {
                found: 2,
                supported: 1
            })
        ));
    }

    #[test]
    fn should_leave_a_newer_file_untouched_when_saving() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, "version = 1\n");
        let (store, _) = Store::open(path.clone()).unwrap();
        fs::write(&path, "version = 2\nfuture = true\n").unwrap();

        let outcome = store.save(document_with_one_tunnel());

        assert!(matches!(
            outcome,
            Err(StoreError::UnsupportedVersion { .. })
        ));
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "version = 2\nfuture = true\n"
        );
    }

    #[test]
    fn should_leave_a_hand_broken_file_untouched_when_saving() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, "version = 1\n");
        let (store, _) = Store::open(path.clone()).unwrap();
        fs::write(&path, "[[tunnels]\n").unwrap();

        let outcome = store.save(document_with_one_tunnel());

        assert!(matches!(outcome, Err(StoreError::Parse { .. })));
        assert_eq!(fs::read_to_string(path).unwrap(), "[[tunnels]\n");
    }

    #[test]
    fn should_report_the_path_when_the_file_is_not_utf8() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, "");
        fs::write(&path, [0xff, 0xfe]).unwrap();

        let message = Store::open(path).err().map(|error| error.to_string());

        assert!(
            message.is_some_and(|text| text.contains("tunnels.toml") && text.contains("UTF-8"))
        );
    }

    #[test]
    fn should_end_the_default_path_with_the_app_directory_and_file_name() {
        let path = default_store_path().unwrap();

        assert!(path.ends_with(Path::new("tunneler").join("tunnels.toml")));
    }

    #[test]
    fn should_skip_reload_when_the_file_content_is_what_the_store_wrote() {
        let directory = tempfile::tempdir().unwrap();
        let (store, _) = Store::open(store_path(&directory)).unwrap();
        store.save(document_with_one_tunnel()).unwrap();

        assert!(store.shared().reload_if_changed().is_none());
    }

    #[cfg(unix)]
    mod unix {
        use super::*;
        use pretty_assertions::assert_eq;
        use std::os::unix::fs::MetadataExt;

        const MODE_MASK: u32 = 0o777;

        fn mode_of(path: &Path) -> u32 {
            fs::metadata(path).unwrap().mode() & MODE_MASK
        }

        #[test]
        fn should_write_a_private_file_when_saving_for_the_first_time() {
            let directory = tempfile::tempdir().unwrap();
            let path = store_path(&directory);
            let (store, _) = Store::open(path.clone()).unwrap();

            store.save(document_with_one_tunnel()).unwrap();

            assert_eq!(mode_of(&path), 0o600);
        }

        #[test]
        fn should_create_a_private_directory_when_saving_for_the_first_time() {
            let directory = tempfile::tempdir().unwrap();
            let path = store_path(&directory);
            let (store, _) = Store::open(path.clone()).unwrap();

            store.save(document_with_one_tunnel()).unwrap();

            assert_eq!(mode_of(path.parent().unwrap()), 0o700);
        }
    }

    #[test]
    fn should_remember_the_pending_normalization_when_opening_a_file_without_ids() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, HAND_WRITTEN);

        let (store, _) = Store::open(path).unwrap();

        assert!(store.has_pending_normalization());
    }

    #[test]
    fn should_have_no_pending_normalization_when_the_file_is_missing() {
        let directory = tempfile::tempdir().unwrap();

        let (store, _) = Store::open(store_path(&directory)).unwrap();

        assert!(!store.has_pending_normalization());
    }

    #[test]
    fn should_clear_the_pending_normalization_when_the_document_is_saved() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_store_file(&directory, HAND_WRITTEN);
        let (store, _) = Store::open(path).unwrap();

        store.save(store.document()).unwrap();

        assert!(!store.has_pending_normalization());
    }
}
