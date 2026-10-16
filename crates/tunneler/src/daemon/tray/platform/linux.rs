//! Linux: the icon is a `StatusNotifierItem` served over D-Bus by a worker
//! thread of `ksni`, so the tray thread only builds it and then applies
//! commands; no event loop is needed.

mod host_watcher;

use std::io;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

use tokio::sync::{mpsc, oneshot};

use super::TRAY_THREAD_NAME;
use crate::daemon::tray::command::{RunningTray, TrayCommand, TrayCommandSender, TrayThreadGone};
use crate::daemon::tray::native::{NativeTray, forward_events};
use crate::daemon::tray::{TrayAction, TrayStatus, TrayStatusCell, TrayView};

pub(in crate::daemon::tray) fn spawn(
    initial_view: TrayView,
    actions: mpsc::UnboundedSender<TrayAction>,
    status: Arc<TrayStatusCell>,
    built: oneshot::Sender<Result<(), String>>,
) -> io::Result<RunningTray> {
    let (sender, receiver) = channel();
    let thread = std::thread::Builder::new()
        .name(TRAY_THREAD_NAME.to_owned())
        .spawn(move || run(&initial_view, actions, &status, built, &receiver))?;
    Ok(RunningTray {
        commands: TrayCommandSender::new(move |command| {
            sender.send(command).map_err(|_| TrayThreadGone)
        }),
        thread: Some(thread),
    })
}

fn run(
    initial_view: &TrayView,
    actions: mpsc::UnboundedSender<TrayAction>,
    status: &Arc<TrayStatusCell>,
    built: oneshot::Sender<Result<(), String>>,
    commands: &Receiver<TrayCommand>,
) {
    forward_events(actions);
    let tray = match NativeTray::build(initial_view) {
        Ok(tray) => tray,
        Err(message) => {
            let _ = built.send(Err(message));
            return;
        }
    };
    watch_host(status);
    let _ = built.send(Ok(()));
    apply_commands(&tray, commands);
}

/// Follows the notification area for the lifetime of the daemon. When it
/// cannot be followed the icon is assumed to be shown.
fn watch_host(status: &Arc<TrayStatusCell>) {
    if let Err(error) = host_watcher::start(Arc::clone(status)) {
        tracing::warn!(%error, "cannot tell whether a notification area shows the tray icon");
        status.set(TrayStatus::Active);
    }
}

fn apply_commands(tray: &NativeTray, commands: &Receiver<TrayCommand>) {
    while let Ok(command) = commands.recv() {
        match command {
            TrayCommand::Render(view) => tray.show(&view),
            TrayCommand::Shutdown => return,
        }
    }
}
