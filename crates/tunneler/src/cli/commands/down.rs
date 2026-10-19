use super::super::args::DownArguments;
use super::super::session::{ConnectionMode, connect};
use super::super::target::{TargetInterpretation, all_tunnels, resolve_targets};
use super::super::{CliError, Context};
use crate::logging;

/// `tunneler down`: stops tunnels by port, host:port or tag, or all of them.
pub async fn down(context: &Context, arguments: DownArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    let snapshot = client.snapshot().await?;
    let targets = if arguments.all {
        all_tunnels(&snapshot, context.style)
    } else {
        let interpretation = if arguments.tag {
            TargetInterpretation::TagOnly
        } else {
            TargetInterpretation::PortEndpointOrTag
        };
        resolve_targets(&arguments.targets, interpretation, &snapshot, context.style)?
    };
    for target in &targets {
        client.stop_tunnel(&target.id).await?;
        println!("{}: stopping", target.label);
    }
    Ok(())
}
