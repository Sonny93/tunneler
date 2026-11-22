//! The connection between the front end and the daemon: connect, relay events,
//! notice a lost connection.

use std::ffi::OsString;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError};
use std::time::Duration;

use ipc::protocol::{ClientKind, DevslotLinkDto, EventMessage, Hello};
use ipc::{
    ClientError, DEFAULT_DAEMON_START_TIMEOUT, Endpoint, EndpointError, EventReceiver, IpcClient,
    default_endpoint, ensure_daemon_running,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::bridge_error::BridgeError;
use crate::connection_slot::{ConnectionSlot, Generation};
use crate::daemon_binary::locate_daemon_binary_from_environment;
use crate::foreground_permission::allow_any_process_to_take_the_foreground;
use crate::startup_claim::{StartupAction, startup_action};
use crate::window_attention::{bring_main_window_to_front, react_to_event};

const SOCKET_VARIABLE: &str = "TUNNELER_SOCKET";
const DAEMON_EVENT: &str = "daemon-event";
const DAEMON_DISCONNECTED_EVENT: &str = "daemon-disconnected";
const DAEMON_SUBCOMMAND: &str = "daemon";
const EXIT_SUCCESS: i32 = 0;
const CLAIM_TIMEOUT: Duration = Duration::from_secs(5);
const DISCONNECTED_MESSAGE: &str = "The connection to the daemon was lost.";

#[derive(Serialize, Clone)]
struct DisconnectedPayload {
    message: &'static str,
}

/// A connection whose events nobody reads yet.
struct PendingRelay {
    events: EventReceiver,
    generation: Generation,
}

/// The state shared by every command.
#[derive(Default)]
pub struct BridgeState {
    connection_attempts: Mutex<()>,
    connection: Arc<ConnectionSlot<IpcClient>>,
    is_startup_settled: AtomicBool,
    is_claim_done: AtomicBool,
    pending_relay: std::sync::Mutex<Option<PendingRelay>>,
    launch_link: std::sync::Mutex<Option<DevslotLinkDto>>,
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
    /// connections. When the connection was opened at launch to relay a link,
    /// this is where its events start flowing to the front end.
    pub async fn connect(&self, app: &AppHandle) -> Result<(), BridgeError> {
        let _attempt = self.connection_attempts.lock().await;
        if self.connection.current().is_some() {
            if let Some(relay) = self.take_pending_relay() {
                self.spawn_relay(app, relay);
            }
            return Ok(());
        }
        let relay = self.open_connection().await?;
        self.spawn_relay(app, relay);
        Ok(())
    }

    /// Connects and claims the UI role with `link` before the main window
    /// exists, so that a refused claim never builds a webview: the daemon
    /// hands the link to the window that is already open. Tells what to do
    /// with the window.
    ///
    /// A granted claim keeps the connection (a UI connection receives the
    /// pending quit and standby confirmations, so it must be the one the
    /// front end uses) and the link, for the front end to take. A connection
    /// that cannot be made shows the window with the link kept: the front end
    /// reports the connection problem.
    pub async fn claim_with_launch_link(&self, link: DevslotLinkDto) -> StartupAction {
        allow_any_process_to_take_the_foreground();
        *lock_ignoring_poison(&self.launch_link) = Some(link.clone());
        if let Err(error) = self.open_launch_connection().await {
            tracing::warn!(?error, "cannot connect to the daemon to relay the link");
            return StartupAction::ShowWindow;
        }
        let claim = self.request_claim(Some(link)).await;
        self.is_claim_done.store(true, Ordering::SeqCst);
        startup_action(claim.as_ref())
    }

    /// The link the app was started with, once: the next call answers `None`.
    pub fn take_launch_link(&self) -> Option<DevslotLinkDto> {
        lock_ignoring_poison(&self.launch_link).take()
    }

    /// Ends the hidden startup of the main window, once: shows it, or exits
    /// when another UI is already open. `connection` is the outcome of the
    /// first connection attempt; a failed one shows the window so the front
    /// end can report it.
    pub async fn settle_startup(&self, app: &AppHandle, connection: &Result<(), BridgeError>) {
        if self.is_startup_settled.swap(true, Ordering::SeqCst) {
            return;
        }
        let action = match connection {
            Ok(()) if self.is_claim_done.load(Ordering::SeqCst) => StartupAction::ShowWindow,
            Ok(()) => startup_action(self.request_claim(None).await.as_ref()),
            Err(_) => StartupAction::ShowWindow,
        };
        apply_startup_action(app, action);
    }

    /// Shows the window when no connection attempt settled the startup in
    /// time, so a stuck front end never leaves the app invisible.
    pub fn reveal_if_unsettled(&self, app: &AppHandle) {
        if self.is_startup_settled.swap(true, Ordering::SeqCst) {
            return;
        }
        apply_startup_action(app, StartupAction::ShowWindow);
    }

    async fn request_claim(
        &self,
        link: Option<DevslotLinkDto>,
    ) -> Option<Result<bool, ClientError>> {
        let client = self.client().ok()?;
        tokio::time::timeout(CLAIM_TIMEOUT, client.claim_ui_with_link(link))
            .await
            .ok()
    }

    async fn open_launch_connection(&self) -> Result<(), BridgeError> {
        let _attempt = self.connection_attempts.lock().await;
        let relay = self.open_connection().await?;
        *lock_ignoring_poison(&self.pending_relay) = Some(relay);
        Ok(())
    }

    /// Opens and installs a connection; the caller holds `connection_attempts`
    /// and decides when its events are read.
    async fn open_connection(&self) -> Result<PendingRelay, BridgeError> {
        let endpoint = resolve_endpoint(std::env::var_os(SOCKET_VARIABLE))?;
        ensure_daemon_running(&endpoint, daemon_command(), DEFAULT_DAEMON_START_TIMEOUT).await?;
        let mut client = IpcClient::connect(&endpoint, hello())
            .await
            .map_err(BridgeError::from_connect_failure)?;
        let events = client
            .take_events()
            .ok_or_else(|| BridgeError::internal("event stream already taken"))?;
        let generation = self.connection.install(Arc::new(client));
        Ok(PendingRelay { events, generation })
    }

    fn take_pending_relay(&self) -> Option<PendingRelay> {
        lock_ignoring_poison(&self.pending_relay).take()
    }

    fn spawn_relay(&self, app: &AppHandle, relay: PendingRelay) {
        let connection = Arc::clone(&self.connection);
        tauri::async_runtime::spawn(relay_events(
            app.clone(),
            relay.events,
            connection,
            relay.generation,
        ));
    }
}

fn lock_ignoring_poison<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn apply_startup_action(app: &AppHandle, action: StartupAction) {
    match action {
        StartupAction::ShowWindow => bring_main_window_to_front(app),
        StartupAction::Exit => app.exit(EXIT_SUCCESS),
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
        react_to_event(&app, &event);
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
