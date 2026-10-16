use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;

use crate::environment::EnvironmentError;

/// A position in a configuration file: path and 1-based line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// The file.
    pub path: Arc<Path>,
    /// The 1-based line number.
    pub line: usize,
}

impl Origin {
    /// An origin at `line` of `path`.
    pub fn new(path: &Path, line: usize) -> Self {
        Self {
            path: Arc::from(path),
            line,
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} line {}", self.path.display(), self.line)
    }
}

/// Everything that can go wrong while parsing or resolving `ssh_config` files.
///
/// Every variant that concerns the content of a file carries the file path and
/// line number through its [`Origin`].
#[allow(
    missing_docs,
    reason = "the error messages describe the variants and their fields"
)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot read {}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },

    #[error("{origin}: {kind}")]
    Syntax { origin: Origin, kind: SyntaxError },

    #[error("{origin}: invalid value for {keyword}: {kind}")]
    Value {
        origin: Origin,
        keyword: String,
        kind: ValueError,
    },

    #[error("{origin}: bad Match condition: {kind}")]
    Criteria { origin: Origin, kind: CriteriaError },

    #[error("{origin}: {source}")]
    Token { origin: Origin, source: TokenError },

    #[error(
        "{origin}: includes nested more than {limit} levels deep: {}",
        format_chain(chain)
    )]
    IncludeDepthExceeded {
        origin: Origin,
        limit: usize,
        chain: Vec<PathBuf>,
    },

    #[error("{origin}: include cycle: {}", format_chain(chain))]
    IncludeCycle { origin: Origin, chain: Vec<PathBuf> },

    #[error("{origin}: bad include path {path}")]
    InvalidIncludePath { origin: Origin, path: String },

    #[error("{origin}: invalid include pattern {pattern}: {source}")]
    InvalidIncludePattern {
        origin: Origin,
        pattern: String,
        source: glob::PatternError,
    },

    #[error("{origin}: cannot run Match exec command `{command}`: {source}")]
    MatchExec {
        origin: Origin,
        command: String,
        source: EnvironmentError,
    },
}

fn format_chain(chain: &[PathBuf]) -> String {
    chain
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ")
}

/// A line that cannot be split into a keyword and its arguments.
#[allow(
    missing_docs,
    reason = "the error messages describe the variants and their fields"
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SyntaxError {
    #[error("no argument after keyword \"{keyword}\"")]
    MissingArguments { keyword: String },
    #[error("invalid quotes")]
    UnbalancedQuotes,
    #[error("Host: empty pattern")]
    EmptyHostPattern,
}

/// A directive whose arguments are not valid for its keyword.
#[allow(
    missing_docs,
    reason = "the error messages describe the variants and their fields"
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValueError {
    #[error("missing argument")]
    MissingArgument,
    #[error("empty argument")]
    EmptyArgument,
    #[error("extra arguments at end of line")]
    ExtraArguments,
    #[error("unsupported value \"{value}\"")]
    UnsupportedValue { value: String },
    #[error("bad port \"{value}\"")]
    InvalidPort { value: String },
    #[error("bad integer \"{value}\"")]
    InvalidInteger { value: String },
    #[error("bad time value \"{value}\"")]
    InvalidTime { value: String },
    #[error("bad forwarding specification \"{value}\"")]
    InvalidForward { value: String },
    #[error("missing forwarding target")]
    MissingForwardTarget,
    #[error("bad ProxyJump \"{value}\"")]
    InvalidProxyJump { value: String },
    #[error("\"none\" must appear alone")]
    NoneMustAppearAlone,
    #[error("invalid environment variable name \"{value}\"")]
    InvalidEnvironmentName { value: String },
    #[error("invalid environment assignment \"{value}\"")]
    InvalidEnvironmentAssignment { value: String },
    #[error("invalid environment expansion in \"{value}\"")]
    InvalidEnvironmentExpansion { value: String },
}

/// A malformed `Match` line.
#[allow(
    missing_docs,
    reason = "the error messages describe the variants and their fields"
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CriteriaError {
    #[error("one or more attributes required")]
    NoCriteria,
    #[error("'{criterion}' cannot be combined with other Match attributes")]
    AllCombinedWithOthers { criterion: String },
    #[error("missing argument for Match '{criterion}'")]
    MissingArgument { criterion: String },
    #[error("missing Match criteria for {criterion}")]
    MissingCriteria { criterion: String },
    #[error("unsupported Match attribute {criterion}")]
    UnknownCriterion { criterion: String },
}

/// A failure while expanding `%` tokens or `${VAR}` references.
#[allow(
    missing_docs,
    reason = "the error messages describe the variants and their fields"
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TokenError {
    #[error("unknown or disallowed token %{token}")]
    UnknownToken { token: char },
    #[error("invalid format: dangling %")]
    DanglingPercent,
    #[error("environment variable reference \"{reference}\" is missing its closing brace")]
    UnterminatedEnvironmentReference { reference: String },
    #[error("zero-length environment variable name")]
    EmptyEnvironmentVariableName,
    #[error("environment variable ${{{name}}} has no value")]
    UndefinedEnvironmentVariable { name: String },
}

/// Something noteworthy that did not prevent the configuration from resolving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// The file the warning is about.
    pub path: PathBuf,
    /// The 1-based line number.
    pub line: usize,
    /// What the warning is about.
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

#[allow(
    missing_docs,
    reason = "the error messages describe the variants and their fields"
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WarningKind {
    #[error("unknown configuration option \"{keyword}\"")]
    UnknownKeyword { keyword: String },
    #[error("Match criterion \"{criterion}\" is not supported and never matches")]
    UnsupportedCriterion { criterion: String },
    #[error("CanonicalizeHostname {value} is not supported and is treated as no")]
    UnsupportedCanonicalization { value: String },
    #[error("Include \"{pattern}\" skipped: {reason}")]
    IncludeSkipped { pattern: String, reason: TokenError },
}
