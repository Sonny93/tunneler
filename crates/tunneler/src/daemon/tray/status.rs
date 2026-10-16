//! Whether the tray icon exists and is shown, shared between the tray
//! threads, the snapshot and the `tray.changed` events.

use tokio::sync::watch;

/// Whether the daemon shows its notification-area icon.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum TrayStatus {
    /// The icon is shown.
    Active,
    /// The icon exists but no notification area hosts it.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    MissingHost,
    /// The icon could not be created.
    Failed {
        /// The explanation.
        message: String,
    },
    /// No icon on purpose: disabled, or unsupported on this platform.
    #[default]
    Disabled,
}

impl TrayStatus {
    fn log_change(&self) {
        match self {
            Self::Active => tracing::info!("the tray icon is shown"),
            Self::MissingHost => tracing::warn!(
                "the tray icon is not shown: no notification area hosts it (is a StatusNotifierHost running?)"
            ),
            Self::Failed { message } => {
                tracing::warn!(%message, "the tray icon could not be created");
            }
            Self::Disabled => {}
        }
    }
}

/// The current [`TrayStatus`], settable from any thread.
#[derive(Debug)]
pub(crate) struct TrayStatusCell {
    sender: watch::Sender<TrayStatus>,
}

impl TrayStatusCell {
    pub(crate) fn new(initial: TrayStatus) -> Self {
        Self {
            sender: watch::Sender::new(initial),
        }
    }

    /// Records the status and tells the subscribers, unless it did not change.
    pub(crate) fn set(&self, status: TrayStatus) {
        self.sender.send_if_modified(|current| {
            if *current == status {
                return false;
            }
            status.log_change();
            *current = status;
            true
        });
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<TrayStatus> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_start_with_the_given_status() {
        let cell = TrayStatusCell::new(TrayStatus::MissingHost);

        assert_eq!(*cell.subscribe().borrow(), TrayStatus::MissingHost);
    }

    #[test]
    fn should_notify_subscribers_when_the_status_changes() {
        let cell = TrayStatusCell::new(TrayStatus::Disabled);
        let subscriber = cell.subscribe();

        cell.set(TrayStatus::Active);

        assert!(subscriber.has_changed().unwrap());
    }

    #[test]
    fn should_not_notify_subscribers_when_the_status_is_set_again_unchanged() {
        let cell = TrayStatusCell::new(TrayStatus::Active);
        let subscriber = cell.subscribe();

        cell.set(TrayStatus::Active);

        assert!(!subscriber.has_changed().unwrap());
    }
}
