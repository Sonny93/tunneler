//! The connection between the front end and the daemon: connect, relay events,
//! notice a lost connection.

use std::ffi::OsString;
use std::process::Command;
use std::sync::Arc;

use ipc::protocol::{ClientKind, EventMessage, Hello};
use ipc::{
    DEFAULT_DAEMON_START_TIMEOUT, Endpoint, EndpointError, EventReceiver, IpcClient,
    default_endpoint, ensure_daemon_running,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::bridge_error::BridgeError;
use crate::connection_slot::{ConnectionSlot, Generation};
use crate::daemon_binary::locate_daemon_binary_from_environment;

const SOCKET_VARIABLE: &str = "TUNNELER_SOCKET";
const DAEMON_EVENT: &str = "daemon-event";
const DAEMON_DISCONNECTED_EVENT: &str = "daemon-disconnected";
const DAEMON_SUBCOMMAND: &str = "daemon";
const DISCONNECTED_MESSAGE: &str = "La connexion au daemon a été perdue.";

#[derive(Serialize, Clone)]
struct DisconnectedPayload {
    message: &'static str,
}

/// The state shared by every command.
#[derive(Default)]
pub struct BridgeState {
    connection_attempts: Mutex<()>,
    connection: Arc<ConnectionSlot<IpcClient>>,
}

impl BridgeState {
    /// The live client, or a `disconnected` error.
    pub fn client(&self) -> Result<Arc<IpcClient>, BridgeError> {
        self.connection
            .current()
            .ok_or_else(BridgeError::disconnected)
    }

    /// Connects to the daemon, starting it when needed; a no-op when already
    /// connected. Attempts are serialized, so two calls never open two
    /// connections.
    pub async fn connect(&self, app: &AppHandle) -> Result<(), BridgeError> {
        let _attempt = self.connection_attempts.lock().await;
        if self.connection.current().is_some() {
            return Ok(());
        }
        let endpoint = resolve_endpoint(std::env::var_os(SOCKET_VARIABLE))?;
        ensure_daemon_running(&endpoint, daemon_command(), DEFAULT_DAEMON_START_TIMEOUT).await?;
        let mut client = IpcClient::connect(&endpoint, hello())
            .await
            .map_err(BridgeError::from_connect_failure)?;
        let events = client
            .take_events()
            .ok_or_else(|| BridgeError::internal("event stream already taken"))?;
        let generation = self.connection.install(Arc::new(client));
        let connection = Arc::clone(&self.connection);
        tauri::async_runtime::spawn(relay_events(app.clone(), events, connection, generation));
        Ok(())
    }
}

fn resolve_endpoint(socket_override: Option<OsString>) -> Result<Endpoint, EndpointError> {
    match socket_override {
        Some(path) => Ok(Endpoint::from_path(path)),
        None => default_endpoint(),
    }
}

fn daemon_command() -> Command {
    let mut command = Command::new(locate_daemon_binary_from_environment());
    command.arg(DAEMON_SUBCOMMAND);
    command
}

fn hello() -> Hello {
    Hello::new(ClientKind::Ui, env!("CARGO_PKG_VERSION"), true, true)
}

async fn relay_events(
    app: AppHandle,
    mut events: EventReceiver,
    connection: Arc<ConnectionSlot<IpcClient>>,
    generation: Generation,
) {
    while let Some(event) = events.recv().await {
        emit_event(&app, &event);
    }
    if connection.clear_if_current(generation) {
        emit_disconnected(&app);
    }
}

fn emit_event(app: &AppHandle, event: &EventMessage) {
    if let Err(error) = app.emit(DAEMON_EVENT, event) {
        tracing::warn!(%error, "cannot forward a daemon event to the front end");
    }
}

fn emit_disconnected(app: &AppHandle) {
    let payload = DisconnectedPayload {
        message: DISCONNECTED_MESSAGE,
    };
    if let Err(error) = app.emit(DAEMON_DISCONNECTED_EVENT, payload) {
        tracing::warn!(%error, "cannot tell the front end the daemon is gone");
    }
}
