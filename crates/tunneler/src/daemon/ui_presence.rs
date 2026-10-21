use engine::Engine;
use ipc::ServerHandle;
use ipc::protocol::ClientKind;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Whether a UI is watching: port scanning only runs then.
pub(crate) fn is_ui_connected(client_kinds: impl IntoIterator<Item = ClientKind>) -> bool {
    client_kinds.into_iter().any(|kind| kind == ClientKind::Ui)
}

fn ui_wanted(server: &ServerHandle) -> bool {
    is_ui_connected(
        server
            .connections()
            .iter()
            .map(|connection| connection.client_kind),
    )
}

fn apply_presence(engine: &Engine, is_ui_present: bool) {
    engine.set_port_scanning(is_ui_present);
}

/// Keeps the engine's port scanning in step with the connected UIs.
///
/// It is on while at least one `ui` client is connected and off otherwise. The task ends when `shutdown` is cancelled.
pub(crate) fn spawn_ui_presence_follower(
    engine: Engine,
    server: ServerHandle,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let mut notifications = server.subscribe_connections();
    tokio::spawn(async move {
        apply_presence(&engine, ui_wanted(&server));
        loop {
            tokio::select! {
                () = shutdown.cancelled() => return,
                notification = notifications.recv() => {
                    if matches!(notification, Err(RecvError::Closed)) {
                        return;
                    }
                    apply_presence(&engine, ui_wanted(&server));
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_wanted_when_a_ui_is_connected() {
        assert!(is_ui_connected([ClientKind::Cli, ClientKind::Ui]));
    }

    #[test]
    fn should_not_be_wanted_when_only_clis_are_connected() {
        assert!(!is_ui_connected([ClientKind::Cli, ClientKind::Cli]));
    }

    #[test]
    fn should_not_be_wanted_when_nobody_is_connected() {
        assert!(!is_ui_connected([]));
    }
}
