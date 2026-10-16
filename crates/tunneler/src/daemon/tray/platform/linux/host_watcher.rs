//! Tells whether a notification area is there to show the tray icon.
//!
//! `ksni` assumes a `StatusNotifierHost` exists and never reports otherwise,
//! and `IsStatusNotifierHostRegistered` is true on KDE and GNOME whatever
//! happens. The signal that matters is whether somebody owns the
//! `org.kde.StatusNotifierWatcher` name on the session bus.

use std::io;
use std::sync::Arc;

use zbus::blocking::Connection;
use zbus::blocking::fdo::{DBusProxy, NameOwnerChangedIterator};
use zbus::names::BusName;

use crate::daemon::tray::{TrayStatus, TrayStatusCell};

const STATUS_NOTIFIER_WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHED_NAME_ARGUMENT_INDEX: u8 = 0;
const WATCHER_THREAD_NAME: &str = "tray-host-watcher";

/// Why the notification area could not be followed.
#[derive(Debug, thiserror::Error)]
pub(super) enum HostWatchError {
    #[error(transparent)]
    Bus(#[from] zbus::Error),
    #[error(transparent)]
    Names(#[from] zbus::names::Error),
    #[error("cannot start the watcher thread: {0}")]
    Thread(#[from] io::Error),
}

impl From<zbus::fdo::Error> for HostWatchError {
    fn from(error: zbus::fdo::Error) -> Self {
        Self::Bus(error.into())
    }
}

fn status_for(has_owner: bool) -> TrayStatus {
    if has_owner {
        TrayStatus::Active
    } else {
        TrayStatus::MissingHost
    }
}

/// Sets the status for the current state of the bus, then keeps it up to date
/// from a thread that lives as long as the connection to the bus.
pub(super) fn start(status: Arc<TrayStatusCell>) -> Result<(), HostWatchError> {
    let connection = Connection::session()?;
    let proxy = DBusProxy::new(&connection)?;
    let changes = proxy.receive_name_owner_changed_with_args(&[(
        WATCHED_NAME_ARGUMENT_INDEX,
        STATUS_NOTIFIER_WATCHER,
    )])?;
    let has_owner = proxy.name_has_owner(BusName::try_from(STATUS_NOTIFIER_WATCHER)?)?;
    status.set(status_for(has_owner));
    std::thread::Builder::new()
        .name(WATCHER_THREAD_NAME.to_owned())
        .spawn(move || follow(changes, &status, &connection))?;
    Ok(())
}

fn follow(changes: NameOwnerChangedIterator, status: &TrayStatusCell, _connection: &Connection) {
    for change in changes {
        match change.args() {
            Ok(arguments) => status.set(status_for(arguments.new_owner().is_some())),
            Err(error) => tracing::warn!(%error, "cannot read a bus name change"),
        }
    }
    tracing::debug!("the session bus stopped reporting name changes");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_active_when_the_watcher_has_an_owner() {
        assert_eq!(status_for(true), TrayStatus::Active);
    }

    #[test]
    fn should_report_a_missing_host_when_the_watcher_has_no_owner() {
        assert_eq!(status_for(false), TrayStatus::MissingHost);
    }
}
