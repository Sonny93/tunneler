//! The command line: argument parsing, the commands that talk to the daemon,
//! and how their results are shown.

mod args;
mod commands;
mod error;
pub mod exit_code;
mod forward_spec;
mod launcher;
mod locations;
mod progress;
mod prompting;
mod render;
mod session;
mod style;
mod target;
mod terminal;

use ipc::Endpoint;

pub use args::Cli;
use args::Command;
pub use error::CliError;
use locations::Locations;
use style::Style;

/// What every command needs to know about where and how it runs.
pub(crate) struct Context {
    pub(crate) endpoint: Endpoint,
    pub(crate) locations: Locations,
    pub(crate) style: Style,
}

/// Runs the command line.
///
/// # Errors
///
/// Fails with the reason the command could not do its job; see
/// [`CliError::exit_code`] and [`CliError::hint`].
pub async fn run(cli: Cli) -> Result<(), CliError> {
    let context = Context {
        endpoint: cli.locations.endpoint()?,
        style: Style::for_process(cli.color),
        locations: cli.locations,
    };
    match cli.command {
        None => commands::show_or_run_daemon(&context).await,
        Some(Command::Daemon) => commands::daemon(&context).await,
        Some(Command::Up(arguments)) => commands::up(&context, arguments).await,
        Some(Command::Down(arguments)) => commands::down(&context, arguments).await,
        Some(Command::Status(arguments)) => commands::status(&context, &arguments).await,
        Some(Command::List(arguments)) => commands::list(&context, &arguments).await,
        Some(Command::Open(arguments)) => commands::open(&context, &arguments).await,
        Some(Command::Check(arguments)) => commands::check(&context, &arguments).await,
        Some(Command::Quit(arguments)) => commands::quit(&context, &arguments).await,
    }
}
