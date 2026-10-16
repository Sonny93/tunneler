use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use thiserror::Error;

const SHORT_HOST_NAME_SEPARATOR: char = '.';

#[cfg(unix)]
const SYSTEM_CONFIG_DIRECTORY: &str = "/etc/ssh";
#[cfg(unix)]
const DEFAULT_SHELL: &str = "/bin/sh";
#[cfg(unix)]
const SHELL_VARIABLE: &str = "SHELL";
#[cfg(unix)]
const SHELL_COMMAND_FLAG: &str = "-c";

#[cfg(windows)]
const PROGRAM_DATA_VARIABLE: &str = "PROGRAMDATA";
#[cfg(windows)]
const DEFAULT_PROGRAM_DATA: &str = "C:\\ProgramData";
#[cfg(windows)]
const SYSTEM_CONFIG_SUBDIRECTORY: &str = "ssh";

/// Failures reported by a [`LocalEnvironment`].
#[derive(Debug, Error)]
pub enum EnvironmentError {
    /// A fact about the local machine could not be determined.
    #[error("cannot determine the local {what}: {reason}")]
    Unavailable {
        /// What was being looked up.
        what: &'static str,
        /// Why it failed.
        reason: String,
    },
    /// A `Match exec` command could not be started.
    #[error("cannot start the command: {0}")]
    CommandFailedToStart(#[source] io::Error),
    /// A `Match exec` command was killed by a signal.
    #[error("the command did not exit normally")]
    CommandTerminatedAbnormally,
}

/// Everything configuration resolution needs to know about the local machine.
///
/// Resolution only reads the machine through this trait, which keeps it
/// deterministic: tests provide a fake, applications use [`SystemEnvironment`].
pub trait LocalEnvironment {
    /// Value of an environment variable, used for `${VAR}` references.
    fn variable(&self, name: &str) -> Option<String>;

    /// Name of the local user (`%u`, `Match localuser`).
    fn user_name(&self) -> String;

    /// Home directory of the local user (`%d`, `~`).
    fn home_directory(&self) -> PathBuf;

    /// Full local host name (`%l`).
    fn host_name(&self) -> String;

    /// Local host name up to the first dot (`%L`).
    fn short_host_name(&self) -> String {
        let host_name = self.host_name();
        match host_name.split_once(SHORT_HOST_NAME_SEPARATOR) {
            Some((short_name, _)) => short_name.to_owned(),
            None => host_name,
        }
    }

    /// Numeric identifier of the local user (`%i`).
    fn user_id(&self) -> u32;

    /// Directory holding the system-wide configuration, the base of relative
    /// `Include` paths found in the system configuration.
    fn system_config_directory(&self) -> PathBuf;

