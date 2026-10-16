//! Finds the program that runs the daemon.

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
    ipc::locate_companion_binary(
        DAEMON_BINARY_NAME,
        override_path,
        current_executable,
        exists,
    )
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
