//! Starts the daemon in the background on behalf of a client.

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
