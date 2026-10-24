#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::sync::Arc;
use std::time::Duration;

use engine::{
    ConnectError, ConnectTimeouts, Connector, Engine, EngineDependencies, EngineEvent,
    EngineOptions, FailureReason, Store, SystemClock, TunnelId, TunnelState,
};
use russh::MethodKind;
use russh::keys::{Algorithm, PrivateKey};
use support::{
    ALIAS, FakePortScanner, FakeX11Environment, FakeX11Servers, FixedAgentLocator,
    ScriptedPrompter, TestServer, TestUser, UnresponsiveAgent, Workspace, free_port, generate_key,
    wait_for_state, wait_until,
};
use tokio::sync::broadcast::Receiver;
use tokio::time::sleep;

const WAIT: Duration = Duration::from_secs(5);
const STEP_LIMIT: Duration = Duration::from_millis(300);
const LONG_LIMIT: Duration = Duration::from_secs(60);
const PATIENT_LIMIT: Duration = Duration::from_secs(1);
const SLOW_USER: Duration = Duration::from_millis(2500);
const QUIET_PERIOD: Duration = Duration::from_millis(500);
const PASSWORD: &str = "hunter2";

fn short_timeouts() -> ConnectTimeouts {
    ConnectTimeouts {
        connect: STEP_LIMIT,
        authentication_step: STEP_LIMIT,
        agent_response: STEP_LIMIT,
    }
}

fn long_timeouts() -> ConnectTimeouts {
    ConnectTimeouts {
        connect: LONG_LIMIT,
        authentication_step: LONG_LIMIT,
        agent_response: LONG_LIMIT,
    }
}

fn user_with_key(key: &PrivateKey) -> TestUser {
    TestUser {
        authorized_keys: vec![key.public_key().clone()],
        ..TestUser::default()
    }
}

async fn publickey_server(key: &PrivateKey) -> TestServer {
    TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(support::REMOTE_USER, user_with_key(key))
        .methods(&[MethodKind::PublicKey])
        .start()
        .await
}

async fn stalling_server() -> TestServer {
    TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(support::REMOTE_USER, TestUser::default())
        .stalling_authentication()
        .start()
        .await
}

async fn password_server() -> TestServer {
    let user = TestUser {
        password: Some(PASSWORD.to_owned()),
        ..TestUser::default()
    };
    TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(support::REMOTE_USER, user)
        .methods(&[MethodKind::Password])
        .start()
        .await
}

fn agent_connector(
    workspace: &Workspace,
    server: &TestServer,
    agent: &UnresponsiveAgent,
    timeouts: ConnectTimeouts,
) -> Connector {
    let config_path = workspace.write_config(server.port, &["PreferredAuthentications publickey"]);
    let locator = FixedAgentLocator {
        default_socket: Some(agent.socket_path.clone()),
    };
    workspace
        .connector_with_agent(
            &config_path,
            support::quiet_prompter(),
            support::rejecting_verifier(),
            Arc::new(locator),
        )
        .with_timeouts(timeouts)
}

#[tokio::test]
async fn should_report_an_unresponsive_agent_when_the_agent_never_signs() {
    let key = generate_key(Algorithm::Ed25519);
    let server = publickey_server(&key).await;
    let workspace = Workspace::new();
    let agent = UnresponsiveAgent::start(workspace.path(), &key);
    let connector = agent_connector(&workspace, &server, &agent, short_timeouts());

    let error = connector.connect(ALIAS).await.err().unwrap();

    assert!(matches!(
        error,
        ConnectError::AgentUnresponsive { timeout } if timeout == STEP_LIMIT
    ));
}

