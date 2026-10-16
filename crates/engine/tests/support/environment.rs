use std::path::PathBuf;

use ssh_config::{EnvironmentError, LocalEnvironment};

const LOCAL_USER: &str = "tester";

/// A local machine that owns nothing: resolution never reads the real home.
#[derive(Debug, Clone)]
pub struct FakeEnvironment {
    pub home: PathBuf,
}

impl LocalEnvironment for FakeEnvironment {
    fn variable(&self, _name: &str) -> Option<String> {
        None
    }

    fn user_name(&self) -> String {
        LOCAL_USER.to_owned()
    }

    fn home_directory(&self) -> PathBuf {
        self.home.clone()
    }

    fn host_name(&self) -> String {
        "testbed".to_owned()
    }

    fn user_id(&self) -> u32 {
        1000
    }

    fn system_config_directory(&self) -> PathBuf {
        self.home.join("etc-ssh")
    }

    fn command_succeeds(&self, _command: &str) -> Result<bool, EnvironmentError> {
        Ok(false)
    }
}
