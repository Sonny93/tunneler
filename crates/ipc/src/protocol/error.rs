use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

/// The closed set of error codes a response can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The message is not valid JSON or does not have the expected shape.
    InvalidRequest,
    /// The method name is not known to the daemon.
    UnknownMethod,
    /// A request other than `hello` came first on the connection.
    HandshakeRequired,
    /// The client and the daemon speak different protocol versions.
    ProtocolMismatch,
    /// The tunnel, tag or other target does not exist.
    NotFound,
    /// The submitted definition is invalid; `data` lists the messages.
    ValidationFailed,
    /// Tunnels are running; `data` is their count.
    TunnelsRunning,
    /// The prompt was already answered or never existed.
    PromptNotPending,
    /// No UI could be reached or started.
    UiUnavailable,
    /// The target is not in a state that allows the operation.
    InvalidState,
    /// The process could not be terminated or is still running.
    KillFailed,
    /// A tunnel could not be opened: it did not become ready or the program
    /// that opens it could not be launched.
    OpenFailed,
    /// The opener does not exist or is not available on this machine; `data`
    /// lists the ids that can be used.
    UnknownOpener,
    /// The daemon failed unexpectedly.
    Internal,
}

impl ErrorCode {
    /// The code as it appears on the wire.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::UnknownMethod => "unknown_method",
            Self::HandshakeRequired => "handshake_required",
            Self::ProtocolMismatch => "protocol_mismatch",
            Self::NotFound => "not_found",
            Self::ValidationFailed => "validation_failed",
            Self::TunnelsRunning => "tunnels_running",
            Self::PromptNotPending => "prompt_not_pending",
            Self::UiUnavailable => "ui_unavailable",
            Self::InvalidState => "invalid_state",
            Self::KillFailed => "kill_failed",
            Self::OpenFailed => "open_failed",
            Self::UnknownOpener => "unknown_opener",
            Self::Internal => "internal",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// An error answered to a request: what handlers return and what the wire
/// carries under `error`.
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
#[error("{code}: {message}")]
pub struct IpcError {
    /// What kind of error it is.
    pub code: ErrorCode,
    /// A human-readable explanation. Never contains secrets.
    pub message: String,
    /// Machine-readable details whose shape depends on the code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl IpcError {
    /// An error without details.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Adds machine-readable details.
    #[must_use]
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// The message is malformed.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidRequest, message)
    }

    /// The method does not exist.
    pub fn unknown_method(method: &str) -> Self {
        Self::new(
            ErrorCode::UnknownMethod,
            format!("unknown method `{method}`"),
        )
    }

    /// A request came before `hello`.
    pub fn handshake_required() -> Self {
        Self::new(
            ErrorCode::HandshakeRequired,
            "the first message must be `hello`",
        )
    }

    /// The versions differ; `data` holds both.
    pub fn protocol_mismatch(daemon: u32, client: u32) -> Self {
        Self::new(
            ErrorCode::ProtocolMismatch,
            format!("the daemon speaks protocol {daemon}, the client protocol {client}"),
        )
        .with_data(json!({ "daemon": daemon, "client": client }))
    }

    /// The target does not exist.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    /// The definition is invalid; `data` lists the problems.
    pub fn validation_failed(messages: &[String]) -> Self {
        Self::new(
            ErrorCode::ValidationFailed,
            "the submitted values are invalid",
        )
        .with_data(json!(messages))
    }

    /// Tunnels are still running; `data` is their count.
    pub fn tunnels_running(count: usize) -> Self {
        Self::new(
            ErrorCode::TunnelsRunning,
            format!("{count} tunnels are still running"),
        )
        .with_data(json!(count))
    }

    /// The prompt is not waiting for an answer.
    pub fn prompt_not_pending(prompt_id: &str) -> Self {
        Self::new(
            ErrorCode::PromptNotPending,
            format!("prompt `{prompt_id}` is not pending"),
        )
    }

    /// The UI cannot be shown.
    pub fn ui_unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::UiUnavailable, message)
    }

    /// The target is not in a state that allows the operation.
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidState, message)
    }

    /// The process could not be terminated or is still running.
    pub fn kill_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::KillFailed, message)
    }

    /// The tunnel did not become ready or the launch failed.
    pub fn open_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::OpenFailed, message)
    }

    /// The opener is unknown or unavailable; `data` lists the usable ids.
    pub fn unknown_opener(message: impl Into<String>, available: &[String]) -> Self {
        Self::new(ErrorCode::UnknownOpener, message).with_data(json!(available))
    }

    /// The daemon failed unexpectedly.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_CODES: [ErrorCode; 14] = [
        ErrorCode::InvalidRequest,
        ErrorCode::UnknownMethod,
        ErrorCode::HandshakeRequired,
        ErrorCode::ProtocolMismatch,
        ErrorCode::NotFound,
        ErrorCode::ValidationFailed,
        ErrorCode::TunnelsRunning,
        ErrorCode::PromptNotPending,
        ErrorCode::UiUnavailable,
        ErrorCode::InvalidState,
        ErrorCode::KillFailed,
        ErrorCode::OpenFailed,
        ErrorCode::UnknownOpener,
        ErrorCode::Internal,
    ];

    #[test]
    fn should_serialize_every_code_as_its_snake_case_name() {
        for code in ALL_CODES {
            assert_eq!(
                serde_json::to_string(&code).unwrap(),
                format!("\"{}\"", code.as_str())
            );
        }
    }

    #[test]
    fn should_round_trip_every_code() {
        for code in ALL_CODES {
            let decoded: ErrorCode =
                serde_json::from_str(&serde_json::to_string(&code).unwrap()).unwrap();
            assert_eq!(decoded, code);
        }
    }

    #[test]
    fn should_reject_an_unknown_code() {
        assert!(serde_json::from_str::<ErrorCode>("\"teapot\"").is_err());
    }

    #[test]
    fn should_carry_both_versions_when_the_protocol_mismatches() {
        let error = IpcError::protocol_mismatch(1, 2);

        assert_eq!(error.data, Some(json!({ "daemon": 1, "client": 2 })));
    }

    #[test]
    fn should_carry_the_messages_when_validation_fails() {
        let error = IpcError::validation_failed(&["port is taken".to_owned()]);

        assert_eq!(error.data, Some(json!(["port is taken"])));
    }

    #[test]
    fn should_carry_the_count_when_tunnels_are_running() {
        assert_eq!(IpcError::tunnels_running(3).data, Some(json!(3)));
    }

    #[test]
    fn should_omit_the_data_when_there_is_none() {
        let error = IpcError::not_found("no such tunnel");

        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            r#"{"code":"not_found","message":"no such tunnel"}"#
        );
    }
}
