//! Evaluation of configuration files against an alias, with OpenSSH semantics.

mod finalize;
mod hosts;
mod options;
mod walk;

pub use hosts::list_hosts;

use crate::environment::LocalEnvironment;
use crate::error::Error;
use crate::resolved::Resolution;
use crate::sources::ConfigSources;

pub(crate) const DEFAULT_SSH_PORT: u16 = 22;

/// Resolves the configuration `ssh <alias>` would use.
///
/// The user file is evaluated before the system file; single-valued directives
/// keep their first value and the cumulative ones (`IdentityFile`,
/// `CertificateFile`, the forwards, `SendEnv`) accumulate. When a
/// `Match final` is present the files are evaluated a second time, where
/// `Host` lines match the resolved host name. Defaults and token expansion are
/// then applied, so the result is ready to connect with.
///
/// `CanonicalizeHostname` is not supported: it produces a warning and is
/// treated as `no`.
pub fn resolve(
    sources: &ConfigSources,
    alias: &str,
    environment: &impl LocalEnvironment,
) -> Result<Resolution, Error> {
    let mut walker = walk::Walker::new(environment, alias);
    walker.walk_sources(sources)?;
    let host = walker.settled_host()?;
    if walker.wants_final_pass() {
        walker.begin_final_pass(host.clone());
        walker.walk_sources(sources)?;
    }
    let config = finalize::finalize(&walker.options, alias, &host, environment)?;
    Ok(Resolution {
        config,
        warnings: walker.warnings,
    })
}

#[cfg(test)]
mod tests;
