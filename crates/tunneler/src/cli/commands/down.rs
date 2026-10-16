use ipc::protocol::TunnelDefinitionDto;

use super::super::args::DownArguments;
use super::super::session::{ConnectionMode, connect};
use super::super::target::{TargetInterpretation, all_tunnels, resolve_targets};
use super::super::{CliError, Context};
use crate::logging;

/// `tunneler down`: stops tunnels by name or tag, or all of them.
pub async fn down(context: &Context, arguments: DownArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    let snapshot = client.snapshot().await?;
    let definitions: Vec<TunnelDefinitionDto> = snapshot
        .tunnels
        .into_iter()
        .map(|tunnel| tunnel.definition)
        .collect();
    let targets = if arguments.all {
        all_tunnels(&definitions)
    } else {
        let interpretation = if arguments.tag {
            TargetInterpretation::TagOnly
        } else {
            TargetInterpretation::NameOrTag
        };
        resolve_targets(&arguments.targets, interpretation, &definitions)?
    };
    for target in &targets {
        client.stop_tunnel(&target.id).await?;
        println!("{}: stopping", target.name);
    }
    Ok(())
}
