use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::validation::ValidationError;

/// A position in a text file, 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    /// The 1-based line.
    pub line: usize,
    /// The 1-based column, counted in characters.
    pub column: usize,
}

/// Why the store could not be read, written or watched.
#[derive(Debug, Error)]
pub enum StoreError {
    /// A filesystem operation failed.
    #[error("cannot access {}: {source}", path.display())]
    Io {
        /// The file or directory involved.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The file is not valid TOML or does not match the expected layout; this
    /// includes unknown keys.
    #[error("{}", describe_parse_failure(path, *location, message))]
    Parse {
        /// The file that failed to parse.
        path: PathBuf,
        /// Where the problem is, when known.
        location: Option<SourceLocation>,
        /// What is wrong.
        message: String,
    },
    /// The file was written by a newer version of the app and is left
    /// untouched.
    #[error(
        "the file uses format version {found} but this version of Tunneler only supports up to version {supported}; it was left untouched"
    )]
    UnsupportedVersion {
        /// The version found in the file.
        found: u32,
        /// The newest version this build understands.
        supported: u32,
    },
    /// The content is well-formed but breaks a rule.
    #[error("the tunnels file is invalid:\n{}", describe_validation_failures(.0))]
    Validation(Vec<ValidationError>),
    /// The file watcher failed.
    #[error("cannot watch {}: {source}", path.display())]
    Watch {
        /// The directory being watched.
        path: PathBuf,
        /// The underlying error.
        source: notify::Error,
    },
    /// The platform has no user configuration directory.
    #[error("cannot determine the user configuration directory")]
    ConfigDirectoryUnavailable,
}

fn describe_parse_failure(path: &Path, location: Option<SourceLocation>, message: &str) -> String {
    match location {
        Some(SourceLocation { line, column }) => {
            format!("{}:{line}:{column}: {message}", path.display())
        }
        None => format!("{}: {message}", path.display()),
    }
}

fn describe_validation_failures(failures: &[ValidationError]) -> String {
    failures
        .iter()
        .map(|failure| format!("- {failure}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn io_error(path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn should_include_line_and_column_when_parse_error_has_a_location() {
        let error = StoreError::Parse {
            path: PathBuf::from("tunnels.toml"),
            location: Some(SourceLocation { line: 4, column: 2 }),
            message: "unknown field `lisen_port`".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "tunnels.toml:4:2: unknown field `lisen_port`"
        );
    }

    #[test]
    fn should_omit_location_when_parse_error_has_none() {
        let error = StoreError::Parse {
            path: PathBuf::from("tunnels.toml"),
            location: None,
            message: "file is not valid UTF-8".to_owned(),
        };

        assert_eq!(error.to_string(), "tunnels.toml: file is not valid UTF-8");
    }
}
