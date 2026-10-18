//! How the rest of the daemon talks to the thread that owns the tray icon.

use std::sync::Arc;
use std::thread::JoinHandle;

use super::view::TrayView;

/// What the tray thread is asked to do.
#[derive(Debug)]
pub(super) enum TrayCommand {
    /// Draw this view.
    #[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
    Render(TrayView),
    /// Remove the icon and end the thread.
    Shutdown,
}

/// The tray thread ended, so nothing receives commands any more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TrayThreadGone;

type SendCommand = dyn Fn(TrayCommand) -> Result<(), TrayThreadGone> + Send + Sync;

/// A `Send` way to reach the tray thread, whatever wakes it up on this
/// platform.
#[derive(Clone)]
pub(super) struct TrayCommandSender {
    send: Arc<SendCommand>,
}

impl TrayCommandSender {
    pub(super) fn new(
        send: impl Fn(TrayCommand) -> Result<(), TrayThreadGone> + Send + Sync + 'static,
    ) -> Self {
        Self {
            send: Arc::new(send),
        }
    }

    /// A sender whose thread is already gone.
    pub(super) fn disconnected() -> Self {
        Self::new(|_| Err(TrayThreadGone))
    }

    pub(super) fn send(&self, command: TrayCommand) -> Result<(), TrayThreadGone> {
        (self.send)(command)
    }
}

/// A tray thread that was started.
pub(super) struct RunningTray {
    pub(super) commands: TrayCommandSender,
    /// Absent when there is no thread, for instance on platforms without a tray.
    pub(super) thread: Option<JoinHandle<()>>,
}
