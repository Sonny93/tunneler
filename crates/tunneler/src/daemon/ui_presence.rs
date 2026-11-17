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

/// Whether a UI just connected: it is the moment to look again for the
/// programs of the built-in openers, which may have been installed since.
pub(crate) const fn has_ui_just_connected(was_ui_present: bool, is_ui_present: bool) -> bool {
    !was_ui_present && is_ui_present
}

fn apply_presence(engine: &Engine, was_ui_present: bool, is_ui_present: bool) {
    engine.set_port_scanning(is_ui_present);
    if has_ui_just_connected(was_ui_present, is_ui_present) {
        let engine = engine.clone();
        tokio::spawn(async move { engine.refresh_detected_openers().await });
    }
}

/// Keeps the engine's port scanning in step with the connected UIs.
///
/// It is on while at least one `ui` client is connected and off otherwise.
/// When the first `ui` client connects, the engine also detects the built-in
/// openers again, in a task the follower does not wait for (a hung mount in
/// the `PATH` must not hold back the shutdown or the port scanning). The task ends when `shutdown` is cancelled.
pub(crate) fn spawn_ui_presence_follower(
    engine: Engine,
    server: ServerHandle,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let mut notifications = server.subscribe_connections();
    tokio::spawn(async move {
        let mut is_ui_present = ui_wanted(&server);
        apply_presence(&engine, is_ui_present, is_ui_present);
        loop {
            tokio::select! {
                () = shutdown.cancelled() => return,
                notification = notifications.recv() => {
                    if matches!(notification, Err(RecvError::Closed)) {
                        return;
                    }
                    let was_ui_present = is_ui_present;
                    is_ui_present = ui_wanted(&server);
                    apply_presence(&engine, was_ui_present, is_ui_present);
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
    fn should_refresh_the_openers_when_the_first_ui_connects() {
        assert!(has_ui_just_connected(false, true));
    }

    #[test]
    fn should_not_refresh_the_openers_when_a_ui_was_already_connected() {
        assert!(!has_ui_just_connected(true, true));
    }

    #[test]
    fn should_not_refresh_the_openers_when_the_last_ui_disconnects() {
        assert!(!has_ui_just_connected(true, false));
    }

    #[test]
    fn should_not_be_wanted_when_nobody_is_connected() {
        assert!(!is_ui_connected([]));
    }
}
