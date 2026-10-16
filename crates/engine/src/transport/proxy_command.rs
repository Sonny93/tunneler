use std::io;
use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context, Poll};

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader, Join, ReadBuf, join};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tracing::warn;

use crate::error::TransportError;

#[cfg(unix)]
const DEFAULT_SHELL: &str = "/bin/sh";
#[cfg(unix)]
const SHELL_VARIABLE: &str = "SHELL";

/// The standard input and output of a `ProxyCommand` process as one duplex
/// stream. Dropping it kills and reaps the process.
pub(crate) struct ProxyCommandStream {
    stream: Join<ChildStdout, ChildStdin>,
    child: Option<Child>,
}

pub(crate) fn spawn_proxy_command(command: &str) -> Result<ProxyCommandStream, TransportError> {
    let mut child = shell_command(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|source| TransportError::ProxyCommandSpawn {
            command: command.to_owned(),
            source,
        })?;
    let (Some(stdin), Some(stdout), Some(stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        return Err(TransportError::ProxyCommandStdio {
            command: command.to_owned(),
        });
    };
    tokio::spawn(forward_stderr(command.to_owned(), stderr));
    Ok(ProxyCommandStream {
        stream: join(stdout, stdin),
        child: Some(child),
    })
}

#[cfg(unix)]
fn shell_command(command: &str) -> Command {
    let shell = std::env::var(SHELL_VARIABLE).unwrap_or_else(|_| DEFAULT_SHELL.to_owned());
    let mut shell_command = Command::new(shell);
    shell_command.arg("-c").arg(exec_script(command));
    shell_command
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut shell_command = Command::new("cmd");
    shell_command.arg("/C").raw_arg(command);
    shell_command
}

#[cfg(unix)]
fn exec_script(command: &str) -> String {
    format!("exec {command}")
}

async fn forward_stderr(command: String, stderr: ChildStderr) {
    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        warn!(command = %command, "ProxyCommand: {line}");
    }
}

impl Drop for ProxyCommandStream {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        if child.start_kill().is_err() {
            return;
        }
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let _ = child.wait().await;
            });
        }
    }
}

impl AsyncRead for ProxyCommandStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(context, buffer)
    }
}

impl AsyncWrite for ProxyCommandStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(context, data)
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(context)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(context)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use tokio::io::AsyncReadExt;

    use super::*;

    #[test]
    fn should_prefix_the_command_with_exec_when_building_the_script() {
        assert_eq!(exec_script("nc host 22"), "exec nc host 22");
    }

    #[tokio::test]
    async fn should_read_the_command_output_when_the_command_prints() {
        let mut stream = spawn_proxy_command("printf hello").unwrap();
        let mut output = String::new();
        stream.read_to_string(&mut output).await.unwrap();
        assert_eq!(output, "hello");
    }
}
