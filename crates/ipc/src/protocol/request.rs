use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::protocol::dto::{
    HostKeyDecisionDto, OpenerSpecDto, RememberDto, SettingsDto, SoundEventDto, StandbySoundDto,
    TemporaryTunnelSpecDto, TunnelDefinitionDto, TunnelSpecDto,
};
use crate::protocol::error::IpcError;
use crate::protocol::hello::Hello;
use crate::protocol::secret::Secret;

const ID_KEY: &str = "id";
const METHOD_KEY: &str = "method";
const PARAMS_KEY: &str = "params";

/// The longest script `import.script` accepts, in bytes.
pub const MAX_IMPORT_SCRIPT_BYTES: usize = 256 * 1024;

/// Every method name the protocol defines, as it appears on the wire.
pub const METHOD_NAMES: [&str; 37] = [
    "hello",
    "state.snapshot",
    "tunnel.create",
    "tunnel.update",
    "tunnel.delete",
    "tunnel.start",
    "tunnel.stop",
    "tunnel.restart",
    "tunnel.logs",
    "tunnel.start_temporary",
    "tunnel.persist",
    "tunnel.check_target",
    "tunnel.open",
    "openers.update",
    "hosts.list",
    "hosts.resolve",
    "import.ssh_config",
    "import.script",
    "tag.start",
    "tag.stop",
    "settings.update",
    "settings.preview_sound",
    "credential.respond",
    "hostkey.respond",
    "daemon.quit",
    "ui.show",
    "external.adopt",
    "external.kill",
    "external.ignore",
    "x11.start",
    "x11.stop",
    "x11.launch",
    "secrets.forget",
    "ui.claim",
    "tags.rename",
    "tags.delete",
    "daemon.paths",
];

/// The parameters of the methods that address one tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelIdParams {
    /// The tunnel.
    pub id: String,
}

/// The parameters of `settings.preview_sound`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSoundParams {
    /// The sound to play.
    pub sound: StandbySoundDto,
    /// What the sound announces.
    pub event: SoundEventDto,
}

/// The parameters of `tunnel.open`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTunnelParams {
    /// The tunnel.
    pub id: String,
    /// `browser` or the id of an opener; absent means the default action of
    /// the tunnel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with: Option<String>,
}

/// The parameters of `openers.update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenersUpdateParams {
    /// The new list of openers of the user.
    pub openers: Vec<OpenerSpecDto>,
}

/// The parameters of the methods that address one process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessParams {
    /// The process identifier.
    pub pid: u32,
}

/// The parameters of `hosts.resolve`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostParams {
    /// The `ssh_config` alias to resolve.
    pub host: String,
}

/// The parameters of `import.script`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportScriptParams {
    /// The pasted script, at most [`MAX_IMPORT_SCRIPT_BYTES`] bytes.
    pub text: String,
}

/// The parameters of the methods that address the X11 anchor of one host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct X11HostParams {
    /// The `ssh_config` alias.
    pub host: String,
}

/// The parameters of `x11.launch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct X11LaunchParams {
    /// The `ssh_config` alias the launcher runs on.
    pub host: String,
    /// The name of the saved launcher.
    pub name: String,
}

/// The parameters of the methods that address one tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagParams {
    /// The tag.
    pub tag: String,
}

