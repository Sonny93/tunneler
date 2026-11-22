//! Runs one command on a host over an open SSH session and collects what it
//! prints, within fixed limits.

use std::time::Duration;

use thiserror::Error;
use tokio::time::timeout;

use crate::channel_event::{ChannelEvent, next_channel_event};
use crate::session::SshSession;

/// The most a command may print on its standard output, in bytes.
pub(crate) const MAX_STDOUT_BYTES: usize = 1024 * 1024;

/// How much of the end of the standard error is kept, in bytes.
pub(crate) const STDERR_TAIL_BYTES: usize = 8 * 1024;

/// Why a remote command produced no output to read.
#[derive(Debug, Error)]
pub(crate) enum RemoteCommandError {
    /// The session channel could not be driven.
    #[error("the session channel failed: {0}")]
    Channel(Box<russh::Error>),
    /// The server refused to run the command.
    #[error("the server refused to run the command")]
    Refused,
    /// The command did not end in time.
    #[error("the command did not end within {0:?}")]
    Timeout(Duration),
    /// The command printed more than [`MAX_STDOUT_BYTES`].
    #[error("the command printed more than {MAX_STDOUT_BYTES} bytes")]
    StdoutTooLarge,
}

/// What a remote command printed and how it ended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommandOutput {
    /// The standard output, at most [`MAX_STDOUT_BYTES`].
    pub(crate) stdout: Vec<u8>,
    /// The last [`STDERR_TAIL_BYTES`] of the standard error.
    pub(crate) stderr_tail: Vec<u8>,
    /// The exit status; `None` when the command was ended by a signal or the
    /// channel closed without reporting one.
    pub(crate) exit_status: Option<u32>,
}

#[derive(Default)]
struct Collector {
    stdout: Vec<u8>,
    stderr_tail: Vec<u8>,
    exit_status: Option<u32>,
}

impl Collector {
    fn push_stdout(&mut self, chunk: &[u8]) -> Result<(), RemoteCommandError> {
        if self.stdout.len() + chunk.len() > MAX_STDOUT_BYTES {
            return Err(RemoteCommandError::StdoutTooLarge);
        }
        self.stdout.extend_from_slice(chunk);
        Ok(())
    }

    fn push_stderr(&mut self, chunk: &[u8]) {
        self.stderr_tail.extend_from_slice(chunk);
        let excess = self.stderr_tail.len().saturating_sub(STDERR_TAIL_BYTES);
        self.stderr_tail.drain(..excess);
    }

    fn finish(self) -> CommandOutput {
        CommandOutput {
            stdout: self.stdout,
            stderr_tail: self.stderr_tail,
            exit_status: self.exit_status,
        }
    }
}

/// Opens a session channel on `session`, runs `command` in it and collects
/// its output, all within one `time_limit`. The channel, when it was opened,
/// is closed whatever the outcome, so that a command that outlives the limit
/// is not left running on it.
pub(crate) async fn run_remote_command(
    session: &SshSession,
    command: &str,
    time_limit: Duration,
) -> Result<CommandOutput, RemoteCommandError> {
    let mut opened = None;
    let outcome = timeout(time_limit, async {
        let channel = opened.insert(
            session
                .open_session_channel()
                .await
                .map_err(|error| RemoteCommandError::Channel(Box::new(error)))?,
        );
        collect_output(channel, command).await
    })
    .await;
    if let Some(channel) = opened {
        let _ = channel.close().await;
    }
    outcome.unwrap_or(Err(RemoteCommandError::Timeout(time_limit)))
}

async fn collect_output(
    channel: &mut russh::Channel<russh::client::Msg>,
    command: &str,
) -> Result<CommandOutput, RemoteCommandError> {
    channel
        .exec(true, command)
        .await
        .map_err(|error| RemoteCommandError::Channel(Box::new(error)))?;
    let mut collector = Collector::default();
    loop {
        match next_channel_event(channel).await {
            ChannelEvent::Refused => return Err(RemoteCommandError::Refused),
            ChannelEvent::Stdout(chunk) => collector.push_stdout(&chunk)?,
            ChannelEvent::Stderr(chunk) => collector.push_stderr(&chunk),
            ChannelEvent::ExitStatus(status) => collector.exit_status = Some(status),
            ChannelEvent::Closed => return Ok(collector.finish()),
            ChannelEvent::Accepted | ChannelEvent::ExitSignal | ChannelEvent::Eof => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_the_end_of_the_standard_error_when_it_exceeds_the_tail() {
        let mut collector = Collector::default();
        collector.push_stderr(&vec![b'a'; STDERR_TAIL_BYTES]);
        collector.push_stderr(b"tail");

        assert_eq!(collector.stderr_tail.len(), STDERR_TAIL_BYTES);
        assert!(collector.stderr_tail.ends_with(b"tail"));
    }

    #[test]
    fn should_refuse_the_output_when_it_exceeds_the_limit() {
        let mut collector = Collector::default();
        collector
            .push_stdout(&vec![b'a'; MAX_STDOUT_BYTES])
            .unwrap();

        assert!(matches!(
            collector.push_stdout(b"a"),
            Err(RemoteCommandError::StdoutTooLarge)
        ));
    }

    #[test]
    fn should_accept_the_output_when_it_is_exactly_the_limit() {
        let mut collector = Collector::default();

        assert!(collector.push_stdout(&vec![b'a'; MAX_STDOUT_BYTES]).is_ok());
    }
}
