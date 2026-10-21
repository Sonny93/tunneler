#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::sync::Arc;
use std::time::Duration;

use engine::{
    Engine, EngineDependencies, EngineError, EngineEvent, EngineOptions, Store, SystemClock,
    TunnelId, TunnelState, X11AnchorState, X11Launcher,
};
use russh::keys::Algorithm;
use support::{
    FakePortScanner, FakeX11Environment, FakeXServer, TestServer, TestUser, Workspace, decode_hex,
    expected_server_setup, free_port, generate_key, wait_for_event, wait_until, write_private_key,
    x_client_setup, x_client_setup_with_cookie,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast::Receiver;
use tokio::time::{sleep, timeout};

const WAIT: Duration = Duration::from_secs(5);
const QUIET_PERIOD: Duration = Duration::from_millis(300);
const RECONNECT_INITIAL_MILLISECONDS: u64 = 20;
const RECONNECT_MAXIMUM_MILLISECONDS: u64 = 80;
const X_TCP_BASE_PORT: u16 = 6000;
const X_SERVER_REPLY: &[u8] = b"X-SERVER-REPLY";
const REAL_COOKIE: [u8; 16] = [0x5a; 16];
const REMOTE_DISPLAY: &str = "localhost:10.0";
const LAUNCHER_NAME: &str = "Editor";
const LAUNCHER_COMMAND: &str = "code --wait";

enum Authority {
    Cookie,
    Missing,
}

enum ServerPolicy {
    Anchoring,
    RefusingX11,
    ReportingEmptyDisplay,
}

struct HarnessBuilder {
    server_policy: ServerPolicy,
    authority: Authority,
    has_x_server: bool,
    has_display_variable: bool,
}

struct Harness {
    _workspace: Workspace,
    _store_directory: tempfile::TempDir,
    server: TestServer,
    x_server: FakeXServer,
    engine: Engine,
    events: Receiver<EngineEvent>,
}

impl HarnessBuilder {
    fn new() -> Self {
        Self {
            server_policy: ServerPolicy::Anchoring,
            authority: Authority::Cookie,
            has_x_server: true,
            has_display_variable: true,
        }
    }

    fn without_display_variable(mut self) -> Self {
        self.has_display_variable = false;
        self
    }

    fn refusing_x11(mut self) -> Self {
        self.server_policy = ServerPolicy::RefusingX11;
        self
    }

    fn reporting_empty_display(mut self) -> Self {
        self.server_policy = ServerPolicy::ReportingEmptyDisplay;
        self
    }

    fn without_authority_file(mut self) -> Self {
        self.authority = Authority::Missing;
        self
    }

    fn without_x_server(mut self) -> Self {
        self.has_x_server = false;
        self
    }

    async fn start(self) -> Harness {
        let client_key = generate_key(Algorithm::Ed25519);
        let server = self.start_server(&client_key).await;
        let workspace = Workspace::new();
        let key_path = write_private_key(workspace.path(), "id_test", &client_key, None);
        let identity_line = format!("IdentityFile {}", key_path.display());
        let config_path = workspace.write_config(
            server.port,
            &[
                identity_line.as_str(),
                "IdentitiesOnly yes",
                "PreferredAuthentications publickey",
            ],
        );
        let x_server = FakeXServer::start(X_SERVER_REPLY).await;
        let environment = self.environment(&workspace, &x_server).await;
        let store_directory = tempfile::tempdir().unwrap();
        let store_path = store_directory.path().join("tunnels.toml");
        std::fs::write(&store_path, store_text(free_port().await)).unwrap();
        let (store, _) = Store::open(store_path).unwrap();
        let connector = workspace.connector_dependencies(
            &config_path,
            support::quiet_prompter(),
            support::rejecting_verifier(),
            Arc::new(support::NoAgentLocator),
        );
        let engine = Engine::start_with_options(
            EngineDependencies {
                connector,
                clock: Arc::new(SystemClock),
                store,
                port_scanner: Arc::new(FakePortScanner::default()),
                x11_environment: Arc::new(environment),
                state_store: Arc::new(support::MemoryStateStore::default()),
                keyring: None,
                detected_openers: Vec::new(),
            },
            EngineOptions::default(),
        )
        .unwrap();
        let events = engine.subscribe();
        Harness {
            _workspace: workspace,
            _store_directory: store_directory,
            server,
            x_server,
            engine,
            events,
        }
    }

    async fn start_server(&self, client_key: &russh::keys::PrivateKey) -> TestServer {
        let user = TestUser {
            authorized_keys: vec![client_key.public_key().clone()],
            ..TestUser::default()
        };
        let mut builder = TestServer::builder()
            .host_key(generate_key(Algorithm::Ed25519))
            .user(support::REMOTE_USER, user);
        match self.server_policy {
            ServerPolicy::Anchoring => {}
            ServerPolicy::RefusingX11 => builder = builder.refusing_x11(),
            ServerPolicy::ReportingEmptyDisplay => builder = builder.reporting_empty_display(),
        }
        builder.start().await
    }

    async fn environment(
        &self,
        workspace: &Workspace,
        x_server: &FakeXServer,
    ) -> FakeX11Environment {
        let authority_path = workspace.file("Xauthority");
        if matches!(self.authority, Authority::Cookie) {
            x_server.write_authority(&authority_path, &REAL_COOKIE);
        }
        FakeX11Environment {
            display: self.display(x_server).await,
            xauthority: Some(authority_path),
        }
    }
}

impl HarnessBuilder {
    async fn display(&self, x_server: &FakeXServer) -> Option<String> {
        if !self.has_display_variable {
            return None;
        }
        if self.has_x_server {
            return Some(x_server.display());
        }
        let unused_port = free_port().await;
        Some(format!("127.0.0.1:{}", unused_port - X_TCP_BASE_PORT))
    }
}

fn store_text(plain_tunnel_port: u16) -> String {
    format!(
        "version = 1\n\n[settings]\nreconnect_initial_delay_ms = {RECONNECT_INITIAL_MILLISECONDS}\nreconnect_max_delay_ms = {RECONNECT_MAXIMUM_MILLISECONDS}\nsession_idle_grace_s = 0\n\n\
[[tunnels]]\nhost = \"{host}\"\nkind = \"local\"\nlisten_port = {plain_tunnel_port}\ntarget_port = 9\n\n\
[[x11_launchers]]\nhost = \"{host}\"\nname = \"{LAUNCHER_NAME}\"\ncommand = \"{LAUNCHER_COMMAND}\"\n",
        host = support::ALIAS
    )
}

impl Harness {
    async fn wait_for_anchor(
        &mut self,
        predicate: impl Fn(&X11AnchorState) -> bool,
    ) -> X11AnchorState {
        wait_for_event(&mut self.events, WAIT, |event| match event {
            EngineEvent::X11Changed { host, state }
                if host == support::ALIAS && predicate(state) =>
            {
                Some(state.clone())
            }
            _ => None,
        })
        .await
    }

    async fn start_anchor_and_wait_active(&mut self) -> String {
        self.engine.start_x11(support::ALIAS).unwrap();
        match self
            .wait_for_anchor(|state| matches!(state, X11AnchorState::Active { .. }))
            .await
        {
            X11AnchorState::Active { display } => display,
            other => panic!("expected an active anchor, got {other:?}"),
        }
    }

    fn session_cookie(&self) -> Vec<u8> {
        decode_hex(&self.server.x11_requests()[0].cookie)
    }

    async fn wait_for_received(&self, length: usize) {
        wait_until(WAIT, "the X server to receive the setup", || {
            self.x_server.received().len() >= length
        })
        .await;
    }
}

fn is_failed(state: &X11AnchorState) -> bool {
    matches!(state, X11AnchorState::Failed { .. })
}

fn failure_message(state: X11AnchorState) -> String {
    match state {
        X11AnchorState::Failed { message } => message,
        other => panic!("expected a failed anchor, got {other:?}"),
    }
}

#[tokio::test]
async fn should_report_the_remote_display_and_count_the_anchor_when_the_anchor_is_active() {
    let mut harness = HarnessBuilder::new().start().await;

    let display = harness.start_anchor_and_wait_active().await;

    assert_eq!(display, REMOTE_DISPLAY);
    assert_eq!(harness.engine.active_x11_count(), 1);
}

#[tokio::test]
async fn should_give_the_server_a_hexadecimal_cookie_when_the_anchor_starts() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;

    let requests = harness.server.x11_requests();

    assert!(
        requests.len() == 1
            && requests[0].protocol == "MIT-MAGIC-COOKIE-1"
            && requests[0].cookie.len() == 32
            && requests[0]
                .cookie
                .chars()
                .all(|digit| digit.is_ascii_hexdigit())
            && requests[0].screen == 0
            && !requests[0].is_single_connection
    );
}

