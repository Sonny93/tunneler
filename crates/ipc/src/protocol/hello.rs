use serde::{Deserialize, Serialize};

/// The version of the wire protocol this crate speaks.
///
/// It is bumped on every change that an older peer would misread. Client and
/// daemon must agree exactly: the handshake refuses any other version.
pub const PROTOCOL_VERSION: u32 = 2;

/// What kind of program a client is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientKind {
    /// The graphical interface.
    Ui,
    /// The command line.
    Cli,
}

/// The first message of every connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    /// The protocol version the client speaks.
    pub protocol_version: u32,
    /// What kind of client this is.
    pub client_kind: ClientKind,
    /// The version of the client program.
    pub client_version: String,
    /// Whether the daemon should push events to this connection.
    pub subscribe_events: bool,
    /// Whether this client can answer credential and host key prompts.
    pub can_prompt: bool,
}

impl Hello {
    /// A greeting for the current [`PROTOCOL_VERSION`].
    pub fn new(
        client_kind: ClientKind,
        client_version: impl Into<String>,
        subscribe_events: bool,
        can_prompt: bool,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            client_kind,
            client_version: client_version.into(),
            subscribe_events,
            can_prompt,
        }
    }
}

/// The daemon's answer to [`Hello`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelloResult {
    /// The protocol version the daemon speaks.
    pub protocol_version: u32,
    /// The version of the daemon program.
    pub daemon_version: String,
}
