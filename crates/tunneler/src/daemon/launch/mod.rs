//! Handing an address or a command line to the programs that open it.

#[cfg(test)]
#[cfg(unix)]
pub(crate) mod testing;

use std::io;
use std::process::{Command, Stdio};

use async_trait::async_trait;
use thiserror::Error;

#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// Why something could not be opened.
#[derive(Debug, Error)]
#[error("cannot open {target}: {source}")]
pub(crate) struct LaunchError {
    /// What was to be opened: an address or a program name.
    pub(crate) target: String,
    /// Why it failed.
    #[source]
    pub(crate) source: io::Error,
}

/// Opens things on this machine.
#[async_trait]
pub(crate) trait Launcher: Send + Sync {
    /// Opens `url` with the system default program, without waiting for it.
    async fn open_url(&self, url: &str) -> Result<(), LaunchError>;

    /// Runs `program` with `args` as given, never through a shell, without
    /// waiting for it.
    async fn spawn_program(&self, program: &str, args: &[String]) -> Result<(), LaunchError>;
}

/// Opens things for real, detached from the daemon.
pub(crate) struct SystemLauncher;

#[async_trait]
impl Launcher for SystemLauncher {
    async fn open_url(&self, url: &str) -> Result<(), LaunchError> {
        let target = url.to_owned();
        let outcome = tokio::task::spawn_blocking({
            let target = target.clone();
            move || open::that_detached(target)
        })
        .await
        .unwrap_or_else(|error| Err(io::Error::other(error)));
        outcome.map_err(|source| LaunchError { target, source })
    }

    async fn spawn_program(&self, program: &str, args: &[String]) -> Result<(), LaunchError> {
        spawn_detached(program, args).map_err(|source| LaunchError {
            target: program.to_owned(),
            source,
        })
    }
}

fn spawn_detached(program: &str, args: &[String]) -> io::Result<()> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        if let Err(error) = child.wait() {
            tracing::warn!(%error, "cannot reap a launched program");
        }
    });
    Ok(())
}

#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_run_a_program_that_exists() {
        assert!(SystemLauncher.spawn_program("true", &[]).await.is_ok());
    }

    #[tokio::test]
    async fn should_name_the_program_when_it_cannot_be_started() {
        let error = SystemLauncher
            .spawn_program("tunneler-no-such-program", &[])
            .await
            .unwrap_err();

        assert_eq!(error.target, "tunneler-no-such-program");
    }

    #[tokio::test]
    async fn should_pass_the_arguments_without_a_shell() {
        let marker = std::env::temp_dir().join(format!("tunneler-launch-{}", std::process::id()));
        let argument = format!("{}; echo injected", marker.display());

        SystemLauncher
            .spawn_program("touch", &[argument])
            .await
            .unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let created = loop {
            let path = format!("{}; echo injected", marker.display());
            if std::path::Path::new(&path).exists() {
                break path;
            }
            assert!(std::time::Instant::now() < deadline, "touch never ran");
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let _ = std::fs::remove_file(created);
    }
}
