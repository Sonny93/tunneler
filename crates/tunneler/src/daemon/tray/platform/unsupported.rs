//! Platforms without a tray: there is no icon and the status is `Disabled`.

use std::io;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use crate::daemon::tray::command::{RunningTray, TrayCommandSender};
use crate::daemon::tray::{TrayAction, TrayStatus, TrayStatusCell, TrayView};

#[allow(
    clippy::needless_pass_by_value,
    clippy::unnecessary_wraps,
    reason = "same signature as the native trays"
)]
pub(in crate::daemon::tray) fn spawn(
    _initial_view: TrayView,
    _actions: mpsc::UnboundedSender<TrayAction>,
    status: Arc<TrayStatusCell>,
    built: oneshot::Sender<Result<(), String>>,
) -> io::Result<RunningTray> {
    tracing::info!("there is no tray icon on this platform");
    status.set(TrayStatus::Disabled);
    let _ = built.send(Ok(()));
    Ok(RunningTray {
        commands: TrayCommandSender::disconnected(),
        thread: None,
    })
}
