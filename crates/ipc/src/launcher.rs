//! Starts the daemon in the background on behalf of a client.

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use tokio::time::{Instant, sleep};

use crate::endpoint::Endpoint;
use crate::error::LaunchError;

/// How long a client waits for a daemon it started to answer.
pub const DEFAULT_DAEMON_START_TIMEOUT: Duration = Duration::from_secs(5);

const DAEMON_START_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Makes sure a daemon answers on the endpoint, starting one with `command`
/// when none does.
///
/// `command` is taken by value: it is configured here (no standard streams,
/// detached from the caller) and spawned at most once.
///
/// # Errors
///
/// Fails when the daemon cannot be spawned, exits right away, or does not
/// answer within `timeout`.
pub async fn ensure_daemon_running(
    endpoint: &Endpoint,
    mut command: Command,
    timeout: Duration,
) -> Result<(), LaunchError> {
    if endpoint.is_daemon_alive().await {
        return Ok(());
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);
    let mut daemon = command.spawn().map_err(LaunchError::Spawn)?;
    wait_until_alive(endpoint, &mut daemon, timeout).await
}

#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
}

async fn wait_until_alive(
    endpoint: &Endpoint,
    daemon: &mut Child,
    timeout: Duration,
) -> Result<(), LaunchError> {
    let deadline = Instant::now() + timeout;
    loop {
        if endpoint.is_daemon_alive().await {
            return Ok(());
        }
        if let Some(status) = daemon.try_wait().map_err(LaunchError::Spawn)? {
            return Err(LaunchError::Exited {
                status: status.to_string(),
            });
        }
        if Instant::now() >= deadline {
            return Err(LaunchError::TimedOut { waited: timeout });
        }
        sleep(DAEMON_START_POLL_INTERVAL).await;
    }
}

/// A program shipped next to the running one: `override_path` when set, else
/// `<directory of current_executable>/<name><EXE_SUFFIX>` when it exists, else
/// `name`, resolved through `PATH`.
pub fn locate_companion_binary(
    name: &str,
    override_path: Option<OsString>,
    current_executable: Option<&Path>,
    exists: impl Fn(&Path) -> bool,
) -> PathBuf {
    if let Some(path) = override_path {
        return PathBuf::from(path);
    }
    let sibling = current_executable
        .and_then(Path::parent)
        .map(|directory| directory.join(format!("{name}{EXE_SUFFIX}")));
    match sibling {
        Some(path) if exists(&path) => path,
        _ => PathBuf::from(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPANION_NAME: &str = "companion";

    fn sibling_name() -> String {
        format!("{COMPANION_NAME}{EXE_SUFFIX}")
    }

    #[test]
    fn should_use_the_override_when_it_is_set() {
        let located = locate_companion_binary(
            COMPANION_NAME,
            Some(OsString::from("/opt/custom/companion")),
            Some(Path::new("/app/runner")),
            |_| true,
        );

        assert_eq!(located, PathBuf::from("/opt/custom/companion"));
    }

    #[test]
    fn should_use_the_sibling_when_it_exists() {
        let located =
            locate_companion_binary(COMPANION_NAME, None, Some(Path::new("/app/runner")), |_| {
                true
            });

        assert_eq!(located, Path::new("/app").join(sibling_name()));
    }

    #[test]
    fn should_fall_back_to_the_path_lookup_when_the_sibling_is_missing() {
        let located =
            locate_companion_binary(COMPANION_NAME, None, Some(Path::new("/app/runner")), |_| {
                false
            });

        assert_eq!(located, PathBuf::from(COMPANION_NAME));
    }

    #[test]
    fn should_fall_back_to_the_path_lookup_when_the_executable_is_unknown() {
        let located = locate_companion_binary(COMPANION_NAME, None, None, |_| true);

        assert_eq!(located, PathBuf::from(COMPANION_NAME));
    }
}
