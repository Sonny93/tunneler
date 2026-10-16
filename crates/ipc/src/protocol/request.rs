use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::protocol::dto::{HostKeyDecisionDto, SettingsDto, TunnelDefinitionDto, TunnelSpecDto};
use crate::protocol::error::IpcError;
use crate::protocol::hello::Hello;
use crate::protocol::secret::Secret;

const ID_KEY: &str = "id";
const METHOD_KEY: &str = "method";
const PARAMS_KEY: &str = "params";

/// Every method name the protocol defines, as it appears on the wire.
pub const METHOD_NAMES: [&str; 16] = [
    "hello",
    "state.snapshot",
    "tunnel.create",
    "tunnel.update",
    "tunnel.delete",
    "tunnel.start",
    "tunnel.stop",
    "tunnel.restart",
    "tunnel.logs",
    "tag.start",
    "tag.stop",
    "settings.update",
    "credential.respond",
    "hostkey.respond",
    "daemon.quit",
    "ui.show",
];

/// The parameters of the methods that address one tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelIdParams {
    /// The tunnel.
    pub id: String,
}

/// The parameters of the methods that address one tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagParams {
    /// The tag.
    pub tag: String,
}

/// The answer to a credential prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CredentialAnswer {
    /// The passphrase or the password.
    Single(Secret),
    /// The answers to a keyboard-interactive round, one per question.
    Multiple(Vec<Secret>),
}

/// The parameters of `credential.respond`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialResponseParams {
    /// The prompt being answered.
    pub prompt_id: String,
    /// The secret, or `None` when the user cancelled.
    pub answer: Option<CredentialAnswer>,
}

/// The parameters of `hostkey.respond`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyResponseParams {
    /// The prompt being answered.
    pub prompt_id: String,
    /// What to do with the key.
    pub decision: HostKeyDecisionDto,
}

/// The parameters of `daemon.quit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuitParams {
    /// Quit even though tunnels are running.
    pub force: bool,
}

/// A method call, adjacently tagged: `{"method": "...", "params": {...}}`.
///
/// A method without parameters accepts `params` omitted, `null` or `{}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum Request {
    /// The handshake. Answered by the server itself, never given to a handler.
    #[serde(rename = "hello")]
    Hello(Hello),
    /// Reads every tunnel with its live state, and the settings.
    #[serde(rename = "state.snapshot")]
    StateSnapshot {},
    /// Adds a tunnel.
    #[serde(rename = "tunnel.create")]
    TunnelCreate(TunnelSpecDto),
    /// Replaces the definition of a tunnel.
    #[serde(rename = "tunnel.update")]
    TunnelUpdate(TunnelDefinitionDto),
    /// Removes a tunnel.
    #[serde(rename = "tunnel.delete")]
    TunnelDelete(TunnelIdParams),
    /// Starts a tunnel.
    #[serde(rename = "tunnel.start")]
    TunnelStart(TunnelIdParams),
    /// Stops a tunnel.
    #[serde(rename = "tunnel.stop")]
    TunnelStop(TunnelIdParams),
    /// Stops then starts a tunnel.
    #[serde(rename = "tunnel.restart")]
    TunnelRestart(TunnelIdParams),
    /// Reads the recent log of a tunnel.
    #[serde(rename = "tunnel.logs")]
    TunnelLogs(TunnelIdParams),
    /// Starts every tunnel carrying a tag.
    #[serde(rename = "tag.start")]
    TagStart(TagParams),
    /// Stops every tunnel carrying a tag.
    #[serde(rename = "tag.stop")]
    TagStop(TagParams),
    /// Replaces the settings.
    #[serde(rename = "settings.update")]
    SettingsUpdate(SettingsDto),
    /// Answers a `credential.request` event.
    #[serde(rename = "credential.respond")]
    CredentialRespond(CredentialResponseParams),
    /// Answers a `hostkey.request` event.
    #[serde(rename = "hostkey.respond")]
    HostKeyRespond(HostKeyResponseParams),
    /// Asks the daemon to exit.
    #[serde(rename = "daemon.quit")]
    DaemonQuit(QuitParams),
    /// Asks the daemon to show the UI.
    #[serde(rename = "ui.show")]
    UiShow {},
}

/// A [`Request`] and the identifier its response will carry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestEnvelope {
    /// Chosen by the client; echoed by the response.
    pub id: u64,
    /// The call, flattened into the same JSON object.
    #[serde(flatten)]
    pub request: Request,
}

/// A line that could not be turned into a [`RequestEnvelope`].
#[derive(Debug, Clone, PartialEq)]
pub struct RequestRejection {
    /// The request identifier, when it could be read.
    pub id: Option<u64>,
    /// The error to answer.
    pub error: IpcError,
}

