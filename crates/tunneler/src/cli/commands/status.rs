use super::super::args::OutputArguments;
use super::super::render::{
    render_daemon_summary, render_externals, render_status, render_status_json, render_x11,
};
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
    if let Some(externals) = render_externals(&snapshot, context.style) {
        println!("{externals}");
    }
    if let Some(x11) = render_x11(&snapshot) {
        println!("{x11}");
    }
    Ok(())
}
