//! Starts the daemon in the background on behalf of a command.

use std::process::Command;

use ipc::{DEFAULT_DAEMON_START_TIMEOUT, Endpoint, LaunchError};

use super::error::CliError;
use super::locations::Locations;

/// Makes sure a daemon answers on the endpoint, starting one when none does.
///
/// # Errors
///
/// Fails when the daemon cannot be spawned, exits right away, or does not
/// answer within [`DEFAULT_DAEMON_START_TIMEOUT`].
pub async fn ensure_daemon_running(
    endpoint: &Endpoint,
    locations: &Locations,
) -> Result<(), CliError> {
    let command = daemon_command(locations).map_err(CliError::DaemonSpawn)?;
    ipc::ensure_daemon_running(endpoint, command, DEFAULT_DAEMON_START_TIMEOUT)
        .await
        .map_err(launch_error_to_cli)
}

fn daemon_command(locations: &Locations) -> std::io::Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command.args(locations.as_arguments()).arg("daemon");
    Ok(command)
}

fn launch_error_to_cli(error: LaunchError) -> CliError {
    match error {
        LaunchError::Spawn(source) => CliError::DaemonSpawn(source),
        LaunchError::Exited { status } => CliError::DaemonExited { status },
        LaunchError::TimedOut { waited } => CliError::DaemonStartTimedOut { waited },
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
