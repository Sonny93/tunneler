use std::future::Future;
use std::sync::Arc;

use ipc::protocol::{
    DevslotSkipReasonDto, DevslotUpResultDto, ErrorCode, EventMessage, HostKeyRequestEvent,
};
use ipc::{ClientError, EventReceiver};

use super::super::args::DevslotArguments;
use super::super::devslot_render::{render_devslot_up, render_devslot_up_json};
use super::super::launcher::ensure_daemon_running;
use super::super::prompting::{LocalPrompts, describe_host_key_alert};
use super::super::session::{ConnectionMode, connect};
use super::super::terminal::{StandardTerminal, Terminal, is_interactive};
use super::super::{CliError, Context};
use crate::logging;

/// `tunneler devslot`: asks the daemon to tunnel the web ports of a devslot
/// instance and open it in the browser, starting the daemon first when
/// needed. Prompts that the connection needs are answered on the terminal
/// while the call runs.
pub async fn devslot(context: &Context, arguments: DevslotArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    ensure_daemon_running(&context.endpoint, &context.locations).await?;
    let mode = ConnectionMode {
        subscribes_to_events: true,
        can_prompt: is_interactive(),
    };
    let mut client = connect(&context.endpoint, mode).await?;
    let events = client.take_events();
    let client = Arc::new(client);
    let prompts = mode
        .can_prompt
        .then(|| LocalPrompts::new(Arc::clone(&client), Arc::new(StandardTerminal)));
    let is_open_requested = !arguments.no_open;
    let call = client.devslot_up(
        arguments.host.clone(),
        arguments.instance.clone(),
        is_open_requested,
    );
    let result = run_while_answering(call, events, prompts)
        .await
        .map_err(|error| {
            devslot_error(error, arguments.host.as_str(), arguments.instance.as_str())
        })?;
    if arguments.json {
        println!("{}", render_devslot_up_json(&result)?);
    } else {
        println!(
            "{}",
            render_devslot_up(&result, is_open_requested, context.style)
        );
    }
    find_outcome_error(&result).map_or(Ok(()), Err)
}

/// Waits for the call while answering the prompts the daemon sends.
async fn run_while_answering<Output>(
    call: impl Future<Output = Result<Output, ClientError>>,
    events: Option<EventReceiver>,
    mut prompts: Option<LocalPrompts>,
) -> Result<Output, ClientError> {
    tokio::pin!(call);
    let Some(mut events) = events else {
        return call.await;
    };
    loop {
        tokio::select! {
            outcome = &mut call => return outcome,
            event = events.recv() => {
                let Some(event) = event else {
                    return Err(ClientError::ConnectionClosed);
                };
                handle_event(prompts.as_mut(), &event);
            }
        }
    }
}

fn handle_event(prompts: Option<&mut LocalPrompts>, event: &EventMessage) {
    if let Some(prompts) = prompts {
        prompts.handle_event(event);
    }
    if let EventMessage::HostKeyRequest(HostKeyRequestEvent::Alert {
        alias,
        reason,
        presented_fingerprint,
    }) = event
    {
        StandardTerminal.write(&describe_host_key_alert(
            alias,
            reason,
            presented_fingerprint,
        ));
    }
}

/// What the daemon's refusal means for the user of this command.
fn devslot_error(error: ClientError, host: &str, instance: &str) -> CliError {
    match error {
        ClientError::Remote {
            code: ErrorCode::DevslotInstanceNotFound,
            ..
        } => CliError::DevslotInstanceNotFound {
            host: host.to_owned(),
            instance: instance.to_owned(),
        },
        ClientError::Remote {
            code: ErrorCode::NotFound,
            message,
            ..
        } => CliError::UnknownHost { message },
        ClientError::Remote {
            code: ErrorCode::RemoteCommandTimeout,
            message,
            ..
        } => CliError::RemoteCommandTimedOut { message },
        other => CliError::Client(other),
    }
}