#[tokio::test]
async fn should_fail_when_the_server_refuses_x11_forwarding() {
    let mut harness = HarnessBuilder::new().refusing_x11().start().await;
    harness.engine.start_x11(support::ALIAS).unwrap();

    let state = harness.wait_for_anchor(is_failed).await;

    assert_eq!(failure_message(state), "the server refused X11 forwarding");
}

#[tokio::test]
async fn should_start_again_when_the_anchor_is_started_after_it_failed() {
    let mut harness = HarnessBuilder::new().refusing_x11().start().await;
    harness.engine.start_x11(support::ALIAS).unwrap();
    harness.wait_for_anchor(is_failed).await;

    harness.engine.start_x11(support::ALIAS).unwrap();

    let state = harness
        .wait_for_anchor(|state| *state == X11AnchorState::Starting)
        .await;
    assert_eq!(state, X11AnchorState::Starting);
}

#[tokio::test]
async fn should_fail_when_the_server_reports_an_empty_display() {
    let mut harness = HarnessBuilder::new()
        .reporting_empty_display()
        .start()
        .await;
    harness.engine.start_x11(support::ALIAS).unwrap();

    let state = harness.wait_for_anchor(is_failed).await;

    assert_eq!(
        failure_message(state),
        "the server did not set DISPLAY: is X11Forwarding enabled on the server?"
    );
}

