//! Windows: a `Tunneler` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! holding `"<executable>" daemon`.
//!
//! This code is only cross-compiled in development; it has never been run.

use std::io;
use std::path::Path;

use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

use super::{DAEMON_ARGUMENT, LoginLaunch, LoginLaunchError};

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Tunneler";

pub(super) struct SystemLoginLaunch;

impl LoginLaunch for SystemLoginLaunch {
    fn apply(&self, is_enabled: bool) -> Result<(), LoginLaunchError> {
        let (run_key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(RUN_KEY_PATH)
            .map_err(LoginLaunchError::Registry)?;
        if is_enabled {
            let executable = std::env::current_exe().map_err(LoginLaunchError::Executable)?;
            return run_key
                .set_value(VALUE_NAME, &run_command(&executable))
                .map_err(LoginLaunchError::Registry);
        }
        match run_key.delete_value(VALUE_NAME) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => {
                Err(LoginLaunchError::Registry(error))
            }
            _ => Ok(()),
        }
    }
}

fn run_command(executable: &Path) -> String {
    format!("\"{}\" {DAEMON_ARGUMENT}", executable.display())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn should_quote_the_executable_and_pass_the_daemon_argument() {
        let executable = PathBuf::from(r"C:\Program Files\Tunneler\tunneler.exe");

        assert_eq!(
            run_command(&executable),
            r#""C:\Program Files\Tunneler\tunneler.exe" daemon"#
        );
    }
}