/// The parameters of `tags.rename`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRenameParams {
    /// The tag to rename.
    pub from: String,
    /// Its new name.
    pub to: String,
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
    /// How long to keep a passphrase that unlocks its key; absent means not
    /// at all. Ignored for the other kinds of prompts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remember: Option<RememberDto>,
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
    /// Starts a temporary tunnel: it runs in the daemon, is never saved and
    /// is removed when stopped. Answers `validation_failed` when the
    /// definition is invalid.
    #[serde(rename = "tunnel.start_temporary")]
    TunnelStartTemporary(TemporaryTunnelSpecDto),
    /// Saves a temporary tunnel, which keeps running; answers `not_found`
    /// when the tunnel does not exist and `invalid_state` when it is not
    /// temporary.
    #[serde(rename = "tunnel.persist")]
    TunnelPersist(TunnelIdParams),
    /// Checks once whether the target of a running local or remote tunnel
    /// answers, and answers with its health; answers `not_found` when the
    /// tunnel does not exist and `invalid_state` when it is not running or
    /// has no target (dynamic). Never runs on its own: every call opens one
    /// connection to the target.
    #[serde(rename = "tunnel.check_target")]
    TunnelCheckTarget(TunnelIdParams),
    /// Opens a tunnel with its default action (the browser), starting it
    /// first when it is stopped and waiting until it is ready, and answers
    /// with what was opened. Answers `not_found` when the tunnel does not
    /// exist, `invalid_state` when it cannot be opened (remote, dynamic, or
    /// listening on a host name) and `open_failed` when it did not become
    /// ready or the launch failed.
    #[serde(rename = "tunnel.open")]
    TunnelOpen(OpenTunnelParams),
    /// Replaces the openers of the user with one write of the store, and
    /// answers with the openers that can be used afterwards; answers
    /// `validation_failed` when one breaks a rule.
    #[serde(rename = "openers.update")]
    OpenersUpdate(OpenersUpdateParams),
    /// Lists the concrete `Host` aliases of the user's `ssh_config`.
    #[serde(rename = "hosts.list")]
    HostsList {},
    /// Reads the effective configuration of a host alias, as `ssh -G` would.
    #[serde(rename = "hosts.resolve")]
    HostsResolve(HostParams),
    /// Lists the forwards declared in the user's `ssh_config` as import candidates.
    #[serde(rename = "import.ssh_config")]
    ImportSshConfig {},
    /// Reads the forwards of a pasted script as import candidates; answers
    /// `invalid_request` when the text exceeds [`MAX_IMPORT_SCRIPT_BYTES`].
    #[serde(rename = "import.script")]
    ImportScript(ImportScriptParams),
    /// Starts every tunnel carrying a tag.
    #[serde(rename = "tag.start")]
    TagStart(TagParams),
    /// Stops every tunnel carrying a tag.
    #[serde(rename = "tag.stop")]
    TagStop(TagParams),
    /// Replaces the settings.
    #[serde(rename = "settings.update")]
    SettingsUpdate(SettingsDto),
    /// Plays a standby sound now, whatever `standbyNotify` says and without
    /// the wake-failure throttle.
    #[serde(rename = "settings.preview_sound")]
    SettingsPreviewSound(PreviewSoundParams),
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
    /// Terminates the external ssh process providing a tunnel, then starts
    /// every configured tunnel it provided as managed tunnels.
    #[serde(rename = "external.adopt")]
    ExternalAdopt(TunnelIdParams),
    /// Terminates a process the daemon reported: the owner of an `external` or
    /// `conflict` state, or an unmanaged ssh process.
    #[serde(rename = "external.kill")]
    ExternalKill(ProcessParams),
    /// Stops reporting a process for the lifetime of the daemon.
    #[serde(rename = "external.ignore")]
    ExternalIgnore(ProcessParams),
    /// Opens the X11 anchor of a host; does nothing when it is already open.
    #[serde(rename = "x11.start")]
    X11Start(X11HostParams),
    /// Closes the X11 anchor of a host.
    #[serde(rename = "x11.stop")]
    X11Stop(X11HostParams),
    /// Runs a saved launcher in an X11-enabled session on a host; answers
    /// `not_found` when the launcher is unknown.
    #[serde(rename = "x11.launch")]
    X11Launch(X11LaunchParams),
    /// Forgets every remembered passphrase, in memory and in the OS keyring.
    #[serde(rename = "secrets.forget")]
    SecretsForget {},
    /// Asks whether this UI may stay open. When an older UI is already
    /// connected, it is told to show itself and the answer is
    /// `claimed: false`.
    #[serde(rename = "ui.claim")]
    UiClaim {},
    /// Renames a tag in every tunnel and in the launch tags; answers
    /// `not_found` when no tunnel carries `from` and `validation_failed` when
    /// `to` is not a valid tag.
    #[serde(rename = "tags.rename")]
    TagsRename(TagRenameParams),
    /// Removes a tag from every tunnel and from the launch tags, keeping the
    /// tunnels; answers `not_found` when no tunnel carries it.
    #[serde(rename = "tags.delete")]
    TagsDelete(TagParams),
    /// Reads where the daemon keeps its files.
    #[serde(rename = "daemon.paths")]
    DaemonPaths {},
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
    let request = Request::deserialize(Value::Object(tagged)).map_err(|error| {
        IpcError::invalid_request(format!("invalid parameters for `{method}`: {error}"))
    })?;
    validate(&request)?;
    Ok(request)
}

fn validate(request: &Request) -> Result<(), IpcError> {
    match request {
        Request::ImportScript(params) if params.text.len() > MAX_IMPORT_SCRIPT_BYTES => {
            Err(IpcError::invalid_request(format!(
                "invalid parameters for `import.script`: text exceeds {MAX_IMPORT_SCRIPT_BYTES} bytes"
            )))
        }
        _ => Ok(()),
    }
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
