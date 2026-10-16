use ipc::protocol::{ClientKind, Hello};
use ipc::{ClientError, Endpoint, IpcClient};

use super::error::CliError;

/// What a command asks of its connection to the daemon.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConnectionMode {
    /// Whether the daemon pushes its events.
    pub subscribes_to_events: bool,
    /// Whether the daemon may send prompts to this command.
    pub can_prompt: bool,
}

/// Connects to the running daemon as a CLI client.
pub async fn connect(endpoint: &Endpoint, mode: ConnectionMode) -> Result<IpcClient, CliError> {
    let hello = Hello::new(
        ClientKind::Cli,
        env!("CARGO_PKG_VERSION"),
        mode.subscribes_to_events,
        mode.can_prompt,
    );
    IpcClient::connect(endpoint, hello)
        .await
        .map_err(|error| match error {
            ClientError::DaemonNotRunning => CliError::DaemonNotRunning,
            other => CliError::Client(other),
        })
}
