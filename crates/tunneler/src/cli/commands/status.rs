use super::super::args::OutputArguments;
use super::super::render::{render_daemon_summary, render_status, render_status_json};
use super::super::session::{ConnectionMode, connect};
use super::super::{CliError, Context};
use crate::logging;

/// `tunneler status`: the daemon version and the state of every tunnel.
pub async fn status(context: &Context, arguments: &OutputArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    let snapshot = client.snapshot().await?;
    if arguments.json {
        println!(
            "{}",
            render_status_json(client.daemon_version(), &snapshot)?
        );
        return Ok(());
    }
    println!(
        "{}",
        render_daemon_summary(client.daemon_version(), &snapshot)
    );
    println!("{}", render_status(&snapshot, context.style));
    Ok(())
}
