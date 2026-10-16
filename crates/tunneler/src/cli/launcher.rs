//! Starts the daemon in the background on behalf of a command.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use ipc::Endpoint;
use tokio::time::{Instant, sleep};

use super::error::CliError;
use super::locations::Locations;

/// How long a command waits for a daemon it started to answer.
pub const DAEMON_START_TIMEOUT: Duration = Duration::from_secs(5);

const DAEMON_START_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Makes sure a daemon answers on the endpoint, starting one when none does.
///
/// # Errors
///
/// Fails when the daemon cannot be spawned, exits right away, or does not
/// answer within [`DAEMON_START_TIMEOUT`].
pub async fn ensure_daemon_running(
    endpoint: &Endpoint,
    locations: &Locations,
) -> Result<(), CliError> {
    if endpoint.is_daemon_alive().await {
        return Ok(());
    }
    let mut daemon = spawn_detached_daemon(locations).map_err(CliError::DaemonSpawn)?;
    wait_until_alive(endpoint, &mut daemon).await
}

fn daemon_command(locations: &Locations) -> std::io::Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(locations.as_arguments())
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);
    Ok(command)
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

fn spawn_detached_daemon(locations: &Locations) -> std::io::Result<Child> {
    daemon_command(locations)?.spawn()
}

async fn wait_until_alive(endpoint: &Endpoint, daemon: &mut Child) -> Result<(), CliError> {
    let deadline = Instant::now() + DAEMON_START_TIMEOUT;
    loop {
        if endpoint.is_daemon_alive().await {
            return Ok(());
        }
        if let Some(status) = daemon.try_wait().map_err(CliError::DaemonSpawn)? {
            return Err(CliError::DaemonExited {
                status: status.to_string(),
            });
        }
        if Instant::now() >= deadline {
            return Err(CliError::DaemonStartTimedOut {
                waited: DAEMON_START_TIMEOUT,
            });
        }
        sleep(DAEMON_START_POLL_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn should_run_the_daemon_subcommand_with_the_same_locations() {
        let locations = Locations {
            socket: Some(PathBuf::from("/run/t.sock")),
            ..Locations::default()
        };

        let command = daemon_command(&locations).unwrap();

        let arguments: Vec<OsString> = command.get_args().map(OsString::from).collect();
        assert_eq!(
            arguments,
            vec![
                OsString::from("--socket"),
                OsString::from("/run/t.sock"),
                OsString::from("daemon")
            ]
        );
    }
}
