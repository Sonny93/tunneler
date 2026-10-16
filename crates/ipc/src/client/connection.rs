use std::sync::Arc;

use interprocess::local_socket::tokio::Stream;
use tokio::io::{BufReader, ReadHalf};
use tokio::sync::mpsc;

use crate::client::pending::PendingCalls;
use crate::error::ClientError;
use crate::protocol::{
    ErrorCode, EventMessage, HelloResult, LineRead, MAX_LINE_BYTES, ServerFrame, read_line,
};

type Reader = BufReader<ReadHalf<Stream>>;

pub(crate) async fn read_handshake_answer(
    reader: &mut Reader,
    client_protocol_version: u32,
) -> Result<HelloResult, ClientError> {
    let frame = match read_line(reader, MAX_LINE_BYTES).await? {
        LineRead::Line(line) => ServerFrame::parse(&line)?,
        LineRead::EndOfStream => return Err(ClientError::ConnectionClosed),
        LineRead::TooLong => {
            return Err(ClientError::InvalidResponse(
                "the handshake answer is too long".to_owned(),
            ));
        }
    };
    match frame {
        ServerFrame::Response {
            outcome: Ok(result),
            ..
        } => Ok(serde_json::from_value(result)?),
        ServerFrame::Response {
            outcome: Err(error),
            ..
        } if error.code == ErrorCode::ProtocolMismatch => {
            Err(mismatch_from(&error, client_protocol_version))
        }
        ServerFrame::Response {
            outcome: Err(error),
            ..
        } => Err(error.into()),
        ServerFrame::Event(_) => Err(ClientError::InvalidResponse(
            "an event arrived before the handshake answer".to_owned(),
        )),
    }
}

fn mismatch_from(error: &crate::protocol::IpcError, client: u32) -> ClientError {
    let daemon = error
        .data
        .as_ref()
        .and_then(|data| data.get("daemon"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|version| u32::try_from(version).ok());
    match daemon {
        Some(daemon) => ClientError::ProtocolMismatch { daemon, client },
        None => error.clone().into(),
    }
}

pub(crate) async fn run_reader(
    mut reader: Reader,
    pending: Arc<PendingCalls>,
    events: mpsc::UnboundedSender<EventMessage>,
) {
    loop {
        let line = match read_line(&mut reader, MAX_LINE_BYTES).await {
            Ok(LineRead::Line(line)) => line,
            Ok(LineRead::TooLong) => {
                tracing::warn!("the daemon sent a line that is too long");
                break;
            }
            Ok(LineRead::EndOfStream) => break,
            Err(error) => {
                tracing::debug!(%error, "read failed");
                break;
            }
        };
        match ServerFrame::parse(&line) {
            Ok(ServerFrame::Response {
                id: Some(id),
                outcome,
            }) => pending.resolve(id, outcome),
            Ok(ServerFrame::Response { id: None, outcome }) => {
                tracing::warn!(
                    ?outcome,
                    "the daemon rejected a message it could not attribute"
                );
            }
            Ok(ServerFrame::Event(Some(event))) => {
                let _ = events.send(event);
            }
            Ok(ServerFrame::Event(None)) => tracing::debug!("ignoring an unknown event"),
            Err(error) => tracing::warn!(%error, "ignoring an unreadable message"),
        }
    }
    pending.close();
}
