use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};

use super::atomic_file::{read_if_present, replace_contents};
use super::error::{StoreError, io_error};
use super::model::TunnelId;

const APP_DIRECTORY_NAME: &str = "tunneler";
const STATE_FILE_NAME: &str = "state.toml";
const CURRENT_VERSION: u32 = 1;

/// What the app remembers between two runs, apart from the hand-edited
/// configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppState {
    /// The tunnels the user wants running.
    pub running: BTreeSet<TunnelId>,
    /// The canonical paths of the identity files whose passphrase is stored
    /// in the OS keyring.
    pub keyring_entries: BTreeSet<PathBuf>,
}

/// Where the [`AppState`] is kept.
///
/// Both operations are blocking; async callers run them off the executor.
pub trait StateStore: Send + Sync {
    /// Reads the state; a store that holds nothing yet yields the empty state.
    fn load(&self) -> Result<AppState, StoreError>;

    /// Replaces the stored state.
    fn save(&self, state: &AppState) -> Result<(), StoreError>;
}

/// The default location of the state file.
///
/// This is `tunneler/state.toml` inside the user state directory reported by
/// [`directories::BaseDirs::state_dir`] (Linux: `$XDG_STATE_HOME`, usually
/// `~/.local/state`), or inside the local data directory on the platforms
/// that have no state directory.
pub fn default_state_path() -> Result<PathBuf, StoreError> {
    let base_directories = BaseDirs::new().ok_or(StoreError::ConfigDirectoryUnavailable)?;
    let directory = base_directories
        .state_dir()
        .unwrap_or_else(|| base_directories.data_local_dir());
    Ok(directory.join(APP_DIRECTORY_NAME).join(STATE_FILE_NAME))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredState {
    version: u32,
    #[serde(default)]
    running: Vec<String>,
    #[serde(default)]
    keyring_entries: Vec<PathBuf>,
}

impl StoredState {
    fn from_state(state: &AppState) -> Self {
        Self {
            version: CURRENT_VERSION,
            running: state.running.iter().map(ToString::to_string).collect(),
            keyring_entries: state.keyring_entries.iter().cloned().collect(),
        }
    }

    fn into_state(self, path: &Path) -> Result<AppState, StoreError> {
        let running = self
            .running
            .iter()
            .map(|text| {
                text.parse::<TunnelId>().map_err(|error| StoreError::Parse {
                    path: path.to_path_buf(),
                    location: None,
                    message: error.to_string(),
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(AppState {
            running,
            keyring_entries: self.keyring_entries.into_iter().collect(),
        })
    }
}

/// The state kept in a versioned TOML file (`state.toml`), written
/// atomically. The file is not watched: only this process writes it.
#[derive(Debug, Clone)]
pub struct FileStateStore {
    path: PathBuf,
}

impl FileStateStore {
    /// A store backed by the file at `path`.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The path of the state file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl StateStore for FileStateStore {
    fn load(&self) -> Result<AppState, StoreError> {
        let Some(content) = read_if_present(&self.path)? else {
            return Ok(AppState::default());
        };
        let text = String::from_utf8(content).map_err(|error| {
            io_error(
                &self.path,
                std::io::Error::new(std::io::ErrorKind::InvalidData, error),
            )
        })?;
        let stored: StoredState =
            toml_edit::de::from_str(&text).map_err(|error| StoreError::Parse {
                path: self.path.clone(),
                location: None,
                message: error.message().to_owned(),
            })?;
        if stored.version > CURRENT_VERSION {
            return Err(StoreError::UnsupportedVersion {
                found: stored.version,
                supported: CURRENT_VERSION,
            });
        }
        stored.into_state(&self.path)
    }

    fn save(&self, state: &AppState) -> Result<(), StoreError> {
        let text = toml_edit::ser::to_string(&StoredState::from_state(state)).map_err(|error| {
            StoreError::Parse {
                path: self.path.clone(),
                location: None,
                message: error.to_string(),
            }
        })?;
        replace_contents(&self.path, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use pretty_assertions::assert_eq;

    use super::*;

    fn sample_state() -> AppState {
        AppState {
            running: BTreeSet::from([TunnelId::generate(), TunnelId::generate()]),
            keyring_entries: BTreeSet::from([PathBuf::from("/home/me/.ssh/id_ed25519")]),
        }
    }

    fn store_in(directory: &tempfile::TempDir) -> FileStateStore {
        FileStateStore::new(directory.path().join("state.toml"))
    }

    #[test]
    fn should_read_back_the_state_when_it_was_saved() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        let state = sample_state();

        store.save(&state).unwrap();

        assert_eq!(store.load().unwrap(), state);
    }

    #[test]
    fn should_load_the_empty_state_when_the_file_is_missing() {
        let directory = tempfile::tempdir().unwrap();

        assert_eq!(store_in(&directory).load().unwrap(), AppState::default());
    }

    #[test]
    fn should_write_the_format_version_when_saving() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);

        store.save(&AppState::default()).unwrap();

        assert!(
            fs::read_to_string(store.path())
                .unwrap()
                .starts_with("version = 1")
        );
    }

    #[test]
    fn should_fail_to_load_when_the_file_is_not_toml() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        fs::write(store.path(), "running = [").unwrap();

        assert!(matches!(store.load(), Err(StoreError::Parse { .. })));
    }

    #[test]
    fn should_fail_to_load_when_a_running_id_is_invalid() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        fs::write(store.path(), "version = 1\nrunning = [\"nope\"]\n").unwrap();

        assert!(matches!(store.load(), Err(StoreError::Parse { .. })));
    }

    #[test]
    fn should_fail_to_load_when_the_version_is_newer() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        fs::write(store.path(), "version = 2\n").unwrap();

        assert!(matches!(
            store.load(),
            Err(StoreError::UnsupportedVersion { found: 2, .. })
        ));
    }

    #[test]
    fn should_leave_no_temporary_file_when_saving() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);

        store.save(&sample_state()).unwrap();

        let names: Vec<String> = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["state.toml"]);
    }

    #[test]
    fn should_create_missing_directories_when_saving() {
        let directory = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(directory.path().join("a").join("state.toml"));

        store.save(&AppState::default()).unwrap();

        assert!(store.path().is_file());
    }

    #[test]
    fn should_end_the_default_path_with_the_application_directory_and_file() {
        let path = default_state_path().unwrap();

        assert!(path.ends_with("tunneler/state.toml"));
    }
}
