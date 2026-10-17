//! Launch at login: keeps the operating system entry that starts the daemon
//! (`tunneler daemon`, tray included, no window) in step with the
//! `launch_at_login` setting.
//!
//! [`spawn_login_launch_follower`] reconciles the entry when the daemon
//! starts, which also refreshes a stale executable path, and whenever the
//! settings change, through `settings.update` or an edit of `tunnels.toml`.
//! A failure is logged and never stops the daemon.

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
use linux::SystemLoginLaunch;
#[cfg(not(any(all(unix, not(target_os = "macos")), windows)))]
mod unsupported;
#[cfg(not(any(all(unix, not(target_os = "macos")), windows)))]
use unsupported::SystemLoginLaunch;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::SystemLoginLaunch;

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use engine::EngineEvent;
use thiserror::Error;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// The argument of the registered command: the daemon, without a window.
const DAEMON_ARGUMENT: &str = "daemon";

/// Why the login entry could not be reconciled.
#[derive(Debug, Error)]
#[allow(
    dead_code,
    reason = "each platform builds its own subset of the variants"
)]
pub(crate) enum LoginLaunchError {
    #[error("cannot find the running executable: {0}")]
    Executable(io::Error),
    #[error("the platform has no user configuration directory")]
    NoConfigDirectory,
    #[error("cannot access {}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
    #[error("{} exists and was not written by Tunneler, leaving it alone", path.display())]
    ForeignEntry { path: PathBuf },
    #[error("cannot update the registry: {0}")]
    Registry(io::Error),
}

/// The operating system entry that starts the daemon at login.
pub(crate) trait LoginLaunch: Send + Sync {
    /// Makes the entry exist, pointing at the running executable, when
    /// `is_enabled`, and removes it otherwise. Does nothing when the entry is
    /// already as wanted.
    fn apply(&self, is_enabled: bool) -> Result<(), LoginLaunchError>;
}

/// The entry of the platform the daemon runs on.
pub(crate) fn system_login_launch() -> Arc<dyn LoginLaunch> {
    Arc::new(SystemLoginLaunch)
}

/// Applies `is_enabled()` to `launch` now and each time the engine reloads
/// its configuration, but only when the value changed since the last success.
/// The task ends when `shutdown` is cancelled or the engine's bus closes.
pub(crate) fn spawn_login_launch_follower(
    mut events: broadcast::Receiver<EngineEvent>,
    is_enabled: impl Fn() -> bool + Send + Sync + 'static,
    launch: Arc<dyn LoginLaunch>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut applied = None;
        reconcile(&is_enabled, &launch, &mut applied).await;
        loop {
            tokio::select! {
                () = shutdown.cancelled() => return,
                event = events.recv() => match event {
                    Ok(EngineEvent::ConfigReloaded) | Err(RecvError::Lagged(_)) => {
                        reconcile(&is_enabled, &launch, &mut applied).await;
                    }
                    Ok(_) => {}
                    Err(RecvError::Closed) => return,
                },
            }
        }
    })
}

async fn reconcile(
    is_enabled: &impl Fn() -> bool,
    launch: &Arc<dyn LoginLaunch>,
    applied: &mut Option<bool>,
) {
    let wanted = is_enabled();
    if *applied == Some(wanted) {
        return;
    }
    let launch = Arc::clone(launch);
    match tokio::task::spawn_blocking(move || launch.apply(wanted)).await {
        Ok(Ok(())) => {
            tracing::info!(is_enabled = wanted, "launch at login is up to date");
            *applied = Some(wanted);
        }
        Ok(Err(error)) => tracing::warn!(%error, "cannot update launch at login"),
        Err(error) => tracing::warn!(%error, "launch at login panicked"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use pretty_assertions::assert_eq;
    use tokio::time::sleep;

    use super::*;

    const SETTLE: Duration = Duration::from_millis(100);

    #[derive(Default)]
    struct RecordingLaunch {
        calls: Mutex<Vec<bool>>,
        is_failing: AtomicBool,
    }

    impl RecordingLaunch {
        fn calls(&self) -> Vec<bool> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl LoginLaunch for RecordingLaunch {
        fn apply(&self, is_enabled: bool) -> Result<(), LoginLaunchError> {
            self.calls.lock().unwrap().push(is_enabled);
            if self.is_failing.load(Ordering::SeqCst) {
                return Err(LoginLaunchError::Executable(io::Error::other("failing")));
            }
            Ok(())
        }
    }

    struct Fixture {
        launch: Arc<RecordingLaunch>,
        setting: Arc<AtomicBool>,
        events: broadcast::Sender<EngineEvent>,
        shutdown: CancellationToken,
        task: JoinHandle<()>,
    }

    fn fixture(is_enabled_at_start: bool) -> Fixture {
        let launch = Arc::new(RecordingLaunch::default());
        let setting = Arc::new(AtomicBool::new(is_enabled_at_start));
        let (events, receiver) = broadcast::channel(8);
        let shutdown = CancellationToken::new();
        let task = spawn_login_launch_follower(
            receiver,
            {
                let setting = Arc::clone(&setting);
                move || setting.load(Ordering::SeqCst)
            },
            launch.clone(),
            shutdown.clone(),
        );
        Fixture {
            launch,
            setting,
            events,
            shutdown,
            task,
        }
    }

    #[tokio::test]
    async fn should_apply_the_setting_when_the_daemon_starts() {
        let fixture = fixture(true);

        sleep(SETTLE).await;

        assert_eq!(fixture.launch.calls(), vec![true]);
    }

    #[tokio::test]
    async fn should_apply_the_new_value_when_the_settings_change() {
        let fixture = fixture(false);
        sleep(SETTLE).await;

        fixture.setting.store(true, Ordering::SeqCst);
        fixture.events.send(EngineEvent::ConfigReloaded).unwrap();
        sleep(SETTLE).await;

        assert_eq!(fixture.launch.calls(), vec![false, true]);
    }

    #[tokio::test]
    async fn should_not_apply_again_when_the_value_did_not_change() {
        let fixture = fixture(true);
        sleep(SETTLE).await;

        fixture.events.send(EngineEvent::ConfigReloaded).unwrap();
        sleep(SETTLE).await;

        assert_eq!(fixture.launch.calls(), vec![true]);
    }

    #[tokio::test]
    async fn should_try_again_at_the_next_reload_when_applying_failed() {
        let fixture = fixture(true);
        fixture.launch.is_failing.store(true, Ordering::SeqCst);
        sleep(SETTLE).await;

        fixture.events.send(EngineEvent::ConfigReloaded).unwrap();
        sleep(SETTLE).await;

        assert_eq!(fixture.launch.calls(), vec![true, true]);
    }

    #[tokio::test]
    async fn should_end_when_shutdown_is_requested() {
        let fixture = fixture(false);

        fixture.shutdown.cancel();

        assert!(fixture.task.await.is_ok());
    }
}