#[tokio::test]
async fn should_report_the_authentication_timeout_when_the_server_never_answers() {
    let server = stalling_server().await;
    let workspace = Workspace::new();
    let config_path = workspace.write_config(server.port, &["PreferredAuthentications password"]);
    let connector = workspace
        .connector(
            &config_path,
            support::quiet_prompter(),
            support::rejecting_verifier(),
        )
        .with_timeouts(short_timeouts());

    let error = connector.connect(ALIAS).await.err().unwrap();

    assert!(matches!(
        error,
        ConnectError::AuthenticationTimeout { timeout } if timeout == STEP_LIMIT
    ));
}

#[tokio::test]
async fn should_authenticate_when_the_password_is_typed_slower_than_the_limit() {
    let server = password_server().await;
    let workspace = Workspace::new();
    let config_path = workspace.write_config(server.port, &["PreferredAuthentications password"]);
    let prompter = ScriptedPrompter::default()
        .with_passwords(&[Some(PASSWORD)])
        .with_answer_delay(SLOW_USER);
    let connector = workspace
        .connector(
            &config_path,
            Arc::new(prompter),
            support::rejecting_verifier(),
        )
        .with_timeouts(ConnectTimeouts {
            authentication_step: PATIENT_LIMIT,
            ..short_timeouts()
        });

    let outcome = connector.connect(ALIAS).await;

    assert!(outcome.is_ok());
}

struct StuckTunnel {
    _workspace: Workspace,
    _store_directory: tempfile::TempDir,
    _server: TestServer,
    engine: Engine,
    events: Receiver<EngineEvent>,
    id: TunnelId,
}

impl StuckTunnel {
    async fn start(
        server: TestServer,
        agent: Option<&UnresponsiveAgent>,
        workspace: Workspace,
        timeouts: ConnectTimeouts,
    ) -> Self {
        let config_path =
            workspace.write_config(server.port, &["PreferredAuthentications publickey"]);
        let locator = agent.map(|agent| FixedAgentLocator {
            default_socket: Some(agent.socket_path.clone()),
        });
        let connector = workspace.connector_dependencies(
            &config_path,
            support::quiet_prompter(),
            support::rejecting_verifier(),
            locator.map_or_else(
                || Arc::new(support::NoAgentLocator) as _,
                |locator| Arc::new(locator) as _,
            ),
        );
        let store_directory = tempfile::tempdir().unwrap();
        let store_path = store_directory.path().join("tunnels.toml");
        std::fs::write(&store_path, store_text(free_port().await)).unwrap();
        let (store, _) = Store::open(store_path).unwrap();
        let engine = Engine::start_with_options(
            EngineDependencies {
                connector,
                clock: Arc::new(SystemClock),
                store,
                port_scanner: Arc::new(FakePortScanner::default()),
                x11_environment: Arc::new(FakeX11Environment::default()),
                x11_servers: Arc::new(FakeX11Servers::default()),
                state_store: Arc::new(support::MemoryStateStore::default()),
                keyring: None,
                detected_openers: Vec::new(),
            },
            EngineOptions {
                connect_timeouts: timeouts,
                ..EngineOptions::default()
            },
        )
        .unwrap();
        let events = engine.subscribe();
        let id = engine.snapshot().tunnels[0].definition.id;
        Self {
            _workspace: workspace,
            _store_directory: store_directory,
            _server: server,
            engine,
            events,
            id,
        }
    }

    async fn wait_for(&mut self, predicate: impl Fn(&TunnelState) -> bool) -> TunnelState {
        wait_for_state(&mut self.events, self.id, predicate, WAIT).await
    }

    fn state(&self) -> TunnelState {
        self.engine.snapshot().tunnels[0].state.clone()
    }
}

fn store_text(listen_port: u16) -> String {
    format!(
        "version = 1\n\n[settings]\nreconnect_initial_delay_ms = 20\nreconnect_max_delay_ms = 80\nsession_idle_grace_s = 0\n\n\
[[tunnels]]\nhost = \"{ALIAS}\"\nkind = \"local\"\nlisten_port = {listen_port}\ntarget_port = 9\n"
    )
}

