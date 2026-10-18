//! The notification-area icon of the daemon.
//!
//! The pure parts ([`tray_view`], [`render_icon`], the menu actions) know
//! nothing about the platform. The icon itself lives on a dedicated thread
//! (see `platform`), because `tray-icon` objects are `!Send`; the controller
//! drives it through a [`TrayDisplay`].

mod command;
mod controller;
#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
mod icon;
#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
mod menu;
#[cfg(any(target_os = "linux", windows))]
mod native;
mod platform;
mod publisher;
mod status;
mod view;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use engine::Engine;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::ui::UiCoordinator;
use command::{RunningTray, TrayCommand, TrayCommandSender};
use controller::{TrayControllerParts, spawn_tray_controller};
pub(crate) use menu::TrayAction;
pub(crate) use publisher::spawn_tray_status_publisher;
pub(crate) use status::{TrayStatus, TrayStatusCell};
pub(crate) use view::{TrayView, tray_view};

/// How long the daemon waits for the tray thread to report whether the icon
/// could be created.
const BUILD_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the daemon waits for the tray thread to remove the icon.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Where views are shown.
pub(crate) trait TrayDisplay: Send + Sync {
    /// Redraws the tray for this view.
    fn show(&self, view: TrayView);
}

/// The running tray: draws views and removes the icon on shutdown.
pub(crate) struct TrayHandle {
    commands: TrayCommandSender,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl TrayHandle {
    fn new(running: RunningTray) -> Self {
        Self {
            commands: running.commands,
            thread: Mutex::new(running.thread),
        }
    }

    fn without_thread() -> Self {
        Self::new(RunningTray {
            commands: TrayCommandSender::disconnected(),
            thread: None,
        })
    }

    fn lock_thread(&self) -> MutexGuard<'_, Option<std::thread::JoinHandle<()>>> {
        self.thread.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Removes the icon and waits, for a bounded time, for the thread to end.
    pub(crate) async fn shutdown(&self) {
        if self.commands.send(TrayCommand::Shutdown).is_err() {
            return;
        }
        let Some(thread) = self.lock_thread().take() else {
            return;
        };
        let joining = tokio::task::spawn_blocking(move || thread.join());
        match tokio::time::timeout(SHUTDOWN_TIMEOUT, joining).await {
            Ok(Ok(Ok(()))) => {}
            Ok(_) => tracing::warn!("the tray thread ended abnormally"),
            Err(_) => tracing::warn!("the tray thread did not stop in time"),
        }
    }
}

impl TrayDisplay for TrayHandle {
    fn show(&self, view: TrayView) {
        if self.commands.send(TrayCommand::Render(view)).is_err() {
            tracing::debug!("the tray thread is gone, the view was dropped");
        }
    }
}

/// Starts the tray showing `initial_view` and waits until it is known whether
/// the icon could be created; `status` is then up to date.
///
/// Menu choices and clicks on the icon are sent to `actions`. A tray that
/// cannot start is reported as [`TrayStatus::Failed`], never as an error.
pub(crate) async fn start_tray(
    initial_view: TrayView,
    actions: mpsc::UnboundedSender<TrayAction>,
    status: Arc<TrayStatusCell>,
) -> TrayHandle {
    let (built_sender, built_receiver) = oneshot::channel();
    let spawned = platform::spawn(initial_view, actions, Arc::clone(&status), built_sender);
    let running = match spawned {
        Ok(running) => running,
        Err(error) => {
            status.set(failed(format!("cannot start the tray thread: {error}")));
            return TrayHandle::without_thread();
        }
    };
    match tokio::time::timeout(BUILD_TIMEOUT, built_receiver).await {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(message))) => status.set(failed(message)),
        Ok(Err(_)) => status.set(failed("the tray thread ended unexpectedly".to_owned())),
        Err(_) => status.set(failed("the tray icon was not created in time".to_owned())),
    }
    TrayHandle::new(running)
}

fn failed(message: String) -> TrayStatus {
    TrayStatus::Failed { message }
}

/// The tray and the controller that drives it.
pub(crate) struct TrayService {
    handle: Arc<TrayHandle>,
    controller: JoinHandle<()>,
}

impl TrayService {
    /// Starts the tray and its controller; `status` tells whether the icon
    /// could be created. The controller ends when `shutdown` is cancelled.
    pub(crate) async fn start(
        engine: &Engine,
        status: Arc<TrayStatusCell>,
        ui: Arc<UiCoordinator>,
        quit_requested: CancellationToken,
        shutdown: CancellationToken,
    ) -> Self {
        let initial_view = tray_view(&engine.snapshot());
        let (actions_sender, actions) = mpsc::unbounded_channel();
        let handle = Arc::new(start_tray(initial_view.clone(), actions_sender, status).await);
        let controller = spawn_tray_controller(
            TrayControllerParts {
                engine: Arc::new(engine.clone()),
                display: handle.clone(),
                actions,
                ui,
                quit_requested,
                shown_view: initial_view,
            },
            shutdown,
        );
        Self { handle, controller }
    }

    /// Removes the icon and waits for the controller, which must have been
    /// told to stop.
    pub(crate) async fn stop(self) {
        self.handle.shutdown().await;
        if let Err(error) = self.controller.await {
            tracing::warn!(%error, "the tray controller ended abnormally");
        }
    }
}
