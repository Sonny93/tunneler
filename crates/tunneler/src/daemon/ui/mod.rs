//! Getting a UI in front of the user: showing the one that runs, launching
//! one when none does, and handing it what it was launched for.
//!
//! The daemon keeps no process handle: a UI is whichever connection declared
//! itself of kind `ui`.

mod launcher;
#[cfg(test)]
#[cfg(unix)]
pub(crate) mod testing;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use ipc::protocol::{ClientKind, EventMessage};
use ipc::{ConnectionId, ConnectionNotification, ServerHandle};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub(crate) use launcher::{ProcessUiLauncher, UiLaunchError, UiLauncher};

use super::prompts::PromptResponderSource;

/// How long a launched UI is given to connect before another launch is allowed.
const UI_LAUNCH_GRACE: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
struct LaunchState {
    launched_at: Option<Instant>,
    is_quit_confirmation_pending: bool,
}

impl LaunchState {
    fn is_launch_in_flight(&self, grace: Duration) -> bool {
        self.launched_at
            .is_some_and(|launched_at| launched_at.elapsed() < grace)
    }
}

/// Shows, launches and feeds the UI on behalf of the tray, the CLI and the
/// prompt broker.
pub(crate) struct UiCoordinator {
    server: ServerHandle,
    launcher: Arc<dyn UiLauncher>,
    launch_grace: Duration,
    state: Mutex<LaunchState>,
}

impl UiCoordinator {
    pub(crate) fn new(server: ServerHandle, launcher: Arc<dyn UiLauncher>) -> Self {
        Self {
            server,
            launcher,
            launch_grace: UI_LAUNCH_GRACE,
            state: Mutex::new(LaunchState::default()),
        }
    }

    #[cfg(test)]
    #[cfg(unix)]
    pub(crate) fn with_launch_grace(mut self, launch_grace: Duration) -> Self {
        self.launch_grace = launch_grace;
        self
    }

    /// Follows the connections: a UI that connects ends the wait for a launch
    /// and receives the quit confirmation that was asked while it started.
    /// The task ends when `shutdown` is cancelled.
    pub(crate) fn spawn_follower(self: &Arc<Self>, shutdown: CancellationToken) -> JoinHandle<()> {
        let coordinator = Arc::clone(self);
        let mut notifications = self.server.subscribe_connections();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    () = shutdown.cancelled() => return,
                    notification = notifications.recv() => match notification {
                        Ok(ConnectionNotification::Opened(info)) if info.client_kind == ClientKind::Ui => {
                            coordinator.handle_ui_connected(info.id);
                        }
                        Ok(_) => {}
                        Err(RecvError::Lagged(_)) => coordinator.catch_up(),
                        Err(RecvError::Closed) => return,
                    },
                }
            }
        })
    }

    /// Brings the UI to the front, launching it when none is connected.
    pub(crate) fn show(&self) -> Result<(), UiLaunchError> {
        let uis = self.ui_connections();
        if uis.is_empty() {
            return self.launch_once();
        }
        self.notify(&uis, &EventMessage::ShowRequested {});
        Ok(())
    }

    /// Has the UI ask the user to confirm quitting, launching it when none is
    /// connected; it is then told as soon as it connects.
    pub(crate) fn request_quit_confirmation(&self) -> Result<(), UiLaunchError> {
        let uis = self.ui_connections();
        if !uis.is_empty() {
            self.notify(&uis, &EventMessage::QuitRequested {});
            return Ok(());
        }
        self.lock().is_quit_confirmation_pending = true;
        self.launch_once().inspect_err(|_| {
            self.lock().is_quit_confirmation_pending = false;
        })
    }

    fn launch_once(&self) -> Result<(), UiLaunchError> {
        {
            let mut state = self.lock();
            if state.is_launch_in_flight(self.launch_grace) {
                return Ok(());
            }
            state.launched_at = Some(Instant::now());
        }
        self.launcher.launch().inspect_err(|error| {
            tracing::error!(%error, "cannot launch the UI");
            self.lock().launched_at = None;
        })
    }

    fn handle_ui_connected(&self, id: ConnectionId) {
        let (was_launch_in_flight, was_quit_pending) = {
            let mut state = self.lock();
            let was_launch_in_flight = state.is_launch_in_flight(self.launch_grace);
            state.launched_at = None;
            (
                was_launch_in_flight,
                std::mem::take(&mut state.is_quit_confirmation_pending),
            )
        };
        if !was_quit_pending {
            return;
        }
        if !was_launch_in_flight {
            tracing::debug!("dropped a quit confirmation that outlived its UI launch");
            return;
        }
        self.notify(&[id], &EventMessage::QuitRequested {});
    }

    fn catch_up(&self) {
        if let Some(id) = self.ui_connections().first() {
            self.handle_ui_connected(*id);
        }
    }

    fn ui_connections(&self) -> Vec<ConnectionId> {
        self.server
            .connections()
            .into_iter()
            .filter(|connection| connection.client_kind == ClientKind::Ui)
            .map(|connection| connection.id)
            .collect()
    }

    fn has_prompt_responder(&self) -> bool {
        self.server
            .connections()
            .iter()
            .any(|connection| connection.can_prompt)
    }

    fn notify(&self, connections: &[ConnectionId], event: &EventMessage) {
        for connection in connections {
            if let Err(error) = self.server.send_to(*connection, event) {
                tracing::warn!(%connection, %error, "cannot reach the UI");
            }
        }
    }

    async fn wait_for_prompt_responder(
        &self,
        notifications: &mut broadcast::Receiver<ConnectionNotification>,
    ) {
        loop {
            if self.has_prompt_responder() {
                return;
            }
            if matches!(notifications.recv().await, Err(RecvError::Closed)) {
                std::future::pending::<()>().await;
            }
        }
    }

    fn lock(&self) -> MutexGuard<'_, LaunchState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[async_trait]
