//! Finds the program that runs the daemon.

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Environment variable overriding the daemon program.
pub const DAEMON_BINARY_VARIABLE: &str = "TUNNELER_DAEMON_BINARY";

const DAEMON_BINARY_NAME: &str = "tunneler";

/// The daemon program: the override when set, else the sibling of the running
/// executable when it exists, else `tunneler` resolved through `PATH`.
pub fn locate_daemon_binary(
    override_path: Option<OsString>,
    current_executable: Option<&Path>,
    exists: impl Fn(&Path) -> bool,
) -> PathBuf {
    if let Some(path) = override_path {
        return PathBuf::from(path);
    }
    let sibling = current_executable
        .and_then(Path::parent)
        .map(|directory| directory.join(format!("{DAEMON_BINARY_NAME}{EXE_SUFFIX}")));
    match sibling {
        Some(path) if exists(&path) => path,
        _ => PathBuf::from(DAEMON_BINARY_NAME),
    }
}

/// [`locate_daemon_binary`] on the real environment and file system.
pub fn locate_daemon_binary_from_environment() -> PathBuf {
    let current_executable = std::env::current_exe().ok();
    locate_daemon_binary(
        std::env::var_os(DAEMON_BINARY_VARIABLE),
        current_executable.as_deref(),
        Path::exists,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sibling_name() -> String {
        format!("{DAEMON_BINARY_NAME}{EXE_SUFFIX}")
    }

    #[test]
    fn should_use_the_override_when_the_variable_is_set() {
        let located = locate_daemon_binary(
            Some(OsString::from("/opt/custom/tunneler")),
            Some(Path::new("/app/tunneler-ui")),
            |_| true,
        );

        assert_eq!(located, PathBuf::from("/opt/custom/tunneler"));
    }

    #[test]
    fn should_use_the_sibling_when_it_exists() {
        let located = locate_daemon_binary(None, Some(Path::new("/app/tunneler-ui")), |_| true);

        assert_eq!(located, Path::new("/app").join(sibling_name()));
    }

    #[test]
    fn should_fall_back_to_the_path_lookup_when_the_sibling_is_missing() {
        let located = locate_daemon_binary(None, Some(Path::new("/app/tunneler-ui")), |_| false);

        assert_eq!(located, PathBuf::from("tunneler"));
    }

    #[test]
    fn should_fall_back_to_the_path_lookup_when_the_executable_is_unknown() {
        let located = locate_daemon_binary(None, None, |_| true);

        assert_eq!(located, PathBuf::from("tunneler"));
    }
}
