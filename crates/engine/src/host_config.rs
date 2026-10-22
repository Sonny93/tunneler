use ssh_config::{
    ConfigSources, LocalEnvironment, Resolution, ResolvedConfig, list_hosts, resolve,
};
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
    /// The `ssh_config` files are unreadable or invalid.
    #[error("cannot list the ssh_config hosts: {source}")]
    Listing {
        /// The underlying failure.
        source: Box<ssh_config::Error>,
    },
}

/// Where the effective configuration of a host alias comes from.
pub trait HostConfigResolver: Send + Sync {
    /// The configuration `ssh -G <alias>` would report.
    fn resolve(&self, alias: &str) -> Result<ResolvedConfig, HostConfigError>;

    /// The configuration `ssh -G <alias>` would report, with the warnings found
    /// on the way. Resolvers that collect no warnings return none.
    fn resolve_with_warnings(&self, alias: &str) -> Result<Resolution, HostConfigError> {
        Ok(Resolution {
            config: self.resolve(alias)?,
            warnings: Vec::new(),
        })
    }

    /// The concrete `Host` aliases the configuration declares, without
    /// wildcard patterns.
    fn list_hosts(&self) -> Result<Vec<String>, HostConfigError>;
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
        Ok(self.resolve_with_warnings(alias)?.config)
    }

    fn resolve_with_warnings(&self, alias: &str) -> Result<Resolution, HostConfigError> {
        let resolution = resolve(&self.sources, alias, &self.environment).map_err(|source| {
            HostConfigError::Resolution {
                alias: alias.to_owned(),
                source: Box::new(source),
            }
        })?;
        for warning in &resolution.warnings {
            warn!(alias, %warning, "ssh_config warning");
        }
        Ok(resolution)
    }

    fn list_hosts(&self) -> Result<Vec<String>, HostConfigError> {
        list_hosts(&self.sources, &self.environment).map_err(|source| HostConfigError::Listing {
            source: Box::new(source),
        })
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
        let (_directory, resolver) =
            resolver_for("Host web\n  HostName 198.51.100.10\n  Port 2200\n");
        let config = resolver.resolve("web").unwrap();
        assert_eq!(
            (config.hostname.as_str(), config.port),
            ("198.51.100.10", 2200)
        );
    }

    #[test]
    fn should_return_the_warnings_when_the_configuration_has_unsupported_criteria() {
        let (_directory, resolver) = resolver_for("Match version 9\n  Port 2200\nHost web\n");
        let resolution = resolver.resolve_with_warnings("web").unwrap();
        assert_eq!(resolution.warnings.len(), 1);
    }

    #[test]
    fn should_list_the_concrete_aliases_when_the_configuration_has_wildcards() {
        let (_directory, resolver) = resolver_for(
            "Host web db\n  Port 22\nHost *.internal\n  Port 2200\nHost *\n  User me\n",
        );

        assert_eq!(resolver.list_hosts().unwrap(), vec!["web", "db"]);
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
