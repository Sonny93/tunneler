//! Tells the connected clients when the tray status changes.

use ipc::protocol::{EventMessage, TrayChangedEvent};
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::TrayStatus;
use crate::daemon::mapping::tray_status_to_dto;

/// Publishes a `tray.changed` event for every change of `status`. The task
/// ends when `shutdown` is cancelled.
pub(crate) fn spawn_tray_status_publisher(
    mut status: watch::Receiver<TrayStatus>,
    events: broadcast::Sender<EventMessage>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                () = shutdown.cancelled() => return,
                changed = status.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    let dto = tray_status_to_dto(&status.borrow_and_update());
                    let _ = events.send(EventMessage::TrayChanged(TrayChangedEvent { status: dto }));
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ipc::protocol::TrayStatusDto;
    use tokio::time::timeout;

    use super::*;
    use crate::daemon::tray::TrayStatusCell;

    const WAIT: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn should_publish_the_new_status_when_it_changes() {
        let cell = TrayStatusCell::new(TrayStatus::Disabled);
        let (events, mut published) = broadcast::channel(8);
        spawn_tray_status_publisher(cell.subscribe(), events, CancellationToken::new());

        cell.set(TrayStatus::MissingHost);

        let event = timeout(WAIT, published.recv()).await.unwrap().unwrap();
        assert_eq!(
            event,
            EventMessage::TrayChanged(TrayChangedEvent {
                status: TrayStatusDto::MissingHost
            })
        );
    }

    #[tokio::test]
    async fn should_stop_when_the_shutdown_token_is_cancelled() {
        let cell = TrayStatusCell::new(TrayStatus::Disabled);
        let shutdown = CancellationToken::new();
        let publisher = spawn_tray_status_publisher(
            cell.subscribe(),
            broadcast::channel(8).0,
            shutdown.clone(),
        );

        shutdown.cancel();

        assert!(timeout(WAIT, publisher).await.unwrap().is_ok());
    }
}
