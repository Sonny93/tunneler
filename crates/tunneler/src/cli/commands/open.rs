use ipc::ClientError;
use ipc::protocol::ErrorCode;

use super::super::args::OpenArguments;
use super::super::launcher::ensure_daemon_running;
use super::super::render::describe_route;
use super::super::session::{ConnectionMode, connect};
use super::super::target::find_tunnel;
use super::super::{CliError, Context};
use crate::logging;

/// `tunneler open`: asks the daemon to open a local tunnel in the browser,
/// starting the daemon and the tunnel first when needed.
pub async fn open(context: &Context, arguments: &OpenArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    ensure_daemon_running(&context.endpoint, &context.locations).await?;
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    let snapshot = client.snapshot().await?;
    let definition = find_tunnel(&arguments.target, &snapshot)?;
    let label = describe_route(definition, &snapshot.settings, context.style);
    let target = client
        .open_tunnel(&definition.id, arguments.with.as_deref())
        .await
        .map_err(|error| open_error(error, label))?;
    println!("Opening {target}");
    Ok(())
}

fn open_error(error: ClientError, label: String) -> CliError {
    match error {
        ClientError::Remote {
            code: ErrorCode::InvalidState,
            message,
            ..
        } => CliError::NotOpenable {
            label,
            reason: message,
        },
        ClientError::Remote {
            code: ErrorCode::OpenFailed,
            message,
            ..
        } => CliError::OpenFailed { label, message },
        ClientError::Remote {
            code: ErrorCode::UnknownOpener,
            message,
            ..
        } => CliError::UnknownOpener { message },
        other => CliError::Client(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(code: ErrorCode) -> ClientError {
        ClientError::Remote {
            code,
            message: "boom".to_owned(),
            data: None,
        }
    }

    #[test]
    fn should_report_not_openable_when_the_daemon_answers_invalid_state() {
        let error = open_error(remote(ErrorCode::InvalidState), "route".to_owned());

        assert!(matches!(error, CliError::NotOpenable { reason, .. } if reason == "boom"));
    }

    #[test]
    fn should_report_open_failed_when_the_daemon_answers_open_failed() {
        let error = open_error(remote(ErrorCode::OpenFailed), "route".to_owned());

        assert!(matches!(error, CliError::OpenFailed { message, .. } if message == "boom"));
    }

    #[test]
    fn should_report_an_unknown_opener_with_the_daemon_message() {
        let error = open_error(remote(ErrorCode::UnknownOpener), "route".to_owned());

        assert!(matches!(error, CliError::UnknownOpener { message } if message == "boom"));
    }

    #[test]
    fn should_keep_other_daemon_errors_as_they_are() {
        let error = open_error(remote(ErrorCode::NotFound), "route".to_owned());

        assert!(matches!(error, CliError::Client(_)));
    }
}
