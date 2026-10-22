//! Where the program keeps its files: the user's directories, or the folder of
//! the executable in portable mode.
//!
//! `--config` and `--state` (and their environment variables) are applied by
//! the callers on top of this layout.

use std::path::{Path, PathBuf};

use directories::BaseDirs;
use engine::{StoreError, default_state_path, default_store_path};
use ipc::{PortableLayout, detect_portable_layout};
use thiserror::Error;

/// The Tauri identifier of the UI (`tauri.conf.json`): the name of the
/// directory where the webview keeps its data on Windows and Linux.
const WEBVIEW_DATA_DIRECTORY_NAME: &str = "com.minimalstuff.tunneler";
const APPLICATION_DIRECTORY_NAME: &str = "tunneler";
const LOG_DIRECTORY_NAME: &str = "logs";

/// Why the layout could not be determined.
#[derive(Debug, Error)]
pub enum LayoutError {
    /// The platform has no user data directory.
    #[error("the platform has no user data directory")]
    NoDataDirectory,
    /// The default tunnels or state file could not be located.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// The default location of every file the program owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLayout {
    /// `tunnels.toml`.
    pub tunnels_file: PathBuf,
    /// `state.toml`.
    pub state_file: PathBuf,
    /// The daemon's log files.
    pub log_directory: PathBuf,
    /// What the UI's webview writes.
    pub webview_data_directory: PathBuf,
    /// The directories that belong to Tunneler alone and are worth removing
    /// once they are empty, parents of the files above.
    pub removable_when_empty: Vec<PathBuf>,
}

impl DataLayout {
    /// The layout of the running program: portable when the marker sits next
    /// to the executable, the user's directories otherwise.
    pub fn resolve() -> Result<Self, LayoutError> {
        match detect_portable_layout() {
            Some(portable) => Ok(Self::portable(&portable)),
            None => Self::resolve_for_user(),
        }
    }

    /// The layout of a portable installation.
    pub fn portable(portable: &PortableLayout) -> Self {
        Self {
            tunnels_file: portable.tunnels_file(),
            state_file: portable.state_file(),
            log_directory: portable.log_directory(),
            webview_data_directory: portable.webview_data_directory(),
            removable_when_empty: vec![portable.data_directory()],
        }
    }

    /// The layout under the user's directories.
    fn resolve_for_user() -> Result<Self, LayoutError> {
        let base_directories = BaseDirs::new().ok_or(LayoutError::NoDataDirectory)?;
        Ok(Self::for_user(
            default_store_path()?,
            default_state_path()?,
            base_directories.data_local_dir(),
        ))
    }

    fn for_user(tunnels_file: PathBuf, state_file: PathBuf, data_local_directory: &Path) -> Self {
        let log_directory = data_local_directory
            .join(APPLICATION_DIRECTORY_NAME)
            .join(LOG_DIRECTORY_NAME);
        let mut removable_when_empty: Vec<PathBuf> = Vec::new();
        for parent in [&tunnels_file, &state_file, &log_directory]
            .into_iter()
            .filter_map(|path| path.parent())
        {
            if !removable_when_empty.iter().any(|known| known == parent) {
                removable_when_empty.push(parent.to_path_buf());
            }
        }
        Self {
            tunnels_file,
            state_file,
            log_directory,
            webview_data_directory: data_local_directory.join(WEBVIEW_DATA_DIRECTORY_NAME),
            removable_when_empty,
        }
    }
}

#[cfg(test)]
mod tests {
    use ipc::portable_layout;
    use pretty_assertions::assert_eq;

    use super::*;

    fn user_layout() -> DataLayout {
        DataLayout::for_user(
            PathBuf::from("/home/me/.config/tunneler/tunnels.toml"),
            PathBuf::from("/home/me/.local/state/tunneler/state.toml"),
            Path::new("/home/me/.local/share"),
        )
    }

    fn portable() -> DataLayout {
        DataLayout::portable(&portable_layout(Path::new("/opt/tunneler"), true).unwrap())
    }

    #[test]
    fn should_keep_the_logs_in_a_tunneler_directory_under_the_data_directory() {
        assert_eq!(
            user_layout().log_directory,
            PathBuf::from("/home/me/.local/share/tunneler/logs")
        );
    }

    #[test]
    fn should_keep_the_webview_data_in_the_directory_named_after_the_identifier() {
        assert_eq!(
            user_layout().webview_data_directory,
            PathBuf::from("/home/me/.local/share/com.minimalstuff.tunneler")
        );
    }

    #[test]
    fn should_remove_the_tunneler_directories_of_the_user_once_they_are_empty() {
        assert_eq!(
            user_layout().removable_when_empty,
            vec![
                PathBuf::from("/home/me/.config/tunneler"),
                PathBuf::from("/home/me/.local/state/tunneler"),
                PathBuf::from("/home/me/.local/share/tunneler"),
            ]
        );
    }

    #[test]
    fn should_remove_each_directory_once_when_the_state_and_the_logs_share_one() {
        let layout = DataLayout::for_user(
            PathBuf::from("/home/me/AppData/Roaming/tunneler/tunnels.toml"),
            PathBuf::from("/home/me/AppData/Local/tunneler/state.toml"),
            Path::new("/home/me/AppData/Local"),
        );

        assert_eq!(
            layout.removable_when_empty,
            vec![
                PathBuf::from("/home/me/AppData/Roaming/tunneler"),
                PathBuf::from("/home/me/AppData/Local/tunneler"),
            ]
        );
    }

    #[test]
    fn should_keep_the_files_of_a_portable_installation_next_to_the_executable() {
        let layout = portable();

        assert_eq!(
            (
                layout.tunnels_file,
                layout.state_file,
                layout.log_directory,
                layout.webview_data_directory,
            ),
            (
                PathBuf::from("/opt/tunneler/tunnels.toml"),
                PathBuf::from("/opt/tunneler/data/state.toml"),
                PathBuf::from("/opt/tunneler/data/logs"),
                PathBuf::from("/opt/tunneler/data/webview"),
            )
        );
    }

    #[test]
    fn should_remove_only_the_data_directory_of_a_portable_installation_once_it_is_empty() {
        assert_eq!(
            portable().removable_when_empty,
            vec![PathBuf::from("/opt/tunneler/data")]
        );
    }

    #[test]
    fn should_use_the_identifier_declared_for_the_ui() {
        let configuration: serde_json::Value =
            serde_json::from_str(include_str!("../../../app/src-tauri/tauri.conf.json")).unwrap();

        assert_eq!(configuration["identifier"], WEBVIEW_DATA_DIRECTORY_NAME);
    }
}
