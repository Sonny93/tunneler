use ipc::ClientError;
use ipc::protocol::ErrorCode;

use crate::cli::Context;
use crate::cli::error::CliError;
use crate::cli::session::{ConnectionMode, connect};
use crate::{daemon, logging};

const ALREADY_RUNNING_MESSAGE: &str = "Tunneler is already running (the UI is not available yet).";

/// `tunneler daemon`: runs the daemon in the foreground.
pub async fn daemon(context: &Context) -> Result<(), CliError> {
    let _log_guard = logging::init_daemon_logging();
    daemon::run(context.locations.daemon_options()?).await?;
    Ok(())
}

/// `tunneler` alone: asks the running daemon to show the UI, or becomes the
/// daemon when none runs.
pub async fn show_or_run_daemon(context: &Context) -> Result<(), CliError> {
    if !context.endpoint.is_daemon_alive().await {
        return daemon(context).await;
    }
    logging::init_cli_logging();
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    match client.show_ui().await {
        Ok(()) => Ok(()),
        Err(ClientError::Remote {
            code: ErrorCode::UiUnavailable,
            ..
        }) => {
            println!("{ALREADY_RUNNING_MESSAGE}");
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}
