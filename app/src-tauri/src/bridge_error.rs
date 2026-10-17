//! The error shape the front end receives: `{ code, message, data? }`.

use ipc::protocol::ErrorCode;
use ipc::{ClientError, EndpointError, LaunchError};
use serde::Serialize;
use serde_json::Value;

const CODE_DISCONNECTED: &str = "disconnected";
const CODE_DAEMON_UNREACHABLE: &str = "daemon_unreachable";
const CODE_PROTOCOL_MISMATCH: &str = "protocol_mismatch";
const CODE_INTERNAL: &str = "internal";
const CODE_INVALID_TARGET: &str = "invalid_target";

/// What a command reports to the front end when it fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BridgeError {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl BridgeError {
    fn new(code: &str, message: String) -> Self {
        Self {
            code: code.to_owned(),
            message,
            data: None,
        }
    }

    /// No live connection to the daemon.
    pub fn disconnected() -> Self {
        Self::new(
            CODE_DISCONNECTED,
            "La connexion au daemon a été perdue.".to_owned(),
        )
    }

    /// A failure of the bridge itself.
    pub fn internal(cause: &str) -> Self {
        Self::new(CODE_INTERNAL, format!("Erreur interne : {cause}"))
    }

    /// The host of a link to open is neither `localhost` nor an IP address.
    pub fn invalid_target(host: &str) -> Self {
        Self::new(
            CODE_INVALID_TARGET,
            format!("Adresse refusée, ni localhost ni adresse IP : {host}"),
        )
    }

    /// The daemon could not be started or reached.
    pub fn daemon_unreachable(cause: &dyn std::fmt::Display) -> Self {
        Self::new(
            CODE_DAEMON_UNREACHABLE,
            format!("Impossible de démarrer le daemon : {cause}"),
        )
    }

    /// Maps the failure of a call made on a live connection.
    pub fn from_call_failure(error: ClientError) -> Self {
        match error {
            ClientError::Remote {
                code,
                message,
                data,
            } => Self::remote(code, message, data),
            ClientError::ConnectionClosed => Self::disconnected(),
            ClientError::ProtocolMismatch { .. } => Self::protocol_mismatch(&error),
            ClientError::DaemonNotRunning => Self::daemon_unreachable(&error),
            ClientError::Io(_) | ClientError::InvalidResponse(_) => {
                Self::internal(&error.to_string())
            }
        }
    }

    /// Maps the failure of connecting to the daemon.
    pub fn from_connect_failure(error: ClientError) -> Self {
        match error {
            ClientError::Remote {
                code,
                message,
                data,
            } => Self::remote(code, message, data),
            ClientError::ProtocolMismatch { .. } => Self::protocol_mismatch(&error),
            ClientError::DaemonNotRunning
            | ClientError::ConnectionClosed
            | ClientError::Io(_)
            | ClientError::InvalidResponse(_) => Self::daemon_unreachable(&error),
        }
    }

    fn remote(code: ErrorCode, message: String, data: Option<Value>) -> Self {
        Self {
            code: code.as_str().to_owned(),
            message,
            data,
        }
    }

    fn protocol_mismatch(error: &ClientError) -> Self {
        Self::new(
            CODE_PROTOCOL_MISMATCH,
            format!("Le daemon et l'interface ne sont pas compatibles : {error}"),
        )
    }
}

impl From<ClientError> for BridgeError {
    fn from(error: ClientError) -> Self {
        Self::from_call_failure(error)
    }
}

impl From<LaunchError> for BridgeError {
    fn from(error: LaunchError) -> Self {
        Self::daemon_unreachable(&error)
    }
}

impl From<EndpointError> for BridgeError {
    fn from(error: EndpointError) -> Self {
        Self::daemon_unreachable(&error)
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::time::Duration;

    use serde_json::json;

    use super::*;

    fn remote_error() -> ClientError {
        ClientError::Remote {
            code: ErrorCode::ValidationFailed,
            message: "invalide".to_owned(),
            data: Some(json!(["nom vide"])),
        }
    }

    #[test]
    fn should_keep_code_message_and_data_when_the_daemon_answered_an_error() {
        let error = BridgeError::from_call_failure(remote_error());

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "code": "validation_failed", "message": "invalide", "data": ["nom vide"] })
        );
    }

    #[test]
    fn should_report_invalid_target_with_the_host_when_the_link_is_refused() {
        let error = BridgeError::invalid_target("example.org");

        assert_eq!(
            serde_json::to_value(error).unwrap()["code"],
            json!("invalid_target")
        );
    }

    #[test]
    fn should_omit_data_when_the_daemon_error_has_none() {
        let error = BridgeError::from_call_failure(ClientError::Remote {
            code: ErrorCode::NotFound,
            message: "absent".to_owned(),
            data: None,
        });

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "code": "not_found", "message": "absent" })
        );
    }

    #[test]
    fn should_report_disconnected_when_the_connection_closed_during_a_call() {
        let error = BridgeError::from_call_failure(ClientError::ConnectionClosed);

        assert_eq!(error.code, "disconnected");
    }

    #[test]
    fn should_report_protocol_mismatch_when_the_versions_differ() {
        let error = BridgeError::from_call_failure(ClientError::ProtocolMismatch {
            daemon: 2,
            client: 1,
        });

        assert_eq!(error.code, "protocol_mismatch");
    }

    #[test]
    fn should_report_internal_when_a_call_fails_on_io() {
        let error = BridgeError::from_call_failure(ClientError::Io(io::Error::other("boom")));

        assert_eq!(error.code, "internal");
    }

    #[test]
    fn should_report_internal_when_the_response_is_unreadable() {
        let error = BridgeError::from_call_failure(ClientError::InvalidResponse("bad".to_owned()));

        assert_eq!(error.code, "internal");
    }

    #[test]
    fn should_report_unreachable_when_a_call_finds_no_daemon() {
        let error = BridgeError::from_call_failure(ClientError::DaemonNotRunning);

        assert_eq!(error.code, "daemon_unreachable");
    }

    #[test]
    fn should_report_unreachable_with_the_cause_when_connecting_fails_on_io() {
        let error = BridgeError::from_connect_failure(ClientError::Io(io::Error::other("boom")));

        assert_eq!(error.code, "daemon_unreachable");
        assert!(error.message.contains("boom"));
    }

    #[test]
    fn should_report_unreachable_when_connecting_finds_no_daemon() {
        let error = BridgeError::from_connect_failure(ClientError::DaemonNotRunning);

        assert_eq!(error.code, "daemon_unreachable");
    }

    #[test]
    fn should_report_protocol_mismatch_when_connecting_to_another_version() {
        let error = BridgeError::from_connect_failure(ClientError::ProtocolMismatch {
            daemon: 2,
            client: 1,
        });

        assert_eq!(error.code, "protocol_mismatch");
    }

    #[test]
    fn should_keep_the_daemon_code_when_connecting_is_refused_remotely() {
        let error = BridgeError::from_connect_failure(remote_error());

        assert_eq!(error.code, "validation_failed");
    }

    #[test]
    fn should_report_unreachable_with_the_cause_when_the_launch_fails() {
        let error = BridgeError::from(LaunchError::TimedOut {
            waited: Duration::from_secs(5),
        });

        assert_eq!(error.code, "daemon_unreachable");
        assert!(error.message.contains("5 seconds"));
    }

    #[test]
    fn should_report_unreachable_when_the_endpoint_is_unknown() {
        let error = BridgeError::from(EndpointError::UnknownUser("nobody".to_owned()));

        assert_eq!(error.code, "daemon_unreachable");
    }
}
