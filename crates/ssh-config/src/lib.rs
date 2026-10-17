//! Lexer, parser and resolution of `ssh_config` files (`Host`, `Match`, `Include`, tokens).
//!
//! The behavior reference is OpenSSH (`ssh -G`). [`resolve`] evaluates the
//! configuration files for an alias and returns a typed [`ResolvedConfig`];
//! [`list_hosts`] lists the aliases a user can connect to; [`parse_file`]
//! exposes the syntax tree of a single file.
//!
//! Everything machine-specific (environment variables, user, host name,
//! `Match exec`) goes through [`LocalEnvironment`], so resolution is
//! deterministic given an environment.
//!
//! # Known gaps
//!
//! - `CanonicalizeHostname` other than `no` and the `Match` criteria
//!   `localnetwork`, `version`, `sessiontype` and `command` are reported as
//!   warnings and never match.
//! - Ports must be numeric (no service names) and `~user/` paths are not
//!   expanded.
//! - File ownership and permission checks are not performed.

#![warn(missing_docs)]

mod ast;
mod criteria;
#[doc(hidden)]
pub mod differential_support;
mod directive;
mod environment;
mod error;
mod include;
mod lexer;
mod pattern;
mod render;
mod resolve;
mod resolved;
mod sources;
mod tokens;

pub use ast::{ConfigFile, ConfigLine, DirectiveLine, LineContent, parse_file};
pub use criteria::{Criterion, CriterionKind};
pub use directive::{
    AddKeysMode, AddKeysToAgent, AddressFamily, CanonicalizeHostname, DynamicForward, Endpoint,
    EnvironmentVariable, ForwardSpec, JumpHost, Keyword, PubkeyAuthentication, RemoteForward,
    StrictHostKeyChecking, parse_dynamic_forward_argument, parse_local_forward_argument,
    parse_remote_forward_argument,
};
pub use environment::{EnvironmentError, LocalEnvironment, SystemEnvironment};
pub use error::{
    CriteriaError, Error, Origin, SyntaxError, TokenError, ValueError, Warning, WarningKind,
};
pub use resolve::{list_hosts, resolve};
pub use resolved::{AgentSocket, ForwardAgent, IdentityAgent, Resolution, ResolvedConfig};
pub use sources::ConfigSources;
