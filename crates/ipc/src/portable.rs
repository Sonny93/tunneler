//! Portable mode: the program keeps its files next to its executable instead
//! of in the user's directories.
//!
//! The mode is on when a file named `portable` sits next to the running
//! executable. The daemon and the UI ship side by side, so both detect it the
//! same way and agree on where things are.

use std::path::{Path, PathBuf};

/// The name of the file whose presence next to the executable turns portable
/// mode on.
pub const PORTABLE_MARKER_FILE_NAME: &str = "portable";

const TUNNELS_FILE_NAME: &str = "tunnels.toml";
const DATA_DIRECTORY_NAME: &str = "data";
const STATE_FILE_NAME: &str = "state.toml";
const LOG_DIRECTORY_NAME: &str = "logs";
const WEBVIEW_DIRECTORY_NAME: &str = "webview";

/// Where a portable installation keeps its files, all under the directory of
/// the executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortableLayout {
    executable_directory: PathBuf,
}

impl PortableLayout {
    /// The directory holding the executables, the marker and `tunnels.toml`.
    pub fn executable_directory(&self) -> &Path {
        &self.executable_directory
    }

    /// The marker file that turns portable mode on.
    pub fn marker_file(&self) -> PathBuf {
        self.executable_directory.join(PORTABLE_MARKER_FILE_NAME)
    }

    /// The tunnels file, next to the executable.
    pub fn tunnels_file(&self) -> PathBuf {
        self.executable_directory.join(TUNNELS_FILE_NAME)
    }

    /// The directory for everything that is not the tunnels file: `data/`.
    pub fn data_directory(&self) -> PathBuf {
        self.executable_directory.join(DATA_DIRECTORY_NAME)
    }

    /// The state file: `data/state.toml`.
    pub fn state_file(&self) -> PathBuf {
        self.data_directory().join(STATE_FILE_NAME)
    }

    /// The daemon's log directory: `data/logs`.
    pub fn log_directory(&self) -> PathBuf {
        self.data_directory().join(LOG_DIRECTORY_NAME)
    }

    /// Where the webview (`WebView2`, `WebKitGTK`) keeps its data: `data/webview`.
    pub fn webview_data_directory(&self) -> PathBuf {
        self.data_directory().join(WEBVIEW_DIRECTORY_NAME)
    }
}

/// The portable layout of an executable living in `executable_directory`, or
/// `None` when the marker is absent.
pub fn portable_layout(
    executable_directory: &Path,
    is_marker_present: bool,
) -> Option<PortableLayout> {
    is_marker_present.then(|| PortableLayout {
        executable_directory: executable_directory.to_path_buf(),
    })
}

/// [`portable_layout`] for the running executable, looking for the marker on
/// the file system. `None` too when the executable's location is unknown.
pub fn detect_portable_layout() -> Option<PortableLayout> {
    let executable = std::env::current_exe().ok()?;
    let executable_directory = executable.parent()?;
    let is_marker_present = executable_directory
        .join(PORTABLE_MARKER_FILE_NAME)
        .is_file();
    portable_layout(executable_directory, is_marker_present)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    const EXECUTABLE_DIRECTORY: &str = "/opt/tunneler";

    fn layout() -> PortableLayout {
        portable_layout(Path::new(EXECUTABLE_DIRECTORY), true).unwrap()
    }

    #[test]
    fn should_be_portable_when_the_marker_is_present() {
        assert!(portable_layout(Path::new(EXECUTABLE_DIRECTORY), true).is_some());
    }

    #[test]
    fn should_not_be_portable_when_the_marker_is_absent() {
        assert_eq!(
            portable_layout(Path::new(EXECUTABLE_DIRECTORY), false),
            None
        );
    }

    #[test]
    fn should_keep_the_tunnels_file_next_to_the_executable() {
        assert_eq!(
            layout().tunnels_file(),
            Path::new("/opt/tunneler/tunnels.toml")
        );
    }

    #[test]
    fn should_keep_the_state_file_in_the_data_directory() {
        assert_eq!(
            layout().state_file(),
            Path::new("/opt/tunneler/data/state.toml")
        );
    }

    #[test]
    fn should_keep_the_logs_in_the_data_directory() {
        assert_eq!(
            layout().log_directory(),
            Path::new("/opt/tunneler/data/logs")
        );
    }

    #[test]
    fn should_keep_the_webview_data_in_the_data_directory() {
        assert_eq!(
            layout().webview_data_directory(),
            Path::new("/opt/tunneler/data/webview")
        );
    }

    #[test]
    fn should_name_the_marker_next_to_the_executable() {
        assert_eq!(layout().marker_file(), Path::new("/opt/tunneler/portable"));
    }

    #[test]
    fn should_not_find_the_marker_next_to_the_test_executable() {
        assert_eq!(detect_portable_layout(), None);
    }
}