#[tokio::test]
async fn should_fail_when_no_local_display_is_configured() {
    let mut harness = HarnessBuilder::new()
        .without_display_variable()
        .start()
        .await;
    harness.engine.start_x11(support::ALIAS).unwrap();

    let state = harness.wait_for_anchor(is_failed).await;

    assert_eq!(
        failure_message(state),
        "no local X display: set DISPLAY or the x11_display setting"
    );
}

#[tokio::test]
async fn should_stop_the_anchor_when_asked() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;

    harness.engine.stop_x11(support::ALIAS).unwrap();

    let state = harness
        .wait_for_anchor(|state| *state == X11AnchorState::Stopped)
        .await;
    assert_eq!(state, X11AnchorState::Stopped);
}

#[tokio::test]
async fn should_close_the_session_when_the_anchor_stops() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;

    harness.engine.stop_x11(support::ALIAS).unwrap();

    wait_until(WAIT, "the session to close", || {
        harness.server.open_connection_count() == 0
    })
    .await;
}

#[tokio::test]
async fn should_list_only_running_anchors_in_the_snapshot() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;
    let running = harness.engine.snapshot().x11_anchors;

    harness.engine.stop_x11(support::ALIAS).unwrap();
    harness
        .wait_for_anchor(|state| *state == X11AnchorState::Stopped)
        .await;

    assert_eq!(
        (running.len(), harness.engine.snapshot().x11_anchors.len()),
        (1, 0)
    );
}