/// The failure to report once the result is shown: a port that failed or was
/// skipped, else a browser that did not open.
fn find_outcome_error(result: &DevslotUpResultDto) -> Option<CliError> {
    let blocked = result
        .skipped
        .iter()
        .filter(|port| port.reason != DevslotSkipReasonDto::PrivilegedPort)
        .count();
    let problems = blocked + result.failed.len();
    if problems > 0 {
        let total = result.started.len()
            + result.existing.len()
            + result.skipped.len()
            + result.failed.len();
        return Some(CliError::PortsNotTunneled {
            instance: result.instance.clone(),
            count: problems,
            total,
        });
    }
    result
        .open_error
        .as_ref()
        .map(|message| CliError::OpenFailed {
            label: result.instance.clone(),
            message: message.clone(),
        })
}

#[cfg(test)]
mod tests {
    use ipc::protocol::{
        DevslotFailedPortDto, DevslotSkipReasonDto, DevslotSkippedPortDto, DevslotUpPortDto,
    };

    use super::super::super::exit_code;
    use super::*;

    fn remote(code: ErrorCode) -> ClientError {
        ClientError::Remote {
            code,
            message: "boom".to_owned(),
            data: None,
        }
    }

    fn up_port(remote_port: u16) -> DevslotUpPortDto {
        DevslotUpPortDto {
            remote_port,
            listen_port: remote_port,
            tunnel_id: "01J9ZK3Q8M5N7R2T4V6X8Y0ABC".to_owned(),
        }
    }

    fn healthy_result() -> DevslotUpResultDto {
        DevslotUpResultDto {
            host: "vega".to_owned(),
            instance: "web-shop-098a".to_owned(),
            started: vec![up_port(20050)],
            existing: vec![up_port(20051)],
            skipped: Vec::new(),
            failed: Vec::new(),
            removed: Vec::new(),
            needs_review: 0,
            app_url: None,
            app_port_shared: false,
            opened: Some("http://web-shop-098a.localhost:20050/".to_owned()),
            open_error: None,
        }
    }

    fn exit_code_of(result: &DevslotUpResultDto) -> u8 {
        find_outcome_error(result).map_or(exit_code::SUCCESS, |error| error.exit_code())
    }

    #[test]
    fn should_exit_with_0_when_every_port_is_up_and_the_browser_opened() {
        assert_eq!(exit_code_of(&healthy_result()), exit_code::SUCCESS);
    }

    #[test]
    fn should_exit_with_4_when_a_port_failed() {
        let result = DevslotUpResultDto {
            failed: vec![DevslotFailedPortDto {
                remote_port: 20052,
                message: "boom".to_owned(),
            }],
            ..healthy_result()
        };

        assert_eq!(exit_code_of(&result), exit_code::TARGETS_FAILED);
    }

    #[test]
    fn should_exit_with_4_when_a_port_was_skipped_because_it_is_blocked() {
        let result = DevslotUpResultDto {
            skipped: vec![DevslotSkippedPortDto {
                remote_port: 20053,
                reason: DevslotSkipReasonDto::NeedsReview,
            }],
            ..healthy_result()
        };

        assert_eq!(exit_code_of(&result), exit_code::TARGETS_FAILED);
    }

    #[test]
    fn should_exit_with_4_when_a_port_was_skipped_because_the_local_port_is_busy() {
        let result = DevslotUpResultDto {
            skipped: vec![DevslotSkippedPortDto {
                remote_port: 20053,
                reason: DevslotSkipReasonDto::LocalPortBusy,
            }],
            ..healthy_result()
        };

        assert_eq!(exit_code_of(&result), exit_code::TARGETS_FAILED);
    }

    #[test]
    fn should_exit_with_4_when_a_port_was_skipped_because_a_configured_tunnel_holds_it() {
        let result = DevslotUpResultDto {
            skipped: vec![DevslotSkippedPortDto {
                remote_port: 20053,
                reason: DevslotSkipReasonDto::ExistingTunnel,
            }],
            ..healthy_result()
        };

        assert_eq!(exit_code_of(&result), exit_code::TARGETS_FAILED);
    }

