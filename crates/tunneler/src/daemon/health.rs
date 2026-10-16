use engine::Engine;
use ipc::ServerHandle;
use ipc::protocol::ClientKind;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Whether the target health should be probed: only while a UI is watching.
pub(crate) fn should_probe_health(client_kinds: impl IntoIterator<Item = ClientKind>) -> bool {
    client_kinds.into_iter().any(|kind| kind == ClientKind::Ui)
}

fn probing_wanted(server: &ServerHandle) -> bool {
    should_probe_health(
        server
            .connections()
            .iter()
            .map(|connection| connection.client_kind),
    )
}

/// Keeps the engine's health probing in step with the connected UIs.
///
/// Probing is on while at least one `ui` client is connected and off
/// otherwise. The task ends when `shutdown` is cancelled.
pub(crate) fn spawn_health_probing_follower(
    engine: Engine,
    server: ServerHandle,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let mut notifications = server.subscribe_connections();
    tokio::spawn(async move {
        engine.set_health_probing(probing_wanted(&server));
        loop {
            tokio::select! {
                () = shutdown.cancelled() => return,
                notification = notifications.recv() => {
                    if matches!(notification, Err(RecvError::Closed)) {
                        return;
                    }
                    engine.set_health_probing(probing_wanted(&server));
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_probe_when_a_ui_is_connected() {
        assert!(should_probe_health([ClientKind::Cli, ClientKind::Ui]));
    }

    #[test]
    fn should_not_probe_when_only_clis_are_connected() {
        assert!(!should_probe_health([ClientKind::Cli, ClientKind::Cli]));
    }

    #[test]
    fn should_not_probe_when_nobody_is_connected() {
        assert!(!should_probe_health([]));
    }
}