#[tokio::test]
async fn should_ask_the_server_once_when_the_anchor_is_started_twice() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;

    harness.engine.start_x11(support::ALIAS).unwrap();
    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.server.x11_requests().len(), 1);
}

#[tokio::test]
async fn should_reconnect_when_the_session_is_cut() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;

    harness.server.cut_connections();

    let state = harness
        .wait_for_anchor(|state| matches!(state, X11AnchorState::Reconnecting { .. }))
        .await;
    assert!(matches!(
        state,
        X11AnchorState::Reconnecting { attempt: 1, .. }
    ));
}

#[tokio::test]
async fn should_be_active_again_when_the_session_comes_back() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;
    harness.server.cut_connections();
    harness
        .wait_for_anchor(|state| matches!(state, X11AnchorState::Reconnecting { .. }))
        .await;

    let state = harness
        .wait_for_anchor(|state| matches!(state, X11AnchorState::Active { .. }))
        .await;

    assert!(matches!(state, X11AnchorState::Active { .. }));
}

#[tokio::test]
async fn should_send_the_real_cookie_and_the_remaining_bytes_when_an_x_client_connects() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;
    let mut client = harness.server.open_x11_channel().await;
    let mut sent = x_client_setup_with_cookie(&harness.session_cookie());
    sent.extend(b"GetWindowAttributes");
    client.write_all(&sent).await.unwrap();
    let mut expected = expected_server_setup(&REAL_COOKIE);
    expected.extend(b"GetWindowAttributes");

    harness.wait_for_received(expected.len()).await;

    assert_eq!(harness.x_server.received(), expected);
}

#[tokio::test]
async fn should_relay_the_x_server_reply_when_an_x_client_connects() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;
    let mut client = harness.server.open_x11_channel().await;
    client
        .write_all(&x_client_setup_with_cookie(&harness.session_cookie()))
        .await
        .unwrap();
    let mut reply = vec![0_u8; X_SERVER_REPLY.len()];

    timeout(WAIT, client.read_exact(&mut reply))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(reply, X_SERVER_REPLY);
}

#[tokio::test]
async fn should_close_the_channel_when_the_x_client_presents_a_wrong_cookie() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;
    let mut client = harness.server.open_x11_channel().await;
    client
        .write_all(&x_client_setup_with_cookie(&[0xee; 16]))
        .await
        .unwrap();
    let mut buffer = [0_u8; 16];

    let count = timeout(WAIT, client.read(&mut buffer))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(count, 0);
}

#[tokio::test]
async fn should_send_nothing_to_the_x_server_when_the_x_client_presents_a_wrong_cookie() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;
    let mut client = harness.server.open_x11_channel().await;
    client
        .write_all(&x_client_setup_with_cookie(&[0xee; 16]))
        .await
        .unwrap();
    let mut buffer = [0_u8; 16];
    timeout(WAIT, client.read(&mut buffer))
        .await
        .unwrap()
        .unwrap();

    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.x_server.received().len(), 0);
}

#[tokio::test]
async fn should_close_the_channel_when_the_x_client_uses_another_protocol() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;
    let mut client = harness.server.open_x11_channel().await;
    client
        .write_all(&x_client_setup(
            b"XDM-AUTHORIZATION-1",
            &harness.session_cookie(),
        ))
        .await
        .unwrap();
    let mut buffer = [0_u8; 16];

    let count = timeout(WAIT, client.read(&mut buffer))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(count, 0);
}

#[tokio::test]
async fn should_forward_an_empty_authorization_when_there_is_no_authority_file() {
    let mut harness = HarnessBuilder::new().without_authority_file().start().await;
    harness.start_anchor_and_wait_active().await;
    let mut client = harness.server.open_x11_channel().await;
    let mut sent = x_client_setup_with_cookie(&harness.session_cookie());
    sent.extend(b"requests");
    client.write_all(&sent).await.unwrap();
    let mut expected = x_client_setup(&[], &[]);
    expected.extend(b"requests");

    harness.wait_for_received(expected.len()).await;

    assert_eq!(harness.x_server.received(), expected);
}

