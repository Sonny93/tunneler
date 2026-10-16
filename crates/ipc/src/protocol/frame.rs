use serde::Serialize;
use serde_json::Value;

use crate::protocol::error::IpcError;
use crate::protocol::event::EventMessage;
use crate::protocol::line::encode_line;
use crate::protocol::response::ResponsePayload;

const ID_KEY: &str = "id";
const EVENT_KEY: &str = "event";
const RESULT_KEY: &str = "result";
const ERROR_KEY: &str = "error";

/// The outcome of one request.
pub type Outcome = Result<ResponsePayload, IpcError>;

#[derive(Serialize)]
struct ResponseFrame<'a> {
    id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<&'a ResponsePayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a IpcError>,
}

/// Serializes a response as one line. `id` is `None` when the request's
/// identifier could not be read.
///
/// # Errors
///
/// Fails when the payload cannot be represented as JSON.
pub fn encode_response(id: Option<u64>, outcome: &Outcome) -> Result<String, serde_json::Error> {
    let frame = match outcome {
        Ok(payload) => ResponseFrame {
            id,
            result: Some(payload),
            error: None,
        },
        Err(error) => ResponseFrame {
            id,
            result: None,
            error: Some(error),
        },
    };
    encode_line(&frame)
}

/// Serializes an event as one line.
///
/// # Errors
///
/// Fails when the event cannot be represented as JSON.
pub fn encode_event(event: &EventMessage) -> Result<String, serde_json::Error> {
    encode_line(event)
}

/// What the daemon sent, as seen by a client.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerFrame {
    /// The answer to a request.
    Response {
        /// The request's identifier; `None` when the daemon could not read it.
        id: Option<u64>,
        /// The raw `result`, or the error.
        outcome: Result<Value, IpcError>,
    },
    /// An event, or `None` when it is not one this client knows.
    Event(Option<EventMessage>),
}

/// Why a line from the daemon was not understood.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid message from the daemon: {0}")]
pub struct MalformedFrame(String);

impl ServerFrame {
    /// Reads one frame from the bytes of a line.
    ///
    /// # Errors
    ///
    /// Fails when the line is neither a response nor an event.
    pub fn parse(line: &[u8]) -> Result<Self, MalformedFrame> {
        let Value::Object(mut fields) = serde_json::from_slice::<Value>(line)
            .map_err(|error| MalformedFrame(error.to_string()))?
        else {
            return Err(MalformedFrame("not a JSON object".to_owned()));
        };
        if fields.contains_key(EVENT_KEY) {
            let event = serde_json::from_value(Value::Object(fields)).ok();
            return Ok(Self::Event(event));
        }
        let id = fields.remove(ID_KEY).and_then(|id| id.as_u64());
        if let Some(error) = fields.remove(ERROR_KEY) {
            let error =
                serde_json::from_value(error).map_err(|error| MalformedFrame(error.to_string()))?;
            return Ok(Self::Response {
                id,
                outcome: Err(error),
            });
        }
        let Some(result) = fields.remove(RESULT_KEY) else {
            return Err(MalformedFrame("neither `result` nor `error`".to_owned()));
        };
        Ok(Self::Response {
            id,
            outcome: Ok(result),
        })
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;
    use crate::protocol::error::ErrorCode;
    use crate::protocol::{TrayChangedEvent, TrayStatusDto};

    #[test]
    fn should_encode_a_success_with_its_result() {
        let line = encode_response(Some(2), &Ok(ResponsePayload::empty())).unwrap();

        assert_eq!(line, "{\"id\":2,\"result\":{}}\n");
    }

    #[test]
    fn should_encode_an_error_with_a_null_id_when_the_id_is_unknown() {
        let line = encode_response(None, &Err(IpcError::invalid_request("bad"))).unwrap();

        assert_eq!(
            line,
            "{\"id\":null,\"error\":{\"code\":\"invalid_request\",\"message\":\"bad\"}}\n"
        );
    }

    #[test]
    fn should_parse_a_success_response() {
        let frame = ServerFrame::parse(br#"{"id":5,"result":{"a":1}}"#).unwrap();

        assert_eq!(
            frame,
            ServerFrame::Response {
                id: Some(5),
                outcome: Ok(json!({"a": 1}))
            }
        );
    }

    #[test]
    fn should_parse_an_error_response_with_a_null_id() {
        let frame =
            ServerFrame::parse(br#"{"id":null,"error":{"code":"internal","message":"boom"}}"#)
                .unwrap();

        assert_eq!(
            frame,
            ServerFrame::Response {
                id: None,
                outcome: Err(IpcError::new(ErrorCode::Internal, "boom"))
            }
        );
    }

    #[test]
    fn should_parse_an_event() {
        let frame = ServerFrame::parse(br#"{"event":"quit.requested","data":{}}"#).unwrap();

        assert_eq!(
            frame,
            ServerFrame::Event(Some(EventMessage::QuitRequested {}))
        );
    }

    #[test]
    fn should_parse_a_tray_changed_event() {
        let frame = ServerFrame::parse(
            br#"{"event":"tray.changed","data":{"status":{"kind":"missingHost"}}}"#,
        )
        .unwrap();

        assert_eq!(
            frame,
            ServerFrame::Event(Some(EventMessage::TrayChanged(TrayChangedEvent {
                status: TrayStatusDto::MissingHost
            })))
        );
    }

    #[test]
    fn should_parse_a_show_requested_event() {
        let frame = ServerFrame::parse(br#"{"event":"show.requested","data":{}}"#).unwrap();

        assert_eq!(
            frame,
            ServerFrame::Event(Some(EventMessage::ShowRequested {}))
        );
    }

    #[test]
    fn should_parse_an_unknown_event_as_unknown() {
        let frame = ServerFrame::parse(br#"{"event":"from.the.future","data":{}}"#).unwrap();

        assert_eq!(frame, ServerFrame::Event(None));
    }

    #[test]
    fn should_reject_a_frame_without_result_or_error() {
        assert!(ServerFrame::parse(br#"{"id":1}"#).is_err());
    }

    #[test]
    fn should_reject_a_line_that_is_not_json() {
        assert!(ServerFrame::parse(b"nope").is_err());
    }
}
