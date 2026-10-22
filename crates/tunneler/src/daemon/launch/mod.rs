//! Handing an address or a command line to the programs that open it.

#[cfg(test)]
#[cfg(unix)]
pub(crate) mod testing;

use std::io;
#[cfg(all(unix, not(target_os = "macos")))]
use std::io::Write;
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

/// Runs `program` detached from the daemon, with null standard streams and
/// without a shell; the child is reaped from a thread of its own.
pub(crate) fn spawn_detached(program: &str, args: &[String]) -> io::Result<()> {
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

#[cfg(all(unix, not(target_os = "macos")))]
/// Runs `program` detached like [`spawn_detached`], with `input` on its
/// standard input. A thread of its own feeds the program and reaps it, so the
/// caller never blocks on a program that reads slowly.
pub(crate) fn spawn_detached_with_input(
    program: &str,
    args: &[String],
    input: &'static [u8],
) -> io::Result<()> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);
    let mut child = command.spawn()?;
    let program = program.to_owned();
    std::thread::spawn(move || {
        if let Some(mut stdin) = child.stdin.take()
            && let Err(error) = stdin.write_all(input)
            && error.kind() != io::ErrorKind::BrokenPipe
        {
            tracing::warn!(program, %error, "cannot feed a launched program");
        }
        if let Err(error) = child.wait() {
            tracing::warn!(program, %error, "cannot reap a launched program");
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

    #[test]
    fn should_feed_the_input_to_the_program_when_it_reads_its_standard_input() {
        let output = std::env::temp_dir().join(format!("tunneler-input-{}", std::process::id()));
        let script = format!("cat > '{}'", output.display());

        spawn_detached_with_input("sh", &["-c".to_owned(), script], b"sound bytes").unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let received = loop {
            if let Ok(content) = std::fs::read(&output)
                && !content.is_empty()
            {
                break content;
            }
            assert!(std::time::Instant::now() < deadline, "cat never ran");
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let _ = std::fs::remove_file(&output);
        assert_eq!(received, b"sound bytes");
    }

    #[test]
    fn should_report_the_missing_program_when_feeding_one_that_does_not_exist() {
        let error = spawn_detached_with_input("tunneler-no-such-program", &[], b"").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
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
