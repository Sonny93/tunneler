use ssh_config::{ConfigSources, LocalEnvironment, ResolvedConfig, resolve};
use thiserror::Error;
use tracing::warn;

/// Why the configuration of a host alias could not be resolved.
#[derive(Debug, Error)]
pub enum HostConfigError {
    /// The `ssh_config` files are unreadable or invalid.
    #[error("cannot resolve the configuration of \"{alias}\": {source}")]
    Resolution {
        /// The alias being resolved.
        alias: String,
        /// The underlying failure.
        source: Box<ssh_config::Error>,
    },
}

/// Where the effective configuration of a host alias comes from.
pub trait HostConfigResolver: Send + Sync {
    /// The configuration `ssh -G <alias>` would report.
    fn resolve(&self, alias: &str) -> Result<ResolvedConfig, HostConfigError>;
}

/// Resolves aliases from `ssh_config` files, logging the warnings of the
/// resolution.
#[derive(Debug)]
pub struct SshConfigResolver<Environment> {
    sources: ConfigSources,
    environment: Environment,
}

impl<Environment> SshConfigResolver<Environment> {
    /// A resolver reading `sources` on the machine described by `environment`.
    pub fn new(sources: ConfigSources, environment: Environment) -> Self {
        Self {
            sources,
            environment,
        }
    }
}

impl<Environment> HostConfigResolver for SshConfigResolver<Environment>
where
    Environment: LocalEnvironment + Send + Sync,
{
    fn resolve(&self, alias: &str) -> Result<ResolvedConfig, HostConfigError> {
        let resolution = resolve(&self.sources, alias, &self.environment).map_err(|source| {
            HostConfigError::Resolution {
                alias: alias.to_owned(),
                source: Box::new(source),
            }
        })?;
        for warning in &resolution.warnings {
            warn!(alias, %warning, "ssh_config warning");
        }
        Ok(resolution.config)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ssh_config::EnvironmentError;

    use super::*;

    struct FixedEnvironment;

    impl LocalEnvironment for FixedEnvironment {
        fn variable(&self, _name: &str) -> Option<String> {
            None
        }

        fn user_name(&self) -> String {
            "alice".to_owned()
        }

        fn home_directory(&self) -> PathBuf {
            PathBuf::from("/home/alice")
        }

        fn host_name(&self) -> String {
            "laptop".to_owned()
        }

        fn user_id(&self) -> u32 {
            1000
        }

        fn system_config_directory(&self) -> PathBuf {
            PathBuf::from("/etc/ssh")
        }

        fn command_succeeds(&self, _command: &str) -> Result<bool, EnvironmentError> {
            Ok(false)
        }
    }

    fn resolver_for(content: &str) -> (tempfile::TempDir, SshConfigResolver<FixedEnvironment>) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config");
        std::fs::write(&path, content).unwrap();
        let resolver = SshConfigResolver::new(ConfigSources::user_only(path), FixedEnvironment);
        (directory, resolver)
    }

    #[test]
    fn should_resolve_the_host_name_when_the_alias_is_configured() {
        let (_directory, resolver) = resolver_for("Host web\n  HostName 198.51.100.10\n  Port 2200\n");
        let config = resolver.resolve("web").unwrap();
        assert_eq!((config.hostname.as_str(), config.port), ("198.51.100.10", 2200));
    }

    #[test]
    fn should_report_the_alias_when_the_configuration_is_invalid() {
        let (_directory, resolver) = resolver_for("Host web\n  Port not-a-port\n");
        assert!(matches!(
            resolver.resolve("web"),
            Err(HostConfigError::Resolution { alias, .. }) if alias == "web"
        ));
    }
}
