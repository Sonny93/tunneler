//! Log output.
//!
//! The daemon logs to daily rolling files and, when attached to a terminal, to
//! stderr. The CLI subcommands stay silent unless `TUNNELER_LOG` is set.

use std::io::{self, IsTerminal};
use std::path::PathBuf;

use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{InitError, RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::layout::{DataLayout, LayoutError};

/// The environment variable holding the log filter directives.
pub const LOG_FILTER_VARIABLE: &str = "TUNNELER_LOG";

/// How many daily log files are kept.
pub const MAX_LOG_FILES: usize = 7;

const LOG_FILE_PREFIX: &str = "tunneler";
const LOG_FILE_SUFFIX: &str = "log";

#[derive(Debug, Error)]
enum LogFileError {
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error("cannot create the log file in {}: {source}", directory.display())]
    Appender {
        directory: PathBuf,
        source: InitError,
    },
}

/// Keeps the log file writer alive; dropping it flushes pending lines.
pub struct LogGuard {
    _file_writer: Option<WorkerGuard>,
}

fn filter_from(directives: Option<&str>) -> EnvFilter {
    EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy(directives.unwrap_or_default())
}

fn open_log_file()
-> Result<(tracing_appender::non_blocking::NonBlocking, WorkerGuard), LogFileError> {
    let directory = DataLayout::resolve()?.log_directory;
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix(LOG_FILE_SUFFIX)
        .max_log_files(MAX_LOG_FILES)
        .build(&directory)
        .map_err(|source| LogFileError::Appender { directory, source })?;
    Ok(tracing_appender::non_blocking(appender))
}

/// Sets up the daemon's logging: rolling files, plus stderr on a terminal.
///
/// The filter comes from `TUNNELER_LOG` and defaults to `info`. A log file that
/// cannot be opened is reported and the daemon carries on without it.
pub fn init_daemon_logging() -> LogGuard {
    let filter = filter_from(std::env::var(LOG_FILTER_VARIABLE).ok().as_deref());
    let (file_layer, guard, file_error) = match open_log_file() {
        Ok((writer, guard)) => {
            let layer = tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false);
            (Some(layer), Some(guard), None)
        }
        Err(error) => (None, None, Some(error)),
    };
    let stderr_layer = io::stderr()
        .is_terminal()
        .then(|| tracing_subscriber::fmt::layer().with_writer(io::stderr));
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();
    if let Some(error) = file_error {
        tracing::warn!(%error, "logging to a file is disabled");
    }
    LogGuard {
        _file_writer: guard,
    }
}

/// Sets up the CLI's logging: stderr only, and only when `TUNNELER_LOG` is set
/// to something.
pub fn init_cli_logging() {
    let Ok(directives) = std::env::var(LOG_FILTER_VARIABLE) else {
        return;
    };
    if directives.is_empty() {
        return;
    }
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter_from(Some(&directives)))
        .with_writer(io::stderr)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_log_at_info_when_no_directives_are_given() {
        assert_eq!(filter_from(None).to_string(), "info");
    }

    #[test]
    fn should_use_the_given_directives_when_they_are_valid() {
        assert_eq!(filter_from(Some("debug")).to_string(), "debug");
    }

    #[test]
    fn should_fall_back_to_info_when_the_directives_are_invalid() {
        assert_eq!(filter_from(Some("=,")).to_string(), "info");
    }
}
