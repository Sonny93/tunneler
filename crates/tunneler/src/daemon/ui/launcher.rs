//! Starting the UI program.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use ipc::locate_companion_binary;
use thiserror::Error;
use tokio::process::Command;

const UI_BINARY_NAME: &str = "tunneler-ui";
const UI_BINARY_VARIABLE: &str = "TUNNELER_UI_BINARY";
const SOCKET_VARIABLE: &str = "TUNNELER_SOCKET";
const DAEMON_BINARY_VARIABLE: &str = "TUNNELER_DAEMON_BINARY";

#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// Why the UI program could not be started.
#[derive(Debug, Error)]
pub(crate) enum UiLaunchError {
    /// The program is missing or cannot be executed.
    #[error("cannot start the UI program `{}`: {source}", .program.display())]
    Spawn {
        /// What was run.
        program: PathBuf,
        /// Why it failed.
        #[source]
        source: io::Error,
    },
}

/// Starts a UI that connects to this daemon.
pub(crate) trait UiLauncher: Send + Sync {
    /// Starts the UI program without waiting for it.
    fn launch(&self) -> Result<(), UiLaunchError>;
}

/// Starts `tunneler-ui` as a detached process of its own.
#[derive(Debug, Clone)]
pub(crate) struct ProcessUiLauncher {
    socket: Option<PathBuf>,
}

impl ProcessUiLauncher {
    /// A launcher whose UIs talk to the daemon listening on `socket`; `None`
    /// when the UI finds the daemon by itself.
    pub(crate) fn new(socket: Option<PathBuf>) -> Self {
        Self { socket }
    }

    fn command(&self, program: &Path, daemon_binary: Option<&Path>) -> Command {
        let mut command = Command::new(program);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(socket) = &self.socket {
            command.env(SOCKET_VARIABLE, socket);
        }
        if let Some(daemon_binary) = daemon_binary {
            command.env(DAEMON_BINARY_VARIABLE, daemon_binary);
        }
        detach(&mut command);
        command
    }
}

impl UiLauncher for ProcessUiLauncher {
    fn launch(&self) -> Result<(), UiLaunchError> {
        let daemon_binary = std::env::current_exe().ok();
        let program = locate_ui_binary(
            std::env::var_os(UI_BINARY_VARIABLE),
            daemon_binary.as_deref(),
        );
        let mut child = self
            .command(&program, daemon_binary.as_deref())
            .spawn()
            .map_err(|source| UiLaunchError::Spawn {
                program: program.clone(),
                source,
            })?;
        tracing::info!(program = %program.display(), "the UI was launched");
        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => tracing::info!(%status, "the UI program ended"),
                Err(error) => tracing::warn!(%error, "cannot wait for the UI program"),
            }
        });
        Ok(())
    }
}

fn locate_ui_binary(override_path: Option<OsString>, daemon_binary: Option<&Path>) -> PathBuf {
    locate_companion_binary(UI_BINARY_NAME, override_path, daemon_binary, Path::exists)
}

#[cfg(unix)]
fn detach(command: &mut Command) {
    command.process_group(0);
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_use_the_override_when_one_is_given() {
        let located = locate_ui_binary(Some(OsString::from("/opt/ui")), None);

        assert_eq!(located, PathBuf::from("/opt/ui"));
    }

    #[test]
    fn should_look_the_ui_up_on_the_path_when_nothing_sits_next_to_the_daemon() {
        let located = locate_ui_binary(None, Some(Path::new("/nonexistent/tunneler")));

        assert_eq!(located, PathBuf::from(UI_BINARY_NAME));
    }

    #[test]
    fn should_tell_which_program_failed_to_start() {
        let error = UiLaunchError::Spawn {
            program: PathBuf::from("tunneler-ui"),
            source: io::Error::from(io::ErrorKind::NotFound),
        };

        assert!(error.to_string().contains("`tunneler-ui`"));
    }
}
