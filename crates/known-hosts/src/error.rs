use std::fmt;
use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::key::HostKeyError;

/// Everything that can go wrong while reading or writing `known_hosts` files.
///
/// A missing file is not an error when reading, and malformed lines are
/// reported as [`Warning`]s, never as errors.
#[allow(
    missing_docs,
    reason = "the error messages describe the variants and their fields"
)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot access {}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },

    #[error("host name \"{name}\" cannot be recorded in a known_hosts file")]
    InvalidHostName { name: String },

    #[error("cannot generate a random salt: {source}")]
    Entropy { source: getrandom::Error },
}

/// A line that was skipped because it is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// The file the warning is about.
    pub path: PathBuf,
    /// The 1-based line number.
    pub line: usize,
    /// What is wrong with the line.
    pub kind: WarningKind,
}

impl fmt::Display for Warning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} line {}: {}",
            self.path.display(),
            self.line,
            self.kind
        )
    }
}

/// Why a line was skipped.
#[allow(
    missing_docs,
    reason = "the error messages describe the variants and their fields"
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WarningKind {
    #[error("unknown marker \"{marker}\"")]
    UnknownMarker { marker: String },
    #[error("only one marker is allowed")]
    DuplicateMarker,
    #[error("missing {field}")]
    MissingField { field: &'static str },
    #[error("malformed hashed host entry")]
    InvalidHostHash,
    #[error("invalid key: {source}")]
    InvalidKey { source: HostKeyError },
}
