use std::path::{Path, PathBuf};

use crate::environment::LocalEnvironment;

const USER_CONFIG_DIRECTORY: &str = ".ssh";
const USER_CONFIG_FILE: &str = "config";
const SYSTEM_CONFIG_FILE: &str = "ssh_config";

/// The configuration files to read, in evaluation order: user, then system.
///
/// A file that does not exist is treated as empty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigSources {
    /// The user configuration, evaluated first.
    pub user: Option<PathBuf>,
    /// The system-wide configuration, evaluated last.
    pub system: Option<PathBuf>,
}

impl ConfigSources {
    /// The files `ssh` reads by default: `~/.ssh/config` then the system
    /// `ssh_config` (`/etc/ssh`, or `%PROGRAMDATA%\ssh` on Windows).
    pub fn platform_default(environment: &impl LocalEnvironment) -> Self {
        let user = environment
            .home_directory()
            .join(USER_CONFIG_DIRECTORY)
            .join(USER_CONFIG_FILE);
        let system = environment
            .system_config_directory()
            .join(SYSTEM_CONFIG_FILE);
        Self {
            user: Some(user),
            system: Some(system),
        }
    }

    /// Only this user file and no system file, like `ssh -F path`.
    pub fn user_only(path: impl AsRef<Path>) -> Self {
        Self {
            user: Some(path.as_ref().to_path_buf()),
            system: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::fake::FakeEnvironment;

    #[test]
    fn should_point_to_home_and_system_directory_when_using_platform_default() {
        let sources = ConfigSources::platform_default(&FakeEnvironment::default());
        assert_eq!(
            sources,
            ConfigSources {
                user: Some(PathBuf::from("/home/alice/.ssh/config")),
                system: Some(PathBuf::from("/etc/ssh/ssh_config")),
            }
        );
    }

    #[test]
    fn should_have_no_system_file_when_using_user_only() {
        assert_eq!(ConfigSources::user_only("/tmp/c").system, None);
    }

    #[test]
    fn should_keep_given_path_when_using_user_only() {
        assert_eq!(
            ConfigSources::user_only("/tmp/c").user,
            Some(PathBuf::from("/tmp/c"))
        );
    }
}