    /// Runs a `Match exec` command and tells whether it exited with status 0.
    fn command_succeeds(&self, command: &str) -> Result<bool, EnvironmentError>;
}

/// The real machine: process environment, passwd entry, host name and shell.
#[derive(Debug, Clone)]
pub struct SystemEnvironment {
    user_name: String,
    host_name: String,
    home_directory: PathBuf,
}

impl SystemEnvironment {
    /// Reads the user name, host name and home directory once, failing early
    /// when the machine cannot provide them.
    pub fn new() -> Result<Self, EnvironmentError> {
        let user_name = whoami::username().map_err(|error| unavailable("user name", &error))?;
        let host_name = whoami::hostname().map_err(|error| unavailable("host name", &error))?;
        let home_directory = std::env::home_dir().ok_or_else(|| EnvironmentError::Unavailable {
            what: "home directory",
            reason: "no home directory is defined".to_owned(),
        })?;
        Ok(Self {
            user_name,
            host_name,
            home_directory,
        })
    }
}

fn unavailable(what: &'static str, error: &whoami::Error) -> EnvironmentError {
    EnvironmentError::Unavailable {
        what,
        reason: error.to_string(),
    }
}

impl LocalEnvironment for SystemEnvironment {
    fn variable(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn user_name(&self) -> String {
        self.user_name.clone()
    }

    fn home_directory(&self) -> PathBuf {
        self.home_directory.clone()
    }

    fn host_name(&self) -> String {
        self.host_name.clone()
    }

    #[cfg(unix)]
    fn user_id(&self) -> u32 {
        rustix::process::getuid().as_raw()
    }

    #[cfg(not(unix))]
    fn user_id(&self) -> u32 {
        0
    }

    #[cfg(unix)]
    fn system_config_directory(&self) -> PathBuf {
        PathBuf::from(SYSTEM_CONFIG_DIRECTORY)
    }

    #[cfg(windows)]
    fn system_config_directory(&self) -> PathBuf {
        let program_data = self
            .variable(PROGRAM_DATA_VARIABLE)
            .unwrap_or_else(|| DEFAULT_PROGRAM_DATA.to_owned());
        PathBuf::from(program_data).join(SYSTEM_CONFIG_SUBDIRECTORY)
    }

    fn command_succeeds(&self, command: &str) -> Result<bool, EnvironmentError> {
        let status = self
            .shell_command(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(EnvironmentError::CommandFailedToStart)?;
        let exit_code = status
            .code()
            .ok_or(EnvironmentError::CommandTerminatedAbnormally)?;
        Ok(exit_code == 0)
    }
}

impl SystemEnvironment {
    #[cfg(unix)]
    fn shell_command(&self, command: &str) -> Command {
        let shell = self
            .variable(SHELL_VARIABLE)
            .unwrap_or_else(|| DEFAULT_SHELL.to_owned());
        let mut shell_command = Command::new(shell);
        shell_command.arg(SHELL_COMMAND_FLAG).arg(command);
        shell_command
    }

    #[cfg(windows)]
    fn shell_command(&self, command: &str) -> Command {
        let mut shell_command = Command::new("cmd");
        shell_command.arg("/C").arg(command);
        shell_command
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::{EnvironmentError, LocalEnvironment};

    /// Deterministic environment for unit tests.
    pub(crate) struct FakeEnvironment {
        pub variables: BTreeMap<String, String>,
        pub user_name: String,
        pub home_directory: PathBuf,
        pub host_name: String,
        pub user_id: u32,
        pub system_directory: PathBuf,
        pub succeeding_commands: Vec<String>,
        pub executed_commands: RefCell<Vec<String>>,
    }

    impl Default for FakeEnvironment {
        fn default() -> Self {
            Self {
                variables: BTreeMap::new(),
                user_name: "alice".to_owned(),
                home_directory: PathBuf::from("/home/alice"),
                host_name: "laptop.example.org".to_owned(),
                user_id: 1000,
                system_directory: PathBuf::from("/etc/ssh"),
                succeeding_commands: Vec::new(),
                executed_commands: RefCell::new(Vec::new()),
            }
        }
    }

    impl LocalEnvironment for FakeEnvironment {
        fn variable(&self, name: &str) -> Option<String> {
            self.variables.get(name).cloned()
        }

        fn user_name(&self) -> String {
            self.user_name.clone()
        }

        fn home_directory(&self) -> PathBuf {
            self.home_directory.clone()
        }

        fn host_name(&self) -> String {
            self.host_name.clone()
        }

        fn user_id(&self) -> u32 {
            self.user_id
        }

        fn system_config_directory(&self) -> PathBuf {
            self.system_directory.clone()
        }

        fn command_succeeds(&self, command: &str) -> Result<bool, EnvironmentError> {
            self.executed_commands.borrow_mut().push(command.to_owned());
            Ok(self
                .succeeding_commands
                .iter()
                .any(|known| known == command))
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::environment::fake::FakeEnvironment;

    #[test]
    fn should_cut_host_name_at_first_dot_when_computing_short_name() {
        assert_eq!(FakeEnvironment::default().short_host_name(), "laptop");
    }

    #[test]
    fn should_keep_host_name_when_it_has_no_dot() {
        let environment = FakeEnvironment {
            host_name: "laptop".to_owned(),
            ..FakeEnvironment::default()
        };
        assert_eq!(environment.short_host_name(), "laptop");
    }

    #[test]
    fn should_report_success_when_command_exits_with_zero() {
        let environment = SystemEnvironment::new().unwrap();
        assert!(environment.command_succeeds("exit 0").unwrap());
    }

    #[test]
    fn should_report_failure_when_command_exits_with_non_zero_status() {
        let environment = SystemEnvironment::new().unwrap();
        assert!(!environment.command_succeeds("exit 3").unwrap());
    }

    #[test]
    fn should_fail_when_command_is_killed_by_a_signal() {
        let environment = SystemEnvironment::new().unwrap();
        assert!(matches!(
            environment.command_succeeds("kill -9 $$"),
            Err(EnvironmentError::CommandTerminatedAbnormally)
        ));
    }
}