impl PromptResponderSource for UiCoordinator {
    async fn summon_responder(&self, patience: Duration) -> bool {
        let mut notifications = self.server.subscribe_connections();
        if self.has_prompt_responder() {
            return true;
        }
        if self.launch_once().is_err() {
            return false;
        }
        tokio::time::timeout(patience, self.wait_for_prompt_responder(&mut notifications))
            .await
            .is_ok()
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use ipc::protocol::{ClientKind, Hello};
    use ipc::{EventReceiver, IpcClient};
    use pretty_assertions::assert_eq;
    use tokio::time::timeout;

    use super::*;
    use crate::daemon::testing::TestServer;
    use testing::FakeLauncher;

    const WAIT: Duration = Duration::from_secs(5);
    const QUIET_PERIOD: Duration = Duration::from_millis(200);
    const SHORT_PATIENCE: Duration = Duration::from_millis(100);

    struct Fixture {
        server: TestServer,
        launcher: Arc<FakeLauncher>,
        coordinator: Arc<UiCoordinator>,
        shutdown: CancellationToken,
    }

    impl Fixture {
        fn start() -> Self {
            Self::with_launcher(FakeLauncher::succeeding())
        }

        fn with_launcher(launcher: Arc<FakeLauncher>) -> Self {
            let server = TestServer::start();
            let coordinator = Arc::new(UiCoordinator::new(server.handle.clone(), launcher.clone()));
            Self::following(server, launcher, coordinator)
        }

        fn with_grace(launch_grace: Duration) -> Self {
            let server = TestServer::start();
            let launcher = FakeLauncher::succeeding();
            let coordinator = Arc::new(
                UiCoordinator::new(server.handle.clone(), launcher.clone())
                    .with_launch_grace(launch_grace),
            );
            Self::following(server, launcher, coordinator)
        }

        fn following(
            server: TestServer,
            launcher: Arc<FakeLauncher>,
            coordinator: Arc<UiCoordinator>,
        ) -> Self {
            let shutdown = CancellationToken::new();
            coordinator.spawn_follower(shutdown.clone());
            Self {
                server,
                launcher,
                coordinator,
                shutdown,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.shutdown.cancel();
        }
    }

    struct Connected {
        _client: IpcClient,
        events: EventReceiver,
    }

    async fn connect(server: &TestServer, kind: ClientKind) -> Connected {
        let mut client = server.connect(kind, true, true).await;
        let events = client.take_events().unwrap();
        wait_until_registered(server, kind).await;
        Connected {
            _client: client,
            events,
        }
    }

    async fn wait_until_registered(server: &TestServer, kind: ClientKind) {
        timeout(WAIT, async {
            while !server
                .handle
                .connections()
                .iter()
                .any(|connection| connection.client_kind == kind)
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }

    async fn next_event(connected: &mut Connected) -> EventMessage {
        timeout(WAIT, connected.events.recv())
            .await
            .unwrap()
            .unwrap()
    }

    async fn assert_silent(connected: &mut Connected) {
        assert!(
            timeout(QUIET_PERIOD, connected.events.recv())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn should_send_show_requested_when_a_ui_is_connected() {
        let fixture = Fixture::start();
        let mut ui = connect(&fixture.server, ClientKind::Ui).await;

        fixture.coordinator.show().unwrap();

        assert_eq!(next_event(&mut ui).await, EventMessage::ShowRequested {});
    }

    #[tokio::test]
    async fn should_not_launch_when_a_ui_is_connected() {
        let fixture = Fixture::start();
        let _ui = connect(&fixture.server, ClientKind::Ui).await;

        fixture.coordinator.show().unwrap();

        assert_eq!(fixture.launcher.launch_count(), 0);
    }

    #[tokio::test]
    async fn should_launch_once_when_no_ui_is_connected() {
        let fixture = Fixture::start();

        fixture.coordinator.show().unwrap();

        assert_eq!(fixture.launcher.launch_count(), 1);
    }

    #[tokio::test]
    async fn should_not_launch_again_when_a_launch_is_in_flight() {
        let fixture = Fixture::start();
        fixture.coordinator.show().unwrap();

        fixture.coordinator.show().unwrap();

        assert_eq!(fixture.launcher.launch_count(), 1);
    }

    #[tokio::test]
    async fn should_launch_again_when_the_grace_period_is_over() {
        let fixture = Fixture::with_grace(Duration::ZERO);
        fixture.coordinator.show().unwrap();

        fixture.coordinator.show().unwrap();

        assert_eq!(fixture.launcher.launch_count(), 2);
    }

    #[tokio::test]
    async fn should_launch_again_when_the_launched_ui_connected_then_left() {
        let fixture = Fixture::start();
        fixture.coordinator.show().unwrap();
        let ui = connect(&fixture.server, ClientKind::Ui).await;
        drop(ui);
        timeout(WAIT, async {
            while !fixture.server.handle.connections().is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        fixture.coordinator.show().unwrap();

        assert_eq!(fixture.launcher.launch_count(), 2);
    }

    #[tokio::test]
    async fn should_not_take_a_cli_for_a_ui() {
        let fixture = Fixture::start();
        let _cli = connect(&fixture.server, ClientKind::Cli).await;

        fixture.coordinator.show().unwrap();

        assert_eq!(fixture.launcher.launch_count(), 1);
    }

    #[tokio::test]
    async fn should_not_send_show_requested_to_a_cli() {
        let fixture = Fixture::start();
        let mut cli = connect(&fixture.server, ClientKind::Cli).await;

        fixture.coordinator.show().unwrap();

        assert_silent(&mut cli).await;
    }

    #[tokio::test]
    async fn should_return_the_error_when_the_launch_fails() {
        let fixture = Fixture::with_launcher(FakeLauncher::failing());

        let outcome = fixture.coordinator.show();

        assert!(matches!(outcome, Err(UiLaunchError::Spawn { .. })));
    }

    #[tokio::test]
    async fn should_launch_again_when_the_previous_launch_failed() {
        let fixture = Fixture::with_launcher(FakeLauncher::failing());
        let _ = fixture.coordinator.show();

        let _ = fixture.coordinator.show();

        assert_eq!(fixture.launcher.launch_count(), 2);
    }

    #[tokio::test]
    async fn should_send_quit_requested_at_once_when_a_ui_is_connected() {
        let fixture = Fixture::start();
        let mut ui = connect(&fixture.server, ClientKind::Ui).await;

        fixture.coordinator.request_quit_confirmation().unwrap();

        assert_eq!(next_event(&mut ui).await, EventMessage::QuitRequested {});
    }

    #[tokio::test]
    async fn should_launch_the_ui_when_a_quit_confirmation_is_asked_without_one() {
        let fixture = Fixture::start();

        fixture.coordinator.request_quit_confirmation().unwrap();

        assert_eq!(fixture.launcher.launch_count(), 1);
    }

    #[tokio::test]
    async fn should_deliver_the_pending_quit_confirmation_to_the_ui_that_connects() {
        let fixture = Fixture::start();
        fixture.coordinator.request_quit_confirmation().unwrap();

        let mut ui = connect(&fixture.server, ClientKind::Ui).await;

        assert_eq!(next_event(&mut ui).await, EventMessage::QuitRequested {});
    }

    #[tokio::test]
    async fn should_drop_the_pending_quit_confirmation_when_the_launch_grace_is_over() {
        let fixture = Fixture::with_grace(Duration::from_millis(50));
        fixture.coordinator.request_quit_confirmation().unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;

        let mut ui = connect(&fixture.server, ClientKind::Ui).await;

        assert_silent(&mut ui).await;
    }

    #[tokio::test]
    async fn should_deliver_the_pending_quit_confirmation_only_once() {
        let fixture = Fixture::start();
        fixture.coordinator.request_quit_confirmation().unwrap();
        let mut first = connect(&fixture.server, ClientKind::Ui).await;
        next_event(&mut first).await;

        let mut second = connect(&fixture.server, ClientKind::Ui).await;

        assert_silent(&mut second).await;
    }

    #[tokio::test]
    async fn should_not_deliver_a_quit_confirmation_when_the_launch_failed() {
        let fixture = Fixture::with_launcher(FakeLauncher::failing());
        let _ = fixture.coordinator.request_quit_confirmation();

        let mut ui = connect(&fixture.server, ClientKind::Ui).await;

        assert_silent(&mut ui).await;
    }

    #[tokio::test]
    async fn should_not_deliver_a_pending_quit_confirmation_to_a_cli() {
        let fixture = Fixture::start();
        fixture.coordinator.request_quit_confirmation().unwrap();

        let mut cli = connect(&fixture.server, ClientKind::Cli).await;

        assert_silent(&mut cli).await;
    }

    #[tokio::test]
    async fn should_report_a_responder_when_one_is_already_connected() {
        let fixture = Fixture::start();
        let _ui = connect(&fixture.server, ClientKind::Ui).await;

        let summoned = fixture.coordinator.summon_responder(SHORT_PATIENCE).await;

        assert!(summoned);
    }

    #[tokio::test]
    async fn should_not_launch_when_a_prompt_responder_is_already_connected() {
        let fixture = Fixture::start();
        let _ui = connect(&fixture.server, ClientKind::Ui).await;

        fixture.coordinator.summon_responder(SHORT_PATIENCE).await;

        assert_eq!(fixture.launcher.launch_count(), 0);
    }

    #[tokio::test]
    async fn should_report_a_responder_when_a_ui_connects_after_the_launch() {
        let fixture = Fixture::start();
        let summoning = {
            let coordinator = Arc::clone(&fixture.coordinator);
            tokio::spawn(async move { coordinator.summon_responder(WAIT).await })
        };
        timeout(WAIT, async {
            while fixture.launcher.launch_count() == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        let _ui = connect(&fixture.server, ClientKind::Ui).await;

        assert!(timeout(WAIT, summoning).await.unwrap().unwrap());
    }

    #[tokio::test]
    async fn should_report_no_responder_when_none_connects_in_time() {
        let fixture = Fixture::start();

        let summoned = fixture.coordinator.summon_responder(SHORT_PATIENCE).await;

        assert!(!summoned);
    }

    #[tokio::test]
    async fn should_report_no_responder_when_the_launch_fails() {
        let fixture = Fixture::with_launcher(FakeLauncher::failing());

        let summoned = fixture.coordinator.summon_responder(WAIT).await;

        assert!(!summoned);
    }

    #[tokio::test]
    async fn should_launch_when_only_a_client_that_cannot_prompt_is_connected() {
        let fixture = Fixture::start();
        let hello = Hello::new(ClientKind::Cli, "test", false, false);
        let _cli = IpcClient::connect(&fixture.server.endpoint, hello)
            .await
            .unwrap();

        fixture.coordinator.summon_responder(SHORT_PATIENCE).await;

        assert_eq!(fixture.launcher.launch_count(), 1);
    }
}