#[tokio::test]
async fn should_fail_the_tunnel_without_retrying_when_the_agent_never_signs() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let agent = UnresponsiveAgent::start(workspace.path(), &key);
    let server = publickey_server(&key).await;
    let mut tunnel = StuckTunnel::start(server, Some(&agent), workspace, short_timeouts()).await;

    tunnel.engine.start_tunnel(tunnel.id).unwrap();
    let state = tunnel
        .wait_for(|state| matches!(state, TunnelState::Failed { .. }))
        .await;
    sleep(QUIET_PERIOD).await;

    assert_eq!(
        (state, agent.accepted_connection_count()),
        (
            TunnelState::Failed {
                reason: FailureReason::AgentUnresponsive {
                    timeout: STEP_LIMIT
                }
            },
            1
        )
    );
}

#[tokio::test]
async fn should_log_the_unresponsive_agent_when_the_tunnel_fails() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let agent = UnresponsiveAgent::start(workspace.path(), &key);
    let server = publickey_server(&key).await;
    let mut tunnel = StuckTunnel::start(server, Some(&agent), workspace, short_timeouts()).await;

    tunnel.engine.start_tunnel(tunnel.id).unwrap();
    tunnel
        .wait_for(|state| matches!(state, TunnelState::Failed { .. }))
        .await;

    let logs = tunnel.engine.logs(tunnel.id).unwrap();
    assert!(
        logs.iter()
            .any(|entry| entry.message.contains("SSH agent did not answer"))
    );
}

#[tokio::test]
async fn should_reconnect_when_the_server_stalls_authentication() {
    let workspace = Workspace::new();
    let server = stalling_server().await;
    let mut tunnel = StuckTunnel::start(server, None, workspace, short_timeouts()).await;

    tunnel.engine.start_tunnel(tunnel.id).unwrap();
    let state = tunnel
        .wait_for(|state| matches!(state, TunnelState::Reconnecting { .. }))
        .await;

    assert!(matches!(
        state,
        TunnelState::Reconnecting { attempt: 1, .. }
    ));
}

#[tokio::test]
async fn should_stop_and_abort_the_connection_attempt_when_stopping_a_starting_tunnel() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let agent = UnresponsiveAgent::start(workspace.path(), &key);
    let server = publickey_server(&key).await;
    let mut tunnel = StuckTunnel::start(server, Some(&agent), workspace, long_timeouts()).await;
    tunnel.engine.start_tunnel(tunnel.id).unwrap();
    wait_until(WAIT, "the agent to be asked to sign", || {
        agent.open_connection_count() == 1
    })
    .await;

    tunnel.engine.stop_tunnel(tunnel.id).unwrap();
    tunnel
        .wait_for(|state| matches!(state, TunnelState::Stopped))
        .await;

    wait_until(WAIT, "the abandoned attempt to drop the agent", || {
        agent.open_connection_count() == 0
    })
    .await;
}

#[tokio::test]
async fn should_begin_a_new_connection_attempt_when_restarting_an_abandoned_tunnel() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let agent = UnresponsiveAgent::start(workspace.path(), &key);
    let server = publickey_server(&key).await;
    let tunnel = StuckTunnel::start(server, Some(&agent), workspace, long_timeouts()).await;
    tunnel.engine.start_tunnel(tunnel.id).unwrap();
    wait_until(WAIT, "the first attempt to reach the agent", || {
        agent.open_connection_count() == 1
    })
    .await;
    tunnel.engine.stop_tunnel(tunnel.id).unwrap();
    wait_until(WAIT, "the first attempt to be abandoned", || {
        agent.open_connection_count() == 0
    })
    .await;

    tunnel.engine.start_tunnel(tunnel.id).unwrap();

    wait_until(WAIT, "a second attempt to reach the agent", || {
        agent.accepted_connection_count() == 2
    })
    .await;
    assert_eq!(tunnel.state(), TunnelState::Starting);
}
