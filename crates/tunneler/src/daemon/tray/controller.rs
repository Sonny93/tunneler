//! Keeps the tray in step with the engine and carries out what the user
//! chooses in the tray menu.

use std::ops::ControlFlow;
use std::sync::Arc;

use engine::{Engine, EngineError, EngineEvent, EngineSnapshot};
use tokio::sync::broadcast::{
    self,
    error::{RecvError, TryRecvError},
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::TrayDisplay;
use super::menu::TrayAction;
use super::view::{TrayView, current_tray_view};
use crate::daemon::ui::{UiCoordinator, UiLaunchError};

/// What the tray needs from the tunnel engine.
pub(crate) trait TunnelControl: Send + Sync + 'static {
    /// The tunnels with their state.
    fn snapshot(&self) -> EngineSnapshot;
    /// Follows what happens to the tunnels.
    fn subscribe(&self) -> broadcast::Receiver<EngineEvent>;
    /// Starts every tunnel of a tag.
    fn start_tag(&self, tag: &str) -> Result<(), EngineError>;
    /// Stops every tunnel of a tag.
    fn stop_tag(&self, tag: &str) -> Result<(), EngineError>;
    /// Sends every awake tunnel back to standby.
    fn standby_all(&self) -> Result<(), EngineError>;
    /// How many tunnels and X11 anchors hold resources.
    fn active_count(&self) -> usize;
}

impl TunnelControl for Engine {
    fn snapshot(&self) -> EngineSnapshot {
        Engine::snapshot(self)
    }

    fn subscribe(&self) -> broadcast::Receiver<EngineEvent> {
        Engine::subscribe(self)
    }

    fn start_tag(&self, tag: &str) -> Result<(), EngineError> {
        Engine::start_tag(self, tag).map(|_| ())
    }

    fn stop_tag(&self, tag: &str) -> Result<(), EngineError> {
        Engine::stop_tag(self, tag).map(|_| ())
    }

    fn standby_all(&self) -> Result<(), EngineError> {
        Engine::standby_all(self).map(|_| ())
    }

    fn active_count(&self) -> usize {
        Engine::active_tunnel_count(self) + Engine::active_x11_count(self)
    }
}

/// What the tray asks of the UI.
pub(crate) trait UiRequests: Send + Sync + 'static {
    /// Brings the UI to the front, launching it when needed.
    fn show(&self) -> Result<(), UiLaunchError>;
    /// Has the UI ask the user to confirm quitting.
    fn request_quit_confirmation(&self) -> Result<(), UiLaunchError>;
    /// Has the UI ask the user to confirm sending the awake tunnels back to
    /// standby.
    fn request_standby_confirmation(&self) -> Result<(), UiLaunchError>;
}

impl UiRequests for UiCoordinator {
    fn show(&self) -> Result<(), UiLaunchError> {
        UiCoordinator::show(self)
    }

    fn request_quit_confirmation(&self) -> Result<(), UiLaunchError> {
        UiCoordinator::request_quit_confirmation(self)
    }

    fn request_standby_confirmation(&self) -> Result<(), UiLaunchError> {
        UiCoordinator::request_standby_confirmation(self)
    }
}

/// Everything the controller works with.
pub(crate) struct TrayControllerParts {
    /// The engine, for views and for the tag actions.
    pub(crate) engine: Arc<dyn TunnelControl>,
    /// Where the views are shown.
    pub(crate) display: Arc<dyn TrayDisplay>,
    /// What the user chose in the tray.
    pub(crate) actions: mpsc::UnboundedReceiver<TrayAction>,
    /// Where `Open`, `Quit` and `StandbyAll` go.
    pub(crate) ui: Arc<dyn UiRequests>,
    /// Cancelled to stop the daemon.
    pub(crate) quit_requested: CancellationToken,
    /// What the tray shows already.
    pub(crate) shown_view: TrayView,
}

struct TrayController {
    engine: Arc<dyn TunnelControl>,
    display: Arc<dyn TrayDisplay>,
    ui: Arc<dyn UiRequests>,
    quit_requested: CancellationToken,
    shown_view: TrayView,
}