    #[test]
    fn should_exit_with_0_when_the_only_skipped_port_is_below_1024() {
        let result = DevslotUpResultDto {
            skipped: vec![DevslotSkippedPortDto {
                remote_port: 80,
                reason: DevslotSkipReasonDto::PrivilegedPort,
            }],
            ..healthy_result()
        };

        assert_eq!(exit_code_of(&result), exit_code::SUCCESS);
    }

    #[test]
    fn should_count_the_ports_that_are_not_tunneled_out_of_all_the_web_ports() {
        let result = DevslotUpResultDto {
            skipped: vec![DevslotSkippedPortDto {
                remote_port: 20053,
                reason: DevslotSkipReasonDto::NeedsReview,
            }],
            ..healthy_result()
        };

        assert!(matches!(
            find_outcome_error(&result),
            Some(CliError::PortsNotTunneled {
                count: 1,
                total: 3,
                ..
            })
        ));
    }

    #[test]
    fn should_exit_with_8_when_the_browser_could_not_be_opened() {
        let result = DevslotUpResultDto {
            opened: None,
            open_error: Some("no browser".to_owned()),
            ..healthy_result()
        };

        assert_eq!(exit_code_of(&result), exit_code::OPEN_FAILED);
    }

    #[test]
    fn should_exit_with_4_rather_than_8_when_a_port_failed_and_the_browser_did_not_open() {
        let result = DevslotUpResultDto {
            failed: vec![DevslotFailedPortDto {
                remote_port: 20052,
                message: "boom".to_owned(),
            }],
            opened: None,
            open_error: Some("no browser".to_owned()),
            ..healthy_result()
        };

        assert_eq!(exit_code_of(&result), exit_code::TARGETS_FAILED);
    }

    #[test]
    fn should_exit_with_0_when_nothing_was_opened_because_it_was_not_asked() {
        let result = DevslotUpResultDto {
            opened: None,
            ..healthy_result()
        };

        assert_eq!(exit_code_of(&result), exit_code::SUCCESS);
    }

    #[test]
    fn should_report_the_instance_as_not_listed_when_the_daemon_says_so() {
        let error = devslot_error(
            remote(ErrorCode::DevslotInstanceNotFound),
            "vega",
            "web-shop-098a",
        );

        assert!(matches!(
            &error,
            CliError::DevslotInstanceNotFound { host, instance }
                if host == "vega" && instance == "web-shop-098a"
        ));
        assert_eq!(error.exit_code(), exit_code::ERROR);
    }

    #[test]
    fn should_report_an_unknown_host_with_the_daemon_message_when_the_alias_is_not_found() {
        let error = devslot_error(remote(ErrorCode::NotFound), "vega", "web-shop-098a");

        assert!(matches!(&error, CliError::UnknownHost { message } if message == "boom"));
        assert_eq!(error.exit_code(), exit_code::ERROR);
    }

    #[test]
    fn should_exit_with_6_when_the_command_on_the_host_timed_out() {
        let error = devslot_error(
            remote(ErrorCode::RemoteCommandTimeout),
            "vega",
            "web-shop-098a",
        );

        assert_eq!(error.exit_code(), exit_code::TIMED_OUT);
    }

    #[test]
    fn should_exit_with_1_when_the_host_cannot_be_reached_or_ds_fails() {
        let codes = [
            ErrorCode::ConnectionFailed,
            ErrorCode::DevslotNotFound,
            ErrorCode::DevslotFailed,
            ErrorCode::DevslotInvalidOutput,
            ErrorCode::Internal,
        ];

        assert_eq!(
            codes.map(|code| devslot_error(remote(code), "vega", "web-shop-098a").exit_code()),
            [exit_code::ERROR; 5]
        );
    }
}
