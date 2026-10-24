#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::sync::Arc;
use std::time::Duration;

use engine::{
    Engine, EngineDependencies, EngineError, EngineOptions, Store, SystemClock, X11Error, X11Probe,
};
use support::{
    FakePortScanner, FakeX11Environment, FakeX11Servers, MemoryStateStore, Workspace, free_port,
};
use tokio::net::TcpListener;

const X_TCP_BASE_PORT: u16 = 6000;
const HOST: &str = "alpha";
const SERVER_NAME: &str = "VcXsrv";
const NON_LOOPBACK_DISPLAY: &str = "192.0.2.1:0";
const SHORT_START_TIMEOUT: Duration = Duration::from_millis(300);
const SHORT_POLL_INTERVAL: Duration = Duration::from_millis(20);

struct Harness {
    _workspace: Workspace,
    _store_directory: tempfile::TempDir,
    servers: Arc<FakeX11Servers>,
    engine: Engine,
}

impl Harness {
    fn start(display: &str, servers: FakeX11Servers) -> Self {
        let workspace = Workspace::new();
        let config_path = workspace.write_config(1, &[]);
        let connector = workspace.connector_dependencies(
            &config_path,
            support::quiet_prompter(),
            support::rejecting_verifier(),
            Arc::new(support::NoAgentLocator),
        );
        let store_directory = tempfile::tempdir().unwrap();
        let (store, _) = Store::open(store_directory.path().join("tunnels.toml")).unwrap();
        let servers = Arc::new(servers);
        let engine = Engine::start_with_options(
            EngineDependencies {
                connector,
                clock: Arc::new(SystemClock),
                store,
                port_scanner: Arc::new(FakePortScanner::default()),
                x11_environment: Arc::new(FakeX11Environment {
                    display: Some(display.to_owned()),
                    xauthority: None,
                }),
                x11_servers: servers.clone(),
                state_store: Arc::new(MemoryStateStore::default()),
                keyring: None,
                detected_openers: Vec::new(),
            },
            EngineOptions {
                x11_server_start_timeout: SHORT_START_TIMEOUT,
                x11_server_poll_interval: SHORT_POLL_INTERVAL,
                ..EngineOptions::default()
            },
        )
        .unwrap();
        Self {
            _workspace: workspace,
            _store_directory: store_directory,
            servers,
            engine,
        }
    }
}

struct FreeDisplay {
    number: u16,
}

impl FreeDisplay {
    async fn pick() -> Self {
        Self {
            number: free_port().await - X_TCP_BASE_PORT,
        }
    }

    fn text(&self) -> String {
        format!("127.0.0.1:{}", self.number)
    }
}

#[tokio::test]
async fn should_report_the_display_reachable_when_something_listens_on_it() {
    let display = FreeDisplay::pick().await;
    let _listener = TcpListener::bind(("127.0.0.1", X_TCP_BASE_PORT + display.number))
        .await
        .unwrap();
    let harness = Harness::start(&display.text(), FakeX11Servers::default());

    let probe = harness.engine.probe_x11(HOST).await.unwrap();

    assert_eq!(
        probe,
        X11Probe {
            display: format!("127.0.0.1:{}", X_TCP_BASE_PORT + display.number),
            is_reachable: true,
            server: None,
        }
    );
}

#[tokio::test]
async fn should_name_the_installed_server_when_the_local_display_is_unreachable() {
    let display = FreeDisplay::pick().await;
    let harness = Harness::start(&display.text(), FakeX11Servers::with_server(SERVER_NAME));

    let probe = harness.engine.probe_x11(HOST).await.unwrap();

    assert!(!probe.is_reachable);
    assert_eq!(probe.server.as_deref(), Some(SERVER_NAME));
}

#[tokio::test]
async fn should_name_no_server_when_the_display_is_not_on_this_machine() {
    let harness = Harness::start(
        NON_LOOPBACK_DISPLAY,
        FakeX11Servers::with_server(SERVER_NAME),
    );

    let probe = harness.engine.probe_x11(HOST).await.unwrap();

    assert!(!probe.is_reachable);
    assert_eq!(probe.server, None);
}

#[tokio::test]
async fn should_start_the_server_for_the_display_number_when_the_display_is_unreachable() {
    let display = FreeDisplay::pick().await;
    let servers = FakeX11Servers::with_server(SERVER_NAME).listening_when_started();
    let harness = Harness::start(&display.text(), servers);

    harness.engine.start_x11_server(HOST).await.unwrap();

    assert_eq!(harness.servers.started_displays(), vec![display.number]);
    assert!(harness.engine.probe_x11(HOST).await.unwrap().is_reachable);
}

#[tokio::test]
async fn should_fail_when_the_started_server_never_opens_the_display() {
    let display = FreeDisplay::pick().await;
    let harness = Harness::start(&display.text(), FakeX11Servers::with_server(SERVER_NAME));

    let outcome = harness.engine.start_x11_server(HOST).await;

    assert!(matches!(
        outcome,
        Err(EngineError::X11(X11Error::ServerDidNotStart { ref server, .. })) if server == SERVER_NAME
    ));
}

#[tokio::test]
async fn should_fail_when_no_server_is_installed() {
    let display = FreeDisplay::pick().await;
    let harness = Harness::start(&display.text(), FakeX11Servers::default());

    let outcome = harness.engine.start_x11_server(HOST).await;

    assert!(matches!(
        outcome,
        Err(EngineError::X11(X11Error::NoLocalServer))
    ));
}

#[tokio::test]
async fn should_fail_when_the_display_is_not_on_this_machine() {
    let harness = Harness::start(
        NON_LOOPBACK_DISPLAY,
        FakeX11Servers::with_server(SERVER_NAME),
    );

    let outcome = harness.engine.start_x11_server(HOST).await;

    assert!(matches!(
        outcome,
        Err(EngineError::X11(X11Error::ServerDisplayNotLocal { .. }))
    ));
    assert_eq!(harness.servers.started_displays(), Vec::<u16>::new());
}

#[tokio::test]
async fn should_not_start_the_server_when_the_display_is_already_reachable() {
    let display = FreeDisplay::pick().await;
    let _listener = TcpListener::bind(("127.0.0.1", X_TCP_BASE_PORT + display.number))
        .await
        .unwrap();
    let harness = Harness::start(&display.text(), FakeX11Servers::with_server(SERVER_NAME));

    harness.engine.start_x11_server(HOST).await.unwrap();

    assert_eq!(harness.servers.started_displays(), Vec::<u16>::new());
}
