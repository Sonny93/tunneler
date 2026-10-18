use ipc::ClientError;
use ipc::protocol::ErrorCode;

use crate::cli::Context;
use crate::cli::error::CliError;
use crate::cli::session::{ConnectionMode, connect};
use crate::daemon::DaemonOptions;
use crate::{daemon, logging};

/// `tunneler daemon`: runs the daemon in the foreground.
pub async fn daemon(context: &Context) -> Result<(), CliError> {
    run_daemon(context.locations.daemon_options()?).await
}

/// `tunneler` alone: asks the running daemon to show the UI, or becomes the
/// daemon, which then opens the UI, when none runs.
pub async fn show_or_run_daemon(context: &Context) -> Result<(), CliError> {
    if !context.endpoint.is_daemon_alive().await {
        return run_daemon(DaemonOptions {
            should_open_ui_on_start: true,
            ..context.locations.daemon_options()?
        })
        .await;
    }
    logging::init_cli_logging();
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    match client.show_ui().await {
        Ok(()) => Ok(()),
        Err(ClientError::Remote {
            code: ErrorCode::UiUnavailable,
            message,
            ..
        }) => Err(CliError::UiUnavailable { message }),
        Err(error) => Err(error.into()),
    }
}

async fn run_daemon(options: DaemonOptions) -> Result<(), CliError> {
    daemon::release_console_if_owned();
    let _log_guard = logging::init_daemon_logging();
    daemon::run(options).await?;
    Ok(())
}
