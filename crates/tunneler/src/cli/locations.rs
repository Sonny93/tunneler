use std::ffi::OsString;
use std::path::PathBuf;

use clap::Args;
use clap::builder::FalseyValueParser;
use engine::{default_state_path, default_store_path};
use ipc::{Endpoint, default_endpoint};

use super::error::CliError;
use crate::daemon::DaemonOptions;

/// Where the daemon's files and socket are; each falls back to a platform
/// default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Args)]
pub struct Locations {
    /// The socket (or named pipe) the daemon listens on
    #[arg(long, global = true, env = "TUNNELER_SOCKET", value_name = "PATH")]
    pub socket: Option<PathBuf>,
    /// The tunnels file
    #[arg(long, global = true, env = "TUNNELER_CONFIG", value_name = "PATH")]
    pub config: Option<PathBuf>,
    /// The file that remembers which tunnels run between two runs
    #[arg(long, global = true, env = "TUNNELER_STATE", value_name = "PATH")]
    pub state: Option<PathBuf>,
    /// Read this `ssh_config` instead of ~/.ssh/config and the system one, like `ssh -F`
    #[arg(
        long = "ssh-config",
        global = true,
        env = "TUNNELER_SSH_CONFIG",
        value_name = "PATH"
    )]
    pub ssh_config: Option<PathBuf>,
    /// Do not show the notification-area icon
    #[arg(
        long = "no-tray",
        global = true,
        env = "TUNNELER_NO_TRAY",
        value_parser = FalseyValueParser::new()
    )]
    pub no_tray: bool,
}

impl Locations {
    /// The endpoint to talk to.
    pub fn endpoint(&self) -> Result<Endpoint, CliError> {
        match &self.socket {
            Some(path) => Ok(Endpoint::from_path(path)),
            None => Ok(default_endpoint()?),
        }
    }

    /// What the daemon needs to run.
    pub fn daemon_options(&self) -> Result<DaemonOptions, CliError> {
        let store_path = match &self.config {
            Some(path) => path.clone(),
            None => default_store_path()?,
        };
        let state_path = match &self.state {
            Some(path) => path.clone(),
            None => default_state_path()?,
        };
        Ok(DaemonOptions {
            endpoint: self.endpoint()?,
            store_path,
            state_path,
            ssh_config: self.ssh_config.clone(),
            is_tray_enabled: !self.no_tray,
            should_open_ui_on_start: false,
        })
    }

    /// The options to pass to a daemon spawned on behalf of this command, so
    /// that it uses the same locations.
    pub fn as_arguments(&self) -> Vec<OsString> {
        let mut arguments: Vec<OsString> = [
            ("--socket", &self.socket),
            ("--config", &self.config),
            ("--state", &self.state),
            ("--ssh-config", &self.ssh_config),
        ]
        .into_iter()
        .filter_map(|(option, path)| path.as_ref().map(|path| (option, path)))
        .flat_map(|(option, path)| [OsString::from(option), path.clone().into_os_string()])
        .collect();
        if self.no_tray {
            arguments.push(OsString::from("--no-tray"));
        }
        arguments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_pass_no_options_when_no_location_is_set() {
        assert_eq!(Locations::default().as_arguments().len(), 0);
    }

    #[test]
    fn should_pass_every_location_that_is_set_with_its_option_name() {
        let locations = Locations {
            socket: Some(PathBuf::from("/run/t.sock")),
            config: None,
            state: Some(PathBuf::from("/home/me/state.toml")),
            ssh_config: Some(PathBuf::from("/home/me/ssh_config")),
            no_tray: false,
        };

        assert_eq!(
            locations.as_arguments(),
            vec![
                OsString::from("--socket"),
                OsString::from("/run/t.sock"),
                OsString::from("--state"),
                OsString::from("/home/me/state.toml"),
                OsString::from("--ssh-config"),
                OsString::from("/home/me/ssh_config"),
            ]
        );
    }

    #[test]
    fn should_pass_no_tray_when_the_tray_is_disabled() {
        let locations = Locations {
            no_tray: true,
            ..Locations::default()
        };

        assert_eq!(locations.as_arguments(), vec![OsString::from("--no-tray")]);
    }

    #[test]
    fn should_disable_the_tray_of_the_daemon_when_no_tray_is_set() {
        let locations = Locations {
            socket: Some(PathBuf::from("/run/t.sock")),
            no_tray: true,
            ..Locations::default()
        };

        assert!(!locations.daemon_options().unwrap().is_tray_enabled);
    }

    #[test]
    fn should_enable_the_tray_of_the_daemon_by_default() {
        let locations = Locations {
            socket: Some(PathBuf::from("/run/t.sock")),
            ..Locations::default()
        };

        assert!(locations.daemon_options().unwrap().is_tray_enabled);
    }

    #[test]
    fn should_use_the_given_socket_when_building_the_endpoint() {
        let locations = Locations {
            socket: Some(PathBuf::from("/run/t.sock")),
            ..Locations::default()
        };

        assert_eq!(
            locations.endpoint().unwrap(),
            Endpoint::from_path("/run/t.sock")
        );
    }

    #[test]
    fn should_use_the_given_state_file_when_building_the_daemon_options() {
        let locations = Locations {
            socket: Some(PathBuf::from("/run/t.sock")),
            state: Some(PathBuf::from("/tmp/state.toml")),
            ..Locations::default()
        };

        assert_eq!(
            locations.daemon_options().unwrap().state_path,
            PathBuf::from("/tmp/state.toml")
        );
    }

    #[test]
    fn should_use_the_given_config_when_building_the_daemon_options() {
        let locations = Locations {
            socket: Some(PathBuf::from("/run/t.sock")),
            config: Some(PathBuf::from("/tmp/tunnels.toml")),
            state: None,
            ssh_config: None,
            no_tray: false,
        };

        assert_eq!(
            locations.daemon_options().unwrap().store_path,
            PathBuf::from("/tmp/tunnels.toml")
        );
    }
}