/// Runs the controller until `shutdown` is cancelled.
pub(crate) fn spawn_tray_controller(
    parts: TrayControllerParts,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let events = parts.engine.subscribe();
    let mut controller = TrayController {
        engine: parts.engine,
        display: parts.display,
        ui: parts.ui,
        quit_requested: parts.quit_requested,
        shown_view: parts.shown_view,
    };
    let mut actions = parts.actions;
    tokio::spawn(async move {
        controller.refresh();
        controller.run(events, &mut actions, &shutdown).await;
    })
}

/// Whether an engine event can change what the tray shows.
fn changes_the_view(event: &EngineEvent) -> bool {
    matches!(
        event,
        EngineEvent::TunnelChanged { .. } | EngineEvent::ConfigReloaded
    )
}

impl TrayController {
    async fn run(
        &mut self,
        mut events: broadcast::Receiver<EngineEvent>,
        actions: &mut mpsc::UnboundedReceiver<TrayAction>,
        shutdown: &CancellationToken,
    ) {
        loop {
            tokio::select! {
                () = shutdown.cancelled() => return,
                received = events.recv() => {
                    if self.on_event(received, &mut events).is_break() {
                        return;
                    }
                }
                action = actions.recv() => match action {
                    Some(action) => self.on_action(action),
                    None => return,
                },
            }
        }
    }

    /// Breaks when the engine has no more events to give.
    fn on_event(
        &mut self,
        received: Result<EngineEvent, RecvError>,
        events: &mut broadcast::Receiver<EngineEvent>,
    ) -> ControlFlow<()> {
        match received {
            Ok(event) if !changes_the_view(&event) => {}
            Ok(_) | Err(RecvError::Lagged(_)) => {
                Self::discard_queued(events);
                self.refresh();
            }
            Err(RecvError::Closed) => return ControlFlow::Break(()),
        }
        ControlFlow::Continue(())
    }

    /// A burst of events needs one redraw: whatever is queued is covered by
    /// the snapshot read next.
    fn discard_queued(events: &mut broadcast::Receiver<EngineEvent>) {
        while !matches!(
            events.try_recv(),
            Err(TryRecvError::Empty | TryRecvError::Closed)
        ) {}
    }

    fn refresh(&mut self) {
        let view = current_tray_view(&self.engine.snapshot());
        if view != self.shown_view {
            self.show(view);
        }
    }

    fn show(&mut self, view: TrayView) {
        self.display.show(view.clone());
        self.shown_view = view;
    }

    fn on_action(&mut self, action: TrayAction) {
        match action {
            TrayAction::ToggleTag(tag) => self.toggle_tag(&tag),
            TrayAction::StandbyAll => self.standby_all(),
            TrayAction::OpenUi => self.open_ui(),
            TrayAction::Quit => self.quit(),
        }
    }

    /// Redraws afterwards even when nothing changed: a native check item may
    /// have toggled itself.
    fn toggle_tag(&mut self, tag: &str) {
        let is_running = current_tray_view(&self.engine.snapshot())
            .tags
            .iter()
            .any(|entry| entry.tag == tag && entry.is_checked);
        let outcome = if is_running {
            self.engine.stop_tag(tag)
        } else {
            self.engine.start_tag(tag)
        };
        if let Err(error) = outcome {
            tracing::warn!(tag, %error, "the tray could not toggle the tag");
        }
        self.show(current_tray_view(&self.engine.snapshot()));
    }

    fn standby_all(&self) {
        if self.has_awake_tunnel_with_connections() {
            if let Err(error) = self.ui.request_standby_confirmation() {
                tracing::error!(%error, "awake tunnels have open connections and the UI that would confirm the standby cannot be launched, the tunnels stay awake");
            }
            return;
        }
        if let Err(error) = self.engine.standby_all() {
            tracing::warn!(%error, "the tray could not send the awake tunnels back to standby");
        }
    }

    fn has_awake_tunnel_with_connections(&self) -> bool {
        self.engine
            .snapshot()
            .tunnels
            .iter()
            .any(|tunnel| tunnel.is_awake && tunnel.metrics.active_connections > 0)
    }

    fn open_ui(&self) {
        if let Err(error) = self.ui.show() {
            tracing::error!(%error, "the tray could not open the UI");
        }
    }

