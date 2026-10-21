use ipc::protocol::TargetHealthDto;

use super::super::args::CheckArguments;
use super::super::render::describe_route;
use super::super::session::{ConnectionMode, connect};
use super::super::style::{Style, Tone};
use super::super::target::find_tunnel;
use super::super::{CliError, Context};
use crate::logging;

/// `tunneler check`: asks a running tunnel to try its target once.
pub async fn check(context: &Context, arguments: &CheckArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    let snapshot = client.snapshot().await?;
    let definition = find_tunnel(&arguments.target, &snapshot)?;
    let health = client.check_target(&definition.id).await?;
    let label = describe_route(definition, &snapshot.settings, context.style);
    report_outcome(&health, label, context.style)
}

fn report_outcome(health: &TargetHealthDto, label: String, style: Style) -> Result<(), CliError> {
    match health {
        TargetHealthDto::Reachable => {
            println!("{label}: {}", style.paint(Tone::Good, "target reachable"));
            Ok(())
        }
        TargetHealthDto::Unreachable { error, .. } => Err(CliError::TargetUnreachable {
            label,
            error: error.clone(),
        }),
        TargetHealthDto::Unknown => Err(CliError::TargetCheckInconclusive { label }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Style = Style {
        color: false,
        unicode: false,
    };

    fn unreachable_health() -> TargetHealthDto {
        TargetHealthDto::Unreachable {
            since: 1,
            error: "connection refused".to_owned(),
        }
    }

    #[test]
    fn should_succeed_when_the_target_is_reachable() {
        assert!(report_outcome(&TargetHealthDto::Reachable, "route".to_owned(), PLAIN).is_ok());
    }

    #[test]
    fn should_fail_with_the_error_when_the_target_is_unreachable() {
        let outcome = report_outcome(&unreachable_health(), "route".to_owned(), PLAIN);

        assert!(matches!(
            outcome,
            Err(CliError::TargetUnreachable { error, .. }) if error == "connection refused"
        ));
    }

    #[test]
    fn should_fail_as_inconclusive_when_the_health_is_unknown() {
        let outcome = report_outcome(&TargetHealthDto::Unknown, "route".to_owned(), PLAIN);

        assert!(matches!(
            outcome,
            Err(CliError::TargetCheckInconclusive { .. })
        ));
    }
}
