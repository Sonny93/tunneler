use std::time::Duration;

use russh::client::Msg;
use russh::{Channel, ChannelMsg};
use tokio::time::timeout;

use super::cookie::MIT_MAGIC_COOKIE_PROTOCOL;
use super::error::X11Error;
use super::router::X11Forwarding;

/// Prints the `DISPLAY` the server set up, then keeps the channel open.
pub(crate) const ANCHOR_COMMAND: &str = r#"echo "$DISPLAY"; exec sleep 2147483647"#;

const X11_SCREEN_NUMBER: u32 = 0;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const DISPLAY_TIMEOUT: Duration = Duration::from_secs(15);
const STDERR_EXTENDED_DATA: u32 = 1;

/// What a session channel tells the client.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ChannelEvent {
    Accepted,
    Refused,
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    ExitStatus(u32),
    Closed,
}

/// Waits for the next message of the channel that matters to a command.
pub(crate) async fn next_channel_event(channel: &mut Channel<Msg>) -> ChannelEvent {
    loop {
        let Some(message) = channel.wait().await else {
            return ChannelEvent::Closed;
        };
        match message {
            ChannelMsg::Success => return ChannelEvent::Accepted,
            ChannelMsg::Failure => return ChannelEvent::Refused,
            ChannelMsg::Data { data } => return ChannelEvent::Stdout(data.to_vec()),
            ChannelMsg::ExtendedData { data, ext } if ext == STDERR_EXTENDED_DATA => {
                return ChannelEvent::Stderr(data.to_vec());
            }
            ChannelMsg::ExitStatus { exit_status } => return ChannelEvent::ExitStatus(exit_status),
            ChannelMsg::Close => return ChannelEvent::Closed,
            _ => {}
        }
    }
}

/// Asks the server to forward X11 connections to the session, giving X
/// clients the fake cookie of the session.
pub(crate) async fn request_x11_forwarding(
    channel: &mut Channel<Msg>,
    forwarding: &X11Forwarding,
) -> Result<(), X11Error> {
    channel
        .request_x11(
            true,
            false,
            MIT_MAGIC_COOKIE_PROTOCOL,
            forwarding.fake_cookie.to_hex(),
            X11_SCREEN_NUMBER,
        )
        .await
        .map_err(|error| X11Error::Channel(Box::new(error)))?;
    await_acceptance(channel, X11Error::Refused).await
}

/// Starts `command` on the channel and returns once the server accepted it.
pub(crate) async fn start_command(
    channel: &mut Channel<Msg>,
    command: &str,
) -> Result<(), X11Error> {
    channel
        .exec(true, command)
        .await
        .map_err(|error| X11Error::Channel(Box::new(error)))?;
    await_acceptance(channel, X11Error::CommandRefused).await
}

async fn await_acceptance(channel: &mut Channel<Msg>, refusal: X11Error) -> Result<(), X11Error> {
    let waiting = async {
        loop {
            match next_channel_event(channel).await {
                ChannelEvent::Accepted => return Ok(()),
                ChannelEvent::Refused => return Err(refusal),
                ChannelEvent::Closed => return Err(X11Error::ChannelClosed),
                ChannelEvent::Stdout(_) | ChannelEvent::Stderr(_) | ChannelEvent::ExitStatus(_) => {
                }
            }
        }
    };
    timeout(REQUEST_TIMEOUT, waiting)
        .await
        .unwrap_or(Err(X11Error::RequestTimeout(REQUEST_TIMEOUT)))
}

/// Reads the first line the anchor command prints: the remote `DISPLAY`.
pub(crate) async fn read_display_line(channel: &mut Channel<Msg>) -> Result<String, X11Error> {
    let reading = async {
        let mut output = Vec::new();
        loop {
            match next_channel_event(channel).await {
                ChannelEvent::Stdout(chunk) => output.extend(chunk),
                ChannelEvent::Closed => return Err(X11Error::ChannelClosed),
                ChannelEvent::Accepted
                | ChannelEvent::Refused
                | ChannelEvent::Stderr(_)
                | ChannelEvent::ExitStatus(_) => {}
            }
            if let Some(line) = first_line(&output) {
                return Ok(line);
            }
        }
    };
    let display = timeout(DISPLAY_TIMEOUT, reading)
        .await
        .unwrap_or(Err(X11Error::RemoteDisplayTimeout(DISPLAY_TIMEOUT)))?;
    if display.is_empty() {
        return Err(X11Error::EmptyRemoteDisplay);
    }
    Ok(display)
}

fn first_line(output: &[u8]) -> Option<String> {
    let end = output.iter().position(|byte| *byte == b'\n')?;
    Some(String::from_utf8_lossy(&output[..end]).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_nothing_when_the_line_is_not_finished() {
        assert_eq!(first_line(b"localhost:10"), None);
    }

    #[test]
    fn should_return_the_line_without_its_terminator_when_it_is_finished() {
        assert_eq!(
            first_line(b"localhost:10.0\nmore"),
            Some("localhost:10.0".to_owned())
        );
    }

    #[test]
    fn should_trim_the_carriage_return_when_the_line_ends_with_crlf() {
        assert_eq!(
            first_line(b"localhost:10.0\r\n"),
            Some("localhost:10.0".to_owned())
        );
    }

    #[test]
    fn should_return_an_empty_line_when_only_the_terminator_arrived() {
        assert_eq!(first_line(b"\n"), Some(String::new()));
    }
}