#[tokio::test]
async fn should_refuse_the_x11_channel_when_the_local_x_server_is_down() {
    let mut harness = HarnessBuilder::new().without_x_server().start().await;
    harness.start_anchor_and_wait_active().await;

    assert!(harness.server.try_open_x11_channel().await.is_err());
}

#[tokio::test]
async fn should_refuse_the_x11_channel_when_x11_is_not_enabled_on_the_session() {
    let mut harness = HarnessBuilder::new().start().await;
    let tunnel = plain_tunnel_id(&harness.engine);
    harness.engine.start_tunnel(tunnel).unwrap();
    wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelChanged {
            id,
            state: TunnelState::Active,
            ..
        } if *id == tunnel => Some(()),
        _ => None,
    })
    .await;

    assert!(harness.server.try_open_x11_channel().await.is_err());
}

fn plain_tunnel_id(engine: &Engine) -> TunnelId {
    engine.snapshot().tunnels.first().unwrap().definition.id
}

#[tokio::test]
async fn should_list_the_saved_launchers_in_the_snapshot() {
    let harness = HarnessBuilder::new().start().await;

    assert_eq!(
        harness.engine.snapshot().x11_launchers,
        [X11Launcher {
            host: support::ALIAS.to_owned(),
            name: LAUNCHER_NAME.to_owned(),
            command: LAUNCHER_COMMAND.to_owned(),
        }]
    );
}

#[tokio::test]
async fn should_run_the_command_with_x11_forwarding_on_the_server_when_a_launcher_is_launched() {
    let harness = HarnessBuilder::new().start().await;

    harness
        .engine
        .launch_x11(support::ALIAS, LAUNCHER_NAME)
        .await
        .unwrap();

    assert_eq!(harness.server.executed_commands(), [LAUNCHER_COMMAND]);
    assert_eq!(harness.server.x11_requests().len(), 1);
}

#[tokio::test]
async fn should_serve_x_clients_when_a_launcher_is_launched_without_the_anchor() {
    let harness = HarnessBuilder::new().start().await;
    harness
        .engine
        .launch_x11(support::ALIAS, LAUNCHER_NAME)
        .await
        .unwrap();
    let mut client = harness.server.open_x11_channel().await;
    client
        .write_all(&x_client_setup_with_cookie(&harness.session_cookie()))
        .await
        .unwrap();
    let mut reply = vec![0_u8; X_SERVER_REPLY.len()];

    timeout(WAIT, client.read_exact(&mut reply))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(reply, X_SERVER_REPLY);
}

#[tokio::test]
async fn should_fail_when_the_launcher_is_unknown() {
    let harness = HarnessBuilder::new().start().await;

    let error = harness
        .engine
        .launch_x11(support::ALIAS, "Missing")
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::UnknownLauncher { host, name } if host == support::ALIAS && name == "Missing"
    ));
}

#[tokio::test]
async fn should_fail_when_the_server_refuses_x11_forwarding_for_a_launcher() {
    let harness = HarnessBuilder::new().refusing_x11().start().await;

    let error = harness
        .engine
        .launch_x11(support::ALIAS, LAUNCHER_NAME)
        .await
        .unwrap_err();

    assert!(matches!(error, EngineError::X11(engine::X11Error::Refused)));
}

#[tokio::test]
async fn should_stop_the_anchors_when_the_engine_shuts_down() {
    let mut harness = HarnessBuilder::new().start().await;
    harness.start_anchor_and_wait_active().await;

    harness.engine.shutdown().await;

    assert_eq!(harness.engine.active_x11_count(), 0);
}
