use std::time::Duration;

use ipc::protocol::ErrorCode;
use ipc::{ClientError, Endpoint, IpcClient};
use serde_json::Value;
use tokio::time::{Instant, sleep};

use super::super::args::QuitArguments;
use super::super::session::{ConnectionMode, connect};
use super::super::terminal::{StandardTerminal, Terminal, is_interactive};
use super::super::{CliError, Context};
use crate::logging;

/// How long `quit` waits for the daemon to be gone.
const DAEMON_STOP_TIMEOUT: Duration = Duration::from_secs(10);
const DAEMON_STOP_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// `tunneler quit`: stops the daemon, asking first when tunnels are active.
pub async fn quit(context: &Context, arguments: &QuitArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    quit_and_wait(&context.endpoint, arguments.yes).await?;
    println!("Tunneler stopped.");
    Ok(())
}

/// Stops the daemon and the active tunnels without asking, and waits until the
/// daemon is gone.
pub(super) async fn stop_daemon(endpoint: &Endpoint) -> Result<(), CliError> {
    quit_and_wait(endpoint, true).await
}

async fn quit_and_wait(endpoint: &Endpoint, assume_yes: bool) -> Result<(), CliError> {
    let client = connect(endpoint, ConnectionMode::default()).await?;
    request_quit(&client, assume_yes).await?;
    drop(client);
    wait_until_stopped(endpoint).await
}

async fn request_quit(client: &IpcClient, assume_yes: bool) -> Result<(), CliError> {
    if assume_yes {
        return Ok(client.quit(true).await?);
    }
    match client.quit(false).await {
        Err(ClientError::Remote {
            code: ErrorCode::TunnelsRunning,
            data,
            ..
        }) => {
            let running = running_tunnels(data.as_ref());
            if confirm_forced_quit(running, &StandardTerminal, is_interactive())? {
                return Ok(client.quit(true).await?);
            }
            Err(CliError::ConfirmationRequired {
                running: running.unwrap_or_default(),
            })
        }
        other => Ok(other?),
    }
}

fn running_tunnels(data: Option<&Value>) -> Option<usize> {
    let count = data?.as_u64()?;
    usize::try_from(count).ok()
}

fn describe_confirmation(running: Option<usize>) -> String {
    match running {
        Some(1) => "1 tunnel or X11 session is active. Stop it and quit? [y/N] ".to_owned(),
        Some(count) => {
            format!("{count} tunnels or X11 sessions are active. Stop them all and quit? [y/N] ")
        }
        None => "Tunnels or X11 sessions are active. Stop them all and quit? [y/N] ".to_owned(),
    }
}

/// Asks whether to stop the active tunnels. Never asks without a terminal:
/// then the answer is an error telling to pass `--yes`.
fn confirm_forced_quit(
    running: Option<usize>,
    terminal: &dyn Terminal,
    is_terminal: bool,
) -> Result<bool, CliError> {
    if !is_terminal {
        return Err(CliError::ConfirmationRequired {
            running: running.unwrap_or_default(),
        });
    }
    let answer = terminal
        .read_line(&describe_confirmation(running))
        .map_err(CliError::Terminal)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

async fn wait_until_stopped(endpoint: &Endpoint) -> Result<(), CliError> {
    let deadline = Instant::now() + DAEMON_STOP_TIMEOUT;
    while endpoint.is_daemon_alive().await {
        if Instant::now() >= deadline {
            return Err(CliError::DaemonStopTimedOut {
                waited: DAEMON_STOP_TIMEOUT,
            });
        }
        sleep(DAEMON_STOP_POLL_INTERVAL).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Mutex;

    use serde_json::json;
    use zeroize::Zeroizing;

    use super::*;

    struct ScriptedTerminal {
        answer: String,
        asked: Mutex<Vec<String>>,
    }

    impl ScriptedTerminal {
        fn answering(answer: &str) -> Self {
            Self {
                answer: answer.to_owned(),
                asked: Mutex::new(Vec::new()),
            }
        }
    }

    impl Terminal for ScriptedTerminal {
        fn write(&self, _text: &str) {}

        fn read_secret(&self, _prompt: &str) -> io::Result<Zeroizing<String>> {
            Err(io::Error::from(io::ErrorKind::Unsupported))
        }

        fn read_line(&self, prompt: &str) -> io::Result<String> {
            self.asked.lock().unwrap().push(prompt.to_owned());
            Ok(self.answer.clone())
        }
    }

    #[test]
    fn should_read_the_count_of_running_tunnels_from_the_error_data() {
        assert_eq!(running_tunnels(Some(&json!(3))), Some(3));
    }

    #[test]
    fn should_not_know_the_count_when_the_error_has_no_data() {
        assert_eq!(running_tunnels(None), None);
    }

    #[test]
    fn should_ask_how_many_tunnels_are_active_when_confirming() {
        let terminal = ScriptedTerminal::answering("y");

        confirm_forced_quit(Some(3), &terminal, true).unwrap();

        assert_eq!(
            terminal.asked.lock().unwrap().clone(),
            vec!["3 tunnels or X11 sessions are active. Stop them all and quit? [y/N] ".to_owned()]
        );
    }

    #[test]
    fn should_use_the_singular_when_one_tunnel_is_active() {
        assert_eq!(
            describe_confirmation(Some(1)),
            "1 tunnel or X11 session is active. Stop it and quit? [y/N] "
        );
    }

    #[test]
    fn should_confirm_when_the_user_answers_yes() {
        let terminal = ScriptedTerminal::answering("Yes");

        assert!(confirm_forced_quit(Some(2), &terminal, true).unwrap());
    }

    #[test]
    fn should_not_confirm_when_the_user_answers_nothing() {
        let terminal = ScriptedTerminal::answering("");

        assert!(!confirm_forced_quit(Some(2), &terminal, true).unwrap());
    }

    #[test]
    fn should_not_ask_when_there_is_no_terminal() {
        let terminal = ScriptedTerminal::answering("y");

        let outcome = confirm_forced_quit(Some(2), &terminal, false);

        assert!(matches!(
            outcome,
            Err(CliError::ConfirmationRequired { running: 2 })
        ));
        assert!(terminal.asked.lock().unwrap().is_empty());
    }
}