    fn quit(&self) {
        if self.engine.active_count() == 0 {
            self.quit_requested.cancel();
            return;
        }
        if let Err(error) = self.ui.request_quit_confirmation() {
            tracing::error!(%error, "tunnels are active and the UI that would confirm quitting cannot be launched, the daemon keeps running");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use engine::{
        Language, Settings, TargetHealth, TunnelDefinition, TunnelId, TunnelKind, TunnelMetrics,
        TunnelSnapshot, TunnelState,
    };
    use pretty_assertions::assert_eq;

    use crate::daemon::display_language::DisplayLanguage;
    use tokio::time::timeout;

    use super::*;

    const WAIT: Duration = Duration::from_secs(5);
    const QUIET_PERIOD: Duration = Duration::from_millis(200);
    const POLL_INTERVAL: Duration = Duration::from_millis(10);
    const EVENT_CAPACITY: usize = 16;

    fn tunnel(state: TunnelState, tags: &[&str]) -> TunnelSnapshot {
        TunnelSnapshot {
            definition: TunnelDefinition {
                id: TunnelId::generate(),
                host: "nova".to_owned(),
                kind: TunnelKind::Local,
                listen_port: 8080,
                listen_address: None,
                target_host: None,
                target_port: None,
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                open: engine::OpenOptions::default(),
                standby: None,
            },
            state,
            health: TargetHealth::Unknown,
            metrics: TunnelMetrics::default(),
            is_awake: false,
            is_temporary: false,
        }
    }

    fn awake_tunnel(active_connections: usize) -> TunnelSnapshot {
        TunnelSnapshot {
            is_awake: true,
            metrics: TunnelMetrics {
                active_connections,
                ..TunnelMetrics::default()
            },
            ..tunnel(TunnelState::Active, &[])
        }
    }

    struct FakeEngine {
        tunnels: Mutex<Vec<TunnelSnapshot>>,
        language: Mutex<Language>,
        events: broadcast::Sender<EngineEvent>,
        started_tags: Mutex<Vec<String>>,
        stopped_tags: Mutex<Vec<String>>,
        standby_all_calls: AtomicUsize,
        active_tunnels: AtomicUsize,
    }

    impl FakeEngine {
        fn with_tunnels(tunnels: Vec<TunnelSnapshot>) -> Arc<Self> {
            let active_tunnels = tunnels
                .iter()
                .filter(|tunnel| tunnel.state.is_running())
                .count();
            Arc::new(Self {
                tunnels: Mutex::new(tunnels),
                language: Mutex::new(Language::English),
                events: broadcast::channel(EVENT_CAPACITY).0,
                started_tags: Mutex::new(Vec::new()),
                stopped_tags: Mutex::new(Vec::new()),
                standby_all_calls: AtomicUsize::new(0),
                active_tunnels: AtomicUsize::new(active_tunnels),
            })
        }

        fn replace_tunnels(&self, tunnels: Vec<TunnelSnapshot>) {
            *self.tunnels.lock().unwrap() = tunnels;
        }

        fn replace_language(&self, language: Language) {
            *self.language.lock().unwrap() = language;
        }

        fn announce_change(&self) {
            let changed = EngineEvent::ConfigReloaded;
            self.events.send(changed).unwrap();
        }
    }

    impl TunnelControl for FakeEngine {
        fn snapshot(&self) -> EngineSnapshot {
            EngineSnapshot {
                tunnels: self.tunnels.lock().unwrap().clone(),
                settings: Settings {
                    language: *self.language.lock().unwrap(),
                    ..Settings::default()
                },
                externals: Vec::new(),
                x11_anchors: Vec::new(),
                x11_launchers: Vec::new(),
                openers: Vec::new(),
            }
        }

        fn subscribe(&self) -> broadcast::Receiver<EngineEvent> {
            self.events.subscribe()
        }

        fn start_tag(&self, tag: &str) -> Result<(), EngineError> {
            self.started_tags.lock().unwrap().push(tag.to_owned());
            Ok(())
        }

        fn stop_tag(&self, tag: &str) -> Result<(), EngineError> {
            self.stopped_tags.lock().unwrap().push(tag.to_owned());
            Ok(())
        }

        fn standby_all(&self) -> Result<(), EngineError> {
            self.standby_all_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn active_count(&self) -> usize {
            self.active_tunnels.load(Ordering::SeqCst)
        }
    }

    #[derive(Default)]
    struct RecordingDisplay {
        views: Mutex<Vec<TrayView>>,
    }

    impl RecordingDisplay {
        fn shown(&self) -> Vec<TrayView> {
            self.views.lock().unwrap().clone()
        }
    }

    impl TrayDisplay for RecordingDisplay {
        fn show(&self, view: TrayView) {
            self.views.lock().unwrap().push(view);
        }
    }

    #[derive(Default)]
    struct RecordingUi {
        shows: AtomicUsize,
        quit_confirmations: AtomicUsize,
        standby_confirmations: AtomicUsize,
        is_unavailable: bool,
    }

    impl RecordingUi {
        fn unavailable() -> Self {
            Self {
                is_unavailable: true,
                ..Self::default()
            }
        }

        fn outcome(&self) -> Result<(), UiLaunchError> {
            if self.is_unavailable {
                return Err(UiLaunchError::Spawn {
                    program: "tunneler-ui".into(),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                });
            }
            Ok(())
        }
    }

    impl UiRequests for RecordingUi {
        fn show(&self) -> Result<(), UiLaunchError> {
            self.shows.fetch_add(1, Ordering::SeqCst);
            self.outcome()
        }

        fn request_quit_confirmation(&self) -> Result<(), UiLaunchError> {
            self.quit_confirmations.fetch_add(1, Ordering::SeqCst);
            self.outcome()
        }

        fn request_standby_confirmation(&self) -> Result<(), UiLaunchError> {
            self.standby_confirmations.fetch_add(1, Ordering::SeqCst);
            self.outcome()
        }
    }

    struct Fixture {
        engine: Arc<FakeEngine>,
        display: Arc<RecordingDisplay>,
        ui: Arc<RecordingUi>,
        actions: mpsc::UnboundedSender<TrayAction>,
        quit_requested: CancellationToken,
        shutdown: CancellationToken,
        task: JoinHandle<()>,
    }

    impl Fixture {
        fn start(tunnels: Vec<TunnelSnapshot>) -> Self {
            Self::start_with_ui(tunnels, RecordingUi::default())
        }

        fn start_with_ui(tunnels: Vec<TunnelSnapshot>, ui: RecordingUi) -> Self {
            let engine = FakeEngine::with_tunnels(tunnels);
            let display = Arc::new(RecordingDisplay::default());
            let ui = Arc::new(ui);
            let (actions, action_receiver) = mpsc::unbounded_channel();
            let quit_requested = CancellationToken::new();
            let shutdown = CancellationToken::new();
            let task = spawn_tray_controller(
                TrayControllerParts {
                    engine: engine.clone(),
                    display: display.clone(),
                    actions: action_receiver,
                    ui: ui.clone(),
                    quit_requested: quit_requested.clone(),
                    shown_view: current_tray_view(&engine.snapshot()),
                },
                shutdown.clone(),
            );
            Self {
                engine,
                display,
                ui,
                actions,
                quit_requested,
                shutdown,
                task,
            }
        }

        async fn wait_until(&self, description: &str, condition: impl Fn() -> bool) {
            timeout(WAIT, async {
                while !condition() {
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {description}"));
        }

        async fn let_the_controller_catch_up(&self) {
            tokio::time::sleep(QUIET_PERIOD).await;
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.shutdown.cancel();
        }
    }

    #[tokio::test]
    async fn should_show_a_new_view_when_a_tunnel_changes() {
        let fixture = Fixture::start(vec![]);
        fixture
            .engine
            .replace_tunnels(vec![tunnel(TunnelState::Active, &[])]);

        fixture.engine.announce_change();

        fixture
            .wait_until("the new view", || !fixture.display.shown().is_empty())
            .await;
        assert_eq!(fixture.display.shown()[0].summary_line, "1 active tunnel");
    }

    #[tokio::test]
    async fn should_show_the_new_language_when_the_language_setting_changes() {
        let fixture = Fixture::start(vec![]);
        fixture.engine.replace_language(Language::French);

        fixture.engine.announce_change();

        fixture
            .wait_until("the new view", || !fixture.display.shown().is_empty())
            .await;
        let view = fixture.display.shown().remove(0);
        assert_eq!(view.language, DisplayLanguage::French);
        assert_eq!(view.summary_line, "Aucun tunnel actif");
    }

    #[tokio::test]
    async fn should_not_show_a_view_that_did_not_change() {
        let fixture = Fixture::start(vec![]);
        fixture.engine.announce_change();
        fixture.let_the_controller_catch_up().await;

        assert_eq!(fixture.display.shown(), vec![]);
    }

    #[tokio::test]
    async fn should_show_one_view_when_a_burst_of_events_arrives() {
        let fixture = Fixture::start(vec![]);
        fixture
            .engine
            .replace_tunnels(vec![tunnel(TunnelState::Active, &[])]);

        for _ in 0..5 {
            fixture.engine.announce_change();
        }
        fixture.let_the_controller_catch_up().await;

        assert_eq!(fixture.display.shown().len(), 1);
    }

    #[tokio::test]
    async fn should_ignore_events_that_cannot_change_the_view() {
        let fixture = Fixture::start(vec![]);
        fixture.let_the_controller_catch_up().await;
        fixture
            .engine
            .replace_tunnels(vec![tunnel(TunnelState::Active, &[])]);

        fixture
            .engine
            .events
            .send(EngineEvent::ConfigInvalid {
                message: "bad".to_owned(),
            })
            .unwrap();
        fixture.let_the_controller_catch_up().await;

        assert_eq!(fixture.display.shown(), vec![]);
    }

    #[tokio::test]
    async fn should_show_a_view_at_start_when_the_engine_moved_on_meanwhile() {
        let engine = FakeEngine::with_tunnels(vec![tunnel(TunnelState::Active, &[])]);
        let display = Arc::new(RecordingDisplay::default());
        let (_actions, action_receiver) = mpsc::unbounded_channel();
        let stale_view = current_tray_view(&FakeEngine::with_tunnels(vec![]).snapshot());

        let task = spawn_tray_controller(
            TrayControllerParts {
                engine,
                display: display.clone(),
                actions: action_receiver,
                ui: Arc::new(RecordingUi::default()),
                quit_requested: CancellationToken::new(),
                shown_view: stale_view,
            },
            CancellationToken::new(),
        );

        timeout(WAIT, async {
            while display.shown().is_empty() {
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        })
        .await
        .unwrap();
        task.abort();
    }

    #[tokio::test]
    async fn should_stop_the_tag_and_show_the_view_again_when_toggling_a_tag_whose_tunnels_all_run()
    {
        let fixture = Fixture::start(vec![tunnel(TunnelState::Active, &["dev"])]);

        fixture
            .actions
            .send(TrayAction::ToggleTag("dev".to_owned()))
            .unwrap();

        fixture
            .wait_until("the tag to stop", || {
                !fixture.engine.stopped_tags.lock().unwrap().is_empty()
            })
            .await;
        assert_eq!(*fixture.engine.stopped_tags.lock().unwrap(), vec!["dev"]);
        fixture
            .wait_until("the redraw", || !fixture.display.shown().is_empty())
            .await;
    }

    #[tokio::test]
    async fn should_start_the_tag_when_toggling_a_tag_with_a_stopped_tunnel() {
        let fixture = Fixture::start(vec![
            tunnel(TunnelState::Active, &["dev"]),
            tunnel(TunnelState::Stopped, &["dev"]),
        ]);

        fixture
            .actions
            .send(TrayAction::ToggleTag("dev".to_owned()))
            .unwrap();

        fixture
            .wait_until("the tag to start", || {
                !fixture.engine.started_tags.lock().unwrap().is_empty()
            })
            .await;
        assert_eq!(*fixture.engine.started_tags.lock().unwrap(), vec!["dev"]);
    }

    #[tokio::test]
    async fn should_stop_the_tag_when_toggling_a_tag_whose_tunnels_all_run_or_wait_on_standby() {
        let fixture = Fixture::start(vec![
            tunnel(TunnelState::Standby { last_failure: None }, &["dev"]),
            tunnel(TunnelState::Active, &["dev"]),
        ]);

        fixture
            .actions
            .send(TrayAction::ToggleTag("dev".to_owned()))
            .unwrap();

        fixture
            .wait_until("the tag to stop", || {
                !fixture.engine.stopped_tags.lock().unwrap().is_empty()
            })
            .await;
        assert!(fixture.engine.started_tags.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn should_send_the_awake_tunnels_back_to_standby_at_once_when_none_has_open_connections()
    {
        let fixture = Fixture::start(vec![awake_tunnel(0)]);

        fixture.actions.send(TrayAction::StandbyAll).unwrap();

        fixture
            .wait_until("the standby", || {
                fixture.engine.standby_all_calls.load(Ordering::SeqCst) == 1
            })
            .await;
        assert_eq!(fixture.ui.standby_confirmations.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_ask_the_ui_to_confirm_when_an_awake_tunnel_has_open_connections() {
        let fixture = Fixture::start(vec![awake_tunnel(0), awake_tunnel(2)]);

        fixture.actions.send(TrayAction::StandbyAll).unwrap();

        fixture
            .wait_until("the confirmation request", || {
                fixture.ui.standby_confirmations.load(Ordering::SeqCst) == 1
            })
            .await;
        assert_eq!(fixture.engine.standby_all_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_not_ask_for_a_confirmation_when_only_a_tunnel_that_is_not_awake_has_connections()
     {
        let busy_but_not_awake = TunnelSnapshot {
            metrics: TunnelMetrics {
                active_connections: 3,
                ..TunnelMetrics::default()
            },
            ..tunnel(TunnelState::Active, &[])
        };
        let fixture = Fixture::start(vec![busy_but_not_awake, awake_tunnel(0)]);

        fixture.actions.send(TrayAction::StandbyAll).unwrap();

        fixture
            .wait_until("the standby", || {
                fixture.engine.standby_all_calls.load(Ordering::SeqCst) == 1
            })
            .await;
        assert_eq!(fixture.ui.standby_confirmations.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_keep_the_tunnels_awake_when_the_confirming_ui_cannot_be_launched() {
        let fixture = Fixture::start_with_ui(vec![awake_tunnel(1)], RecordingUi::unavailable());

        fixture.actions.send(TrayAction::StandbyAll).unwrap();
        fixture.let_the_controller_catch_up().await;

        assert_eq!(fixture.engine.standby_all_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_open_the_ui_when_the_open_action_arrives() {
        let fixture = Fixture::start(vec![]);

        fixture.actions.send(TrayAction::OpenUi).unwrap();

        fixture
            .wait_until("the UI to be asked", || {
                fixture.ui.shows.load(Ordering::SeqCst) == 1
            })
            .await;
    }

    #[tokio::test]
    async fn should_request_the_quit_when_quitting_with_no_active_tunnel() {
        let fixture = Fixture::start(vec![tunnel(TunnelState::Stopped, &[])]);

        fixture.actions.send(TrayAction::Quit).unwrap();

        timeout(WAIT, fixture.quit_requested.cancelled())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn should_ask_the_ui_to_confirm_when_quitting_with_active_tunnels() {
        let fixture = Fixture::start(vec![tunnel(TunnelState::Active, &[])]);

        fixture.actions.send(TrayAction::Quit).unwrap();

        fixture
            .wait_until("the confirmation request", || {
                fixture.ui.quit_confirmations.load(Ordering::SeqCst) == 1
            })
            .await;
    }

    #[tokio::test]
    async fn should_not_quit_when_quitting_with_active_tunnels() {
        let fixture = Fixture::start(vec![tunnel(TunnelState::Active, &[])]);

        fixture.actions.send(TrayAction::Quit).unwrap();
        fixture.let_the_controller_catch_up().await;

        assert!(!fixture.quit_requested.is_cancelled());
    }

    #[tokio::test]
    async fn should_keep_running_when_the_confirming_ui_cannot_be_launched() {
        let fixture = Fixture::start_with_ui(
            vec![tunnel(TunnelState::Active, &[])],
            RecordingUi::unavailable(),
        );

        fixture.actions.send(TrayAction::Quit).unwrap();
        fixture.let_the_controller_catch_up().await;

        assert!(!fixture.quit_requested.is_cancelled());
    }

    #[tokio::test]
    async fn should_end_when_the_shutdown_token_is_cancelled() {
        let mut fixture = Fixture::start(vec![]);

        fixture.shutdown.cancel();

        let task = std::mem::replace(&mut fixture.task, tokio::spawn(async {}));
        assert!(timeout(WAIT, task).await.unwrap().is_ok());
    }
}
