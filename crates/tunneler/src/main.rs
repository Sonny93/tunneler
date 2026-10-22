//! The `tunneler` program: the daemon and its command line.
#![warn(missing_docs)]

mod cli;
mod daemon;
mod layout;
mod logging;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, CliError, exit_code};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::from(exit_code::SUCCESS),
        Err(error) => {
            report(&error);
            ExitCode::from(error.exit_code())
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(CliError::Runtime)?;
    let outcome = runtime.block_on(cli::run(cli));
    runtime.shutdown_background();
    outcome
}

fn report(error: &CliError) {
    eprintln!("error: {error}");
    if let Some(hint) = error.hint() {
        eprintln!("hint: {hint}");
    }
}