impl RequestEnvelope {
    /// Reads one request from the bytes of a line.
    ///
    /// # Errors
    ///
    /// Returns the rejection to answer when the line is not JSON, lacks `id` or
    /// `method`, names an unknown method or carries invalid parameters.
    pub fn parse(line: &[u8]) -> Result<Self, RequestRejection> {
        let Ok(Value::Object(mut fields)) = serde_json::from_slice::<Value>(line) else {
            return Err(reject(None, IpcError::invalid_request("not a JSON object")));
        };
        let Some(id) = fields.get(ID_KEY).and_then(Value::as_u64) else {
            return Err(reject(
                None,
                IpcError::invalid_request("`id` must be an unsigned integer"),
            ));
        };
        let request = parse_request(&mut fields).map_err(|error| reject(Some(id), error))?;
        Ok(Self { id, request })
    }
}

fn reject(id: Option<u64>, error: IpcError) -> RequestRejection {
    RequestRejection { id, error }
}

fn parse_request(fields: &mut Map<String, Value>) -> Result<Request, IpcError> {
    let Some(Value::String(method)) = fields.remove(METHOD_KEY) else {
        return Err(IpcError::invalid_request("`method` must be a string"));
    };
    if !METHOD_NAMES.contains(&method.as_str()) {
        return Err(IpcError::unknown_method(&method));
    }
    let params = match fields.remove(PARAMS_KEY) {
        None | Some(Value::Null) => Value::Object(Map::new()),
        Some(params) => params,
    };
    let tagged = Map::from_iter([
        (METHOD_KEY.to_owned(), Value::String(method.clone())),
        (PARAMS_KEY.to_owned(), params),
    ]);
    Request::deserialize(Value::Object(tagged)).map_err(|error| {
        IpcError::invalid_request(format!("invalid parameters for `{method}`: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::protocol::error::ErrorCode;

    fn parse(line: &str) -> Result<RequestEnvelope, RequestRejection> {
        RequestEnvelope::parse(line.as_bytes())
    }

    #[test]
    fn should_parse_a_method_without_params_when_params_is_omitted() {
        let envelope = parse(r#"{"id":1,"method":"state.snapshot"}"#).unwrap();

        assert_eq!(
            envelope,
            RequestEnvelope {
                id: 1,
                request: Request::StateSnapshot {}
            }
        );
    }

    #[test]
    fn should_parse_a_method_without_params_when_params_is_an_empty_object() {
        let envelope = parse(r#"{"id":1,"method":"ui.show","params":{}}"#).unwrap();

        assert_eq!(envelope.request, Request::UiShow {});
    }

    #[test]
    fn should_parse_a_method_without_params_when_params_is_null() {
        let envelope = parse(r#"{"id":1,"method":"ui.show","params":null}"#).unwrap();

        assert_eq!(envelope.request, Request::UiShow {});
    }

    #[test]
    fn should_parse_params_when_the_method_takes_some() {
        let envelope = parse(r#"{"id":7,"method":"tunnel.start","params":{"id":"abc"}}"#).unwrap();

        assert_eq!(
            envelope.request,
            Request::TunnelStart(TunnelIdParams {
                id: "abc".to_owned()
            })
        );
    }

    #[test]
    fn should_answer_unknown_method_with_the_id_when_the_method_does_not_exist() {
        let rejection = parse(r#"{"id":4,"method":"tunnel.explode"}"#).unwrap_err();

        assert_eq!(
            (rejection.id, rejection.error.code),
            (Some(4), ErrorCode::UnknownMethod)
        );
    }

    #[test]
    fn should_answer_invalid_request_without_an_id_when_the_line_is_not_json() {
        let rejection = parse("{not json").unwrap_err();

        assert_eq!(
            (rejection.id, rejection.error.code),
            (None, ErrorCode::InvalidRequest)
        );
    }

    #[test]
    fn should_answer_invalid_request_without_an_id_when_the_id_is_missing() {
        let rejection = parse(r#"{"method":"ui.show"}"#).unwrap_err();

        assert_eq!(
            (rejection.id, rejection.error.code),
            (None, ErrorCode::InvalidRequest)
        );
    }

    #[test]
    fn should_answer_invalid_request_with_the_id_when_the_params_are_wrong() {
        let rejection = parse(r#"{"id":9,"method":"tunnel.start","params":{"id":3}}"#).unwrap_err();

        assert_eq!(
            (rejection.id, rejection.error.code),
            (Some(9), ErrorCode::InvalidRequest)
        );
    }

    #[test]
    fn should_answer_invalid_request_when_the_line_is_not_utf8() {
        let rejection = RequestEnvelope::parse(&[0xff, 0xfe]).unwrap_err();

        assert_eq!(rejection.error.code, ErrorCode::InvalidRequest);
    }

    #[test]
    fn should_serialize_the_envelope_as_one_flat_object() {
        let envelope = RequestEnvelope {
            id: 3,
            request: Request::DaemonQuit(QuitParams { force: true }),
        };

        assert_eq!(
            serde_json::to_string(&envelope).unwrap(),
            r#"{"id":3,"method":"daemon.quit","params":{"force":true}}"#
        );
    }

    #[test]
    fn should_round_trip_the_envelope_through_parse() {
        let envelope = RequestEnvelope {
            id: 12,
            request: Request::TagStop(TagParams {
                tag: "dev".to_owned(),
            }),
        };
        let line = serde_json::to_vec(&envelope).unwrap();

        assert_eq!(RequestEnvelope::parse(&line).unwrap(), envelope);
    }
}
