//! What a session channel tells the client, shared by the X11 commands and
//! the remote commands.

use russh::client::Msg;
use russh::{Channel, ChannelMsg};

const STDERR_EXTENDED_DATA: u32 = 1;

/// What a session channel tells the client.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ChannelEvent {
    Accepted,
    Refused,
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    ExitStatus(u32),
    /// The command was ended by a signal, so it has no exit status.
    ExitSignal,
    /// The server will send no more data.
    Eof,
    Closed,
}

/// Waits for the next message of the channel that matters to a command.
pub(crate) async fn next_channel_event(channel: &mut Channel<Msg>) -> ChannelEvent {
    loop {
        let Some(message) = channel.wait().await else {
            return ChannelEvent::Closed;
        };
        if let Some(event) = event_of(message) {
            return event;
        }
    }
}

fn event_of(message: ChannelMsg) -> Option<ChannelEvent> {
    match message {
        ChannelMsg::Success => Some(ChannelEvent::Accepted),
        ChannelMsg::Failure => Some(ChannelEvent::Refused),
        ChannelMsg::Data { data } => Some(ChannelEvent::Stdout(data.to_vec())),
        ChannelMsg::ExtendedData { data, ext } if ext == STDERR_EXTENDED_DATA => {
            Some(ChannelEvent::Stderr(data.to_vec()))
        }
        ChannelMsg::ExitStatus { exit_status } => Some(ChannelEvent::ExitStatus(exit_status)),
        ChannelMsg::ExitSignal { .. } => Some(ChannelEvent::ExitSignal),
        ChannelMsg::Eof => Some(ChannelEvent::Eof),
        ChannelMsg::Close => Some(ChannelEvent::Closed),
        _ => None,
    }
}
