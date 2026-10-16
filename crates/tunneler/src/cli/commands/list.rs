use super::super::args::OutputArguments;
use super::super::render::{render_list, render_list_json};
use super::super::session::{ConnectionMode, connect};
use super::super::{CliError, Context};
use crate::logging;

/// `tunneler list`: the configured tunnels.
pub async fn list(context: &Context, arguments: &OutputArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    let snapshot = client.snapshot().await?;
    if arguments.json {
        println!("{}", render_list_json(&snapshot)?);
        return Ok(());
    }
    println!("{}", render_list(&snapshot, context.style));
    Ok(())
}
