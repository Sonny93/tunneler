#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use engine::{
    Clock, Engine, EngineDependencies, EngineError, EngineEvent, EngineOptions, FailureReason,
    HostKeyDecision, LogLevel, NewTunnel, Store, SystemClock, TargetHealth, TunnelId, TunnelKind,
    TunnelState,
};
use russh::keys::{Algorithm, PrivateKey};
use support::{
    EchoServer, ScriptedPrompter, ScriptedVerifier, TestServer, TestUser, Workspace, free_port,
    generate_key, wait_for_event, wait_for_state, wait_until, write_private_key,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Receiver;
use tokio::time::{sleep, timeout};

const WAIT: Duration = Duration::from_secs(5);
const RECONNECT_INITIAL: Duration = Duration::from_millis(20);
const RECONNECT_MAXIMUM: Duration = Duration::from_millis(80);
const SESSION_GRACE: Duration = Duration::from_millis(100);
const PROBE_INTERVAL: Duration = Duration::from_millis(50);
const QUIET_PERIOD: Duration = Duration::from_millis(300);
const LOOPBACK: &str = "127.0.0.1";
const PAYLOAD: &[u8] = b"hello tunnel";

struct TunnelSpec {
    id: TunnelId,
    name: String,
    kind: &'static str,
    listen_port: u16,
    target_port: u16,
    tags: Vec<String>,
}

#[derive(Clone, Copy)]
struct Keepalive {
    interval: Duration,
    maximum_unanswered: u32,
}

struct Timing {
    reconnect_initial: Duration,
    reconnect_maximum: Duration,
    session_grace: Duration,
}

struct HarnessBuilder {
    tunnels: Vec<(&'static str, Vec<&'static str>)>,
    kind: &'static str,
    timing: Timing,
    is_health_probe_enabled: bool,
    keepalive: Option<Keepalive>,
    is_client_key_authorized: bool,
    is_target_up: bool,
    host_lines: Vec<&'static str>,
    verifier: ScriptedVerifier,
    clock: Arc<dyn Clock>,
}

struct Harness {
    _workspace: Workspace,
    store_directory: tempfile::TempDir,
    server: TestServer,
    echo: EchoServer,
    engine: Engine,
    events: Receiver<EngineEvent>,
    prompter: Arc<ScriptedPrompter>,
    verifier: Arc<ScriptedVerifier>,
    specs: Vec<TunnelSpec>,
    timing: Timing,
}

impl HarnessBuilder {
    fn new(tunnels: &[&'static str]) -> Self {
        Self {
            tunnels: tunnels.iter().map(|name| (*name, Vec::new())).collect(),
            kind: "local",
            timing: Timing {
                reconnect_initial: RECONNECT_INITIAL,
                reconnect_maximum: RECONNECT_MAXIMUM,
                session_grace: SESSION_GRACE,
            },
            is_health_probe_enabled: true,
            keepalive: None,
            is_client_key_authorized: true,
            is_target_up: true,
            host_lines: Vec::new(),
            verifier: ScriptedVerifier::default(),
            clock: Arc::new(SystemClock),
        }
    }

    fn tagged(tunnels: &[(&'static str, &[&'static str])]) -> Self {
        let mut builder = Self::new(&[]);
        builder.tunnels = tunnels
            .iter()
            .map(|(name, tags)| (*name, tags.to_vec()))
            .collect();
        builder
    }

    fn kind(mut self, kind: &'static str) -> Self {
        self.kind = kind;
        self
    }

    fn reconnect_delays(mut self, initial: Duration, maximum: Duration) -> Self {
        self.timing.reconnect_initial = initial;
        self.timing.reconnect_maximum = maximum;
        self
    }

    fn without_health_probe_setting(mut self) -> Self {
        self.is_health_probe_enabled = false;
        self
    }

    fn keepalive(mut self, interval: Duration, maximum_unanswered: u32) -> Self {
        self.keepalive = Some(Keepalive {
            interval,
            maximum_unanswered,
        });
        self
    }

    fn with_unauthorized_client_key(mut self) -> Self {
        self.is_client_key_authorized = false;
        self
    }

    fn with_target_down(mut self) -> Self {
        self.is_target_up = false;
        self
    }

    fn host_line(mut self, line: &'static str) -> Self {
        self.host_lines.push(line);
        self
    }

    fn verifier(mut self, verifier: ScriptedVerifier) -> Self {
        self.verifier = verifier;
        self
    }

    fn clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    async fn start(self) -> Harness {
        let client_key = generate_key(Algorithm::Ed25519);
        let server = server_authorizing(&client_key, self.is_client_key_authorized).await;
        let echo = EchoServer::start().await;
        let target_port = if self.is_target_up {
            echo.port
        } else {
            free_port().await
        };
        let specs = self.specs(target_port).await;
        let workspace = Workspace::new();
        let key_path = write_private_key(workspace.path(), "id_test", &client_key, None);
        let identity_line = format!("IdentityFile {}", key_path.display());
        let mut lines = vec![
            identity_line.as_str(),
            "IdentitiesOnly yes",
            "PreferredAuthentications publickey",
        ];
        lines.extend(self.host_lines.iter().copied());
        let config_path = workspace.write_config(server.port, &lines);
        let prompter = support::quiet_prompter();
        let verifier = Arc::new(self.verifier);
        let dependencies = workspace.connector_dependencies(
            &config_path,
            Arc::clone(&prompter) as _,
            Arc::clone(&verifier) as _,
            Arc::new(support::NoAgentLocator),
        );
        let store_directory = tempfile::tempdir().unwrap();
        let store_path = store_directory.path().join("tunnels.toml");
        std::fs::write(&store_path, store_text(&specs, &self.timing)).unwrap();
        let (store, _) = Store::open(store_path).unwrap();
        apply_sub_second_settings(
            &store,
            &self.timing,
            self.is_health_probe_enabled,
            self.keepalive,
        );
        let engine = Engine::start_with_options(
            EngineDependencies {
                connector: dependencies,
                clock: self.clock,
                store,
            },
            EngineOptions {
                health_probe_interval: PROBE_INTERVAL,
                metrics_event_interval: PROBE_INTERVAL,
            },
        )
        .unwrap();
        let events = engine.subscribe();
        Harness {
            _workspace: workspace,
            store_directory,
            server,
            echo,
            engine,
            events,
            prompter,
            verifier,
            specs,
            timing: self.timing,
        }
    }

    async fn specs(&self, target_port: u16) -> Vec<TunnelSpec> {
        let mut specs = Vec::new();
        for (name, tags) in &self.tunnels {
            specs.push(TunnelSpec {
                id: TunnelId::generate(),
                name: (*name).to_owned(),
                kind: self.kind,
                listen_port: free_port().await,
                target_port,
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            });
        }
        specs
    }
}

async fn server_authorizing(client_key: &PrivateKey, is_authorized: bool) -> TestServer {
    let authorized_key = if is_authorized {
        client_key.clone()
    } else {
        generate_key(Algorithm::Ed25519)
    };
    let user = TestUser {
        authorized_keys: vec![authorized_key.public_key().clone()],
        ..TestUser::default()
    };
    TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(support::REMOTE_USER, user)
        .start()
        .await
}

fn store_text(specs: &[TunnelSpec], timing: &Timing) -> String {
    let mut text = format!(
        "version = 1\n\n[settings]\nreconnect_initial_delay_ms = {}\nreconnect_max_delay_ms = {}\nsession_idle_grace_s = 0\n\n",
        timing.reconnect_initial.as_millis(),
        timing.reconnect_maximum.as_millis()
    );
    for spec in specs {
        text.push_str(&tunnel_block(spec));
    }
    text
}

fn tunnel_block(spec: &TunnelSpec) -> String {
    let tags: Vec<String> = spec.tags.iter().map(|tag| format!("\"{tag}\"")).collect();
    format!(
        "[[tunnels]]\nid = \"{}\"\nname = \"{}\"\nhost = \"{}\"\nkind = \"{}\"\nlisten_port = {}\ntarget_port = {}\ntags = [{}]\n\n",
        spec.id,
        spec.name,
        support::ALIAS,
        spec.kind,
        spec.listen_port,
        spec.target_port,
        tags.join(", ")
    )
}

fn apply_sub_second_settings(
    store: &Store,
    timing: &Timing,
    is_health_probe_enabled: bool,
    keepalive: Option<Keepalive>,
) {
    let mut document = store.document();
    if let Some(keepalive) = keepalive {
        document.settings.keepalive_interval = keepalive.interval;
        document.settings.keepalive_max = keepalive.maximum_unanswered;
    }
    document.settings.session_idle_grace = timing.session_grace;
    document.settings.health_probe = is_health_probe_enabled;
    store.save(document).unwrap();
}

impl Harness {
    fn id(&self, name: &str) -> TunnelId {
        self.spec(name).id
    }

    fn spec(&self, name: &str) -> &TunnelSpec {
        self.specs.iter().find(|spec| spec.name == name).unwrap()
    }

    fn port(&self, name: &str) -> u16 {
        self.spec(name).listen_port
    }

    fn store_path(&self) -> PathBuf {
        self.store_directory.path().join("tunnels.toml")
    }

    fn state_of(&self, name: &str) -> TunnelState {
        self.engine
            .snapshot()
            .tunnels
            .into_iter()
            .find(|tunnel| tunnel.definition.id == self.id(name))
            .unwrap()
            .state
    }

    fn start_tunnel(&self, name: &str) {
        self.engine.start_tunnel(self.id(name)).unwrap();
    }

    fn stop_tunnel(&self, name: &str) {
        self.engine.stop_tunnel(self.id(name)).unwrap();
    }

    async fn wait_for(
        &mut self,
        name: &str,
        predicate: impl Fn(&TunnelState) -> bool,
    ) -> TunnelState {
        let id = self.id(name);
        wait_for_state(&mut self.events, id, predicate, WAIT).await
    }

    async fn wait_all(&mut self, names: &[&str], predicate: impl Fn(&TunnelState) -> bool) {
        let mut pending: Vec<TunnelId> = names.iter().map(|name| self.id(name)).collect();
        wait_for_event(&mut self.events, WAIT, |event| {
            if let EngineEvent::TunnelChanged { id, state, .. } = event
                && predicate(state)
            {
                pending.retain(|waiting| waiting != id);
            }
            pending.is_empty().then_some(())
        })
        .await;
    }

    async fn wait_all_active(&mut self, names: &[&str]) {
        self.wait_all(names, |state| matches!(state, TunnelState::Active))
            .await;
    }

    async fn wait_all_stopped(&mut self, names: &[&str]) {
        self.wait_all(names, |state| matches!(state, TunnelState::Stopped))
            .await;
    }

    async fn wait_active(&mut self, name: &str) {
        self.wait_for(name, |state| matches!(state, TunnelState::Active))
            .await;
    }

    async fn wait_stopped(&mut self, name: &str) {
        self.wait_for(name, |state| matches!(state, TunnelState::Stopped))
            .await;
    }

    async fn start_and_wait_active(&mut self, name: &str) {
        self.start_tunnel(name);
        self.wait_active(name).await;
    }

    async fn wait_for_config_reload(&mut self) {
        wait_for_event(&mut self.events, WAIT, |event| {
            matches!(event, EngineEvent::ConfigReloaded).then_some(())
        })
        .await;
    }

    fn rewrite_store(&self) {
        std::fs::write(self.store_path(), store_text(&self.specs, &self.timing)).unwrap();
    }

    fn target_requests(&self) -> usize {
        let target = (LOOPBACK.to_owned(), self.echo.port);
        self.server
            .direct_tcpip_targets()
            .into_iter()
            .filter(|requested| *requested == target)
            .count()
    }
}

async fn try_echo_through(port: u16, payload: &[u8]) -> io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect((LOOPBACK, port)).await?;
    stream.write_all(payload).await?;
    let mut received = vec![0_u8; payload.len()];
    timeout(WAIT, stream.read_exact(&mut received))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "no echo within the bound"))??;
    Ok(received)
}

async fn echo_through(port: u16, payload: &[u8]) -> Vec<u8> {
    try_echo_through(port, payload).await.unwrap()
}

async fn is_closed_by_the_tunnel(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect((LOOPBACK, port)).await else {
        return true;
    };
    let mut buffer = [0_u8; 1];
    matches!(
        timeout(WAIT, stream.read(&mut buffer)).await,
        Ok(Ok(0) | Err(_))
    )
}

async fn can_bind(port: u16) -> bool {
    TcpListener::bind((LOOPBACK, port)).await.is_ok()
}

fn reconnecting_attempt(state: &TunnelState) -> Option<(u32, SystemTime)> {
    match state {
        TunnelState::Reconnecting {
            attempt,
            next_retry_at,
        } => Some((*attempt, *next_retry_at)),
        _ => None,
    }
}

#[tokio::test]
async fn should_share_one_session_when_two_tunnels_use_the_same_host() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"]).start().await;

    harness.start_and_wait_active("alpha").await;
    harness.start_and_wait_active("beta").await;
    echo_through(harness.port("alpha"), PAYLOAD).await;
    echo_through(harness.port("beta"), PAYLOAD).await;

    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_keep_the_other_tunnel_forwarding_when_one_is_stopped() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.start_and_wait_active("beta").await;

    harness.stop_tunnel("alpha");
    harness.wait_stopped("alpha").await;

    assert_eq!(echo_through(harness.port("beta"), PAYLOAD).await, PAYLOAD);
    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_release_the_port_when_the_tunnel_is_stopped() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.stop_tunnel("alpha");
    harness.wait_stopped("alpha").await;

    assert!(can_bind(harness.port("alpha")).await);
}

#[tokio::test]
async fn should_drop_live_connections_when_the_tunnel_is_stopped() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let mut client = TcpStream::connect((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();
    client.write_all(PAYLOAD).await.unwrap();
    let mut echoed = vec![0_u8; PAYLOAD.len()];
    client.read_exact(&mut echoed).await.unwrap();

    harness.stop_tunnel("alpha");
    harness.wait_stopped("alpha").await;

    let mut buffer = [0_u8; 1];
    let outcome = timeout(WAIT, client.read(&mut buffer)).await.unwrap();
    assert!(matches!(outcome, Ok(0) | Err(_)), "{outcome:?}");
}

#[tokio::test]
async fn should_count_active_tunnels_when_some_run() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"]).start().await;
    assert_eq!(harness.engine.active_tunnel_count(), 0);

    harness.start_and_wait_active("alpha").await;

    assert_eq!(harness.engine.active_tunnel_count(), 1);
}

#[tokio::test]
async fn should_ignore_start_when_the_tunnel_is_already_active() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.start_tunnel("alpha");
    let client = echo_through(harness.port("alpha"), PAYLOAD).await;

    assert_eq!(client, PAYLOAD);
    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_report_the_client_address_to_the_server_when_forwarding() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let mut client = TcpStream::connect((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();
    let client_port = client.local_addr().unwrap().port();

    client.write_all(PAYLOAD).await.unwrap();
    let mut echoed = vec![0_u8; PAYLOAD.len()];
    client.read_exact(&mut echoed).await.unwrap();

    assert_eq!(
        harness.server.direct_tcpip_originators(),
        vec![(LOOPBACK.to_owned(), u32::from(client_port))]
    );
}

#[tokio::test]
async fn should_reconnect_and_forward_again_when_the_connection_is_cut() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.server.cut_connections();
    let reconnecting = harness
        .wait_for("alpha", |state| reconnecting_attempt(state).is_some())
        .await;
    harness.wait_active("alpha").await;

    assert_eq!(reconnecting_attempt(&reconnecting).unwrap().0, 1);
    assert_eq!(echo_through(harness.port("alpha"), PAYLOAD).await, PAYLOAD);
    assert_eq!(harness.server.connection_count(), 2);
}

#[tokio::test]
async fn should_keep_the_port_bound_when_the_server_is_down() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.server.stop().await;
    harness
        .wait_for("alpha", |state| reconnecting_attempt(state).is_some())
        .await;

    assert!(!can_bind(harness.port("alpha")).await);
}

#[tokio::test]
async fn should_close_connections_immediately_and_log_them_when_there_is_no_session() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.server.stop().await;
    harness
        .wait_for("alpha", |state| reconnecting_attempt(state).is_some())
        .await;

    assert!(is_closed_by_the_tunnel(harness.port("alpha")).await);

    let logs = harness.engine.logs(harness.id("alpha")).unwrap();
    assert!(
        logs.iter()
            .any(|entry| entry.message.contains("no SSH session available")),
        "{logs:#?}"
    );
}

#[tokio::test]
async fn should_back_off_with_increasing_attempts_when_the_server_stays_down() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.server.stop().await;
    let id = harness.id("alpha");
    let mut retries = Vec::new();

    wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelChanged {
            id: changed, state, ..
        } if *changed == id => {
            retries.extend(reconnecting_attempt(state));
            Some(()).filter(|()| retries.len() >= 3)
        }
        _ => None,
    })
    .await;

    let attempts: Vec<u32> = retries.iter().map(|(attempt, _)| *attempt).collect();
    let is_retry_time_increasing = retries.windows(2).all(|pair| pair[0].1 < pair[1].1);
    assert_eq!(attempts, vec![1, 2, 3]);
    assert!(is_retry_time_increasing, "{retries:?}");
}

#[tokio::test]
async fn should_forward_again_and_restart_the_attempt_count_when_the_server_comes_back() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.server.stop().await;
    harness
        .wait_for("alpha", |state| {
            reconnecting_attempt(state).is_some_and(|(attempt, _)| attempt >= 2)
        })
        .await;

    harness.server.restart().await;
    harness.wait_active("alpha").await;
    harness.server.cut_connections();
    let reconnecting = harness
        .wait_for("alpha", |state| reconnecting_attempt(state).is_some())
        .await;

    assert_eq!(reconnecting_attempt(&reconnecting).unwrap().0, 1);
    harness.wait_active("alpha").await;
    assert_eq!(echo_through(harness.port("alpha"), PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_cancel_the_reconnection_immediately_when_the_tunnel_is_stopped() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .reconnect_delays(Duration::from_secs(60), Duration::from_secs(60))
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;
    harness.server.stop().await;
    harness
        .wait_for("alpha", |state| reconnecting_attempt(state).is_some())
        .await;

    harness.stop_tunnel("alpha");

    harness.wait_stopped("alpha").await;
    assert!(can_bind(harness.port("alpha")).await);
}

#[tokio::test]
async fn should_fail_without_any_ssh_connection_when_the_port_is_taken() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let port = harness.port("alpha");
    let _squatter = TcpListener::bind((LOOPBACK, port)).await.unwrap();

    harness.start_tunnel("alpha");
    let state = harness
        .wait_for("alpha", |state| matches!(state, TunnelState::Failed { .. }))
        .await;

    assert_eq!(
        state,
        TunnelState::Failed {
            reason: FailureReason::PortInUse {
                address: format!("{LOOPBACK}:{port}")
            }
        }
    );
    assert_eq!(harness.server.connection_count(), 0);
}

#[tokio::test]
async fn should_start_again_when_the_port_is_freed_after_a_failure() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let squatter = TcpListener::bind((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();
    harness.start_tunnel("alpha");
    harness
        .wait_for("alpha", |state| matches!(state, TunnelState::Failed { .. }))
        .await;

    drop(squatter);
    harness.start_and_wait_active("alpha").await;

    assert_eq!(echo_through(harness.port("alpha"), PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_fail_without_retrying_when_authentication_fails() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .with_unauthorized_client_key()
        .start()
        .await;

    harness.start_tunnel("alpha");
    let state = harness
        .wait_for("alpha", |state| matches!(state, TunnelState::Failed { .. }))
        .await;
    sleep(QUIET_PERIOD).await;

    assert!(
        matches!(&state, TunnelState::Failed { reason: FailureReason::Connect { message } } if message.contains("authentication failed")),
        "{state:?}"
    );
    assert_eq!(harness.server.connection_count(), 1);
    assert!(matches!(
        harness.state_of("alpha"),
        TunnelState::Failed { .. }
    ));
}

#[tokio::test]
async fn should_fail_with_unsupported_kind_when_the_tunnel_is_remote() {
    let mut harness = HarnessBuilder::new(&["alpha"]).kind("remote").start().await;

    harness.start_tunnel("alpha");
    let state = harness
        .wait_for("alpha", |state| matches!(state, TunnelState::Failed { .. }))
        .await;

    assert_eq!(
        state,
        TunnelState::Failed {
            reason: FailureReason::UnsupportedKind(TunnelKind::Remote)
        }
    );
    assert_eq!(harness.server.connection_count(), 0);
}

#[tokio::test]
async fn should_reuse_the_session_when_the_tunnel_restarts_within_the_grace_period() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.stop_tunnel("alpha");
    harness.wait_stopped("alpha").await;
    harness.start_and_wait_active("alpha").await;

    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_reuse_the_session_when_the_tunnel_is_restarted() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.engine.restart_tunnel(harness.id("alpha")).unwrap();
    harness
        .wait_for("alpha", |state| matches!(state, TunnelState::Starting))
        .await;
    harness.wait_active("alpha").await;

    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_close_the_session_when_the_grace_period_passed_without_tunnels() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.stop_tunnel("alpha");
    harness.wait_stopped("alpha").await;

    let server = &harness.server;
    wait_until(WAIT, "the server to see the session close", || {
        server.open_connection_count() == 0
    })
    .await;
}

#[tokio::test]
async fn should_open_a_new_session_when_a_tunnel_starts_after_the_grace_period() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.stop_tunnel("alpha");
    harness.wait_stopped("alpha").await;
    let server = &harness.server;
    wait_until(WAIT, "the server to see the session close", || {
        server.open_connection_count() == 0
    })
    .await;

    harness.start_and_wait_active("alpha").await;

    assert_eq!(harness.server.connection_count(), 2);
}

#[tokio::test]
async fn should_prompt_once_when_two_tunnels_start_on_a_host_with_an_unknown_key() {
    let verifier = ScriptedVerifier::default()
        .with_decisions(&[HostKeyDecision::AcceptOnce])
        .with_decision_delay(Duration::from_millis(150));
    let mut harness = HarnessBuilder::new(&["alpha", "beta"])
        .host_line("StrictHostKeyChecking ask")
        .verifier(verifier)
        .start()
        .await;

    harness.start_tunnel("alpha");
    harness.start_tunnel("beta");
    harness.wait_all_active(&["alpha", "beta"]).await;

    assert_eq!(harness.verifier.unknown_requests().len(), 1);
    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_mark_the_target_unreachable_while_the_tunnel_stays_active_when_the_target_is_down()
{
    let mut harness = HarnessBuilder::new(&["alpha"])
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;

    assert!(is_closed_by_the_tunnel(harness.port("alpha")).await);

    let (state, health) = wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelChanged {
            state,
            health: health @ TargetHealth::Unreachable { .. },
            ..
        } => Some((state.clone(), health.clone())),
        _ => None,
    })
    .await;
    assert!(matches!(state, TunnelState::Active), "{state:?}");
    assert!(matches!(health, TargetHealth::Unreachable { .. }));
}

#[tokio::test]
async fn should_mark_the_target_reachable_when_it_recovers() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;
    assert!(is_closed_by_the_tunnel(harness.port("alpha")).await);
    let target_port = harness.spec("alpha").target_port;
    let _recovered_target = EchoServer::start_on(target_port).await;

    echo_through(harness.port("alpha"), PAYLOAD).await;

    wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelChanged {
            health: TargetHealth::Reachable,
            ..
        } => Some(()),
        _ => None,
    })
    .await;
}

#[tokio::test]
async fn should_count_the_bytes_when_data_goes_through_the_tunnel() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let payload = vec![7_u8; 4000];

    echo_through(harness.port("alpha"), &payload).await;

    let expected = u64::try_from(payload.len()).unwrap();
    let id = harness.id("alpha");
    let metrics = wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelMetrics {
            id: changed,
            metrics,
        } if *changed == id && metrics.bytes_out == expected && metrics.bytes_in == expected => {
            Some(*metrics)
        }
        _ => None,
    })
    .await;
    assert_eq!(metrics.total_connections, 1);
    assert!(metrics.last_activity.is_some());
}

#[tokio::test]
async fn should_report_active_connections_in_the_metrics_when_clients_connect_and_leave() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let id = harness.id("alpha");
    let client = TcpStream::connect((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();

    wait_for_active_connections(&mut harness.events, id, 1).await;
    drop(client);

    wait_for_active_connections(&mut harness.events, id, 0).await;
}

async fn wait_for_active_connections(
    events: &mut Receiver<EngineEvent>,
    id: TunnelId,
    expected: usize,
) {
    wait_for_event(events, WAIT, |event| match event {
        EngineEvent::TunnelMetrics {
            id: changed,
            metrics,
        } if *changed == id && metrics.active_connections == expected => Some(()),
        _ => None,
    })
    .await;
}

#[tokio::test]
async fn should_not_publish_a_state_change_when_a_client_connects() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let id = harness.id("alpha");
    let client = TcpStream::connect((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();
    wait_for_active_connections(&mut harness.events, id, 1).await;
    drop(client);
    wait_for_active_connections(&mut harness.events, id, 0).await;

    let state_changes = std::iter::from_fn(|| harness.events.try_recv().ok())
        .filter(|event| matches!(event, EngineEvent::TunnelChanged { .. }))
        .count();

    assert_eq!(state_changes, 0);
}

#[tokio::test]
async fn should_not_probe_the_target_when_probing_is_off() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.target_requests(), 0);
}

#[tokio::test]
async fn should_probe_the_target_when_probing_is_on() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.engine.set_health_probing(true);

    let harness = &harness;
    wait_until(WAIT, "two probes to reach the target", || {
        harness.target_requests() >= 2
    })
    .await;
}

#[tokio::test]
async fn should_mark_the_target_reachable_when_a_probe_succeeds() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.engine.set_health_probing(true);

    wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelChanged {
            health: TargetHealth::Reachable,
            ..
        } => Some(()),
        _ => None,
    })
    .await;
}

#[tokio::test]
async fn should_mark_the_target_unreachable_when_a_probe_finds_it_down() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;

    harness.engine.set_health_probing(true);

    wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelChanged {
            health: TargetHealth::Unreachable { .. },
            ..
        } => Some(()),
        _ => None,
    })
    .await;
}

#[tokio::test]
async fn should_stop_probing_when_probing_is_turned_off() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.engine.set_health_probing(true);
    {
        let harness = &harness;
        wait_until(WAIT, "a probe to reach the target", || {
            harness.target_requests() >= 1
        })
        .await;
    }

    harness.engine.set_health_probing(false);
    sleep(PROBE_INTERVAL * 2).await;
    let requests_after_settling = harness.target_requests();
    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.target_requests(), requests_after_settling);
}

#[tokio::test]
async fn should_not_probe_when_the_health_probe_setting_is_off() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .without_health_probe_setting()
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;

    harness.engine.set_health_probing(true);
    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.target_requests(), 0);
}

#[tokio::test]
async fn should_start_and_stop_exactly_the_tagged_tunnels_when_toggling_a_tag() {
    let mut harness = HarnessBuilder::tagged(&[
        ("alpha", &["dev"]),
        ("beta", &["dev", "web"]),
        ("gamma", &["web"]),
    ])
    .start()
    .await;

    let started = harness.engine.start_tag("dev").unwrap();
    harness.wait_all_active(&["alpha", "beta"]).await;

    assert_eq!(started, vec![harness.id("alpha"), harness.id("beta")]);
    assert_eq!(harness.state_of("gamma"), TunnelState::Stopped);
    harness.engine.stop_tag("dev").unwrap();
    harness.wait_all_stopped(&["alpha", "beta"]).await;
    assert_eq!(harness.engine.active_tunnel_count(), 0);
}

#[tokio::test]
async fn should_fail_with_unknown_tag_when_no_tunnel_carries_it() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;

    let outcome = harness.engine.start_tag("missing");

    assert!(matches!(outcome, Err(EngineError::UnknownTag(tag)) if tag == "missing"));
}

#[tokio::test]
async fn should_fail_with_unknown_tunnel_when_the_id_does_not_exist() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let unknown = TunnelId::generate();

    assert!(matches!(
        harness.engine.start_tunnel(unknown),
        Err(EngineError::UnknownTunnel(id)) if id == unknown
    ));
    assert!(matches!(
        harness.engine.logs(unknown),
        Err(EngineError::UnknownTunnel(_))
    ));
    assert!(matches!(
        harness.engine.delete_tunnel(unknown).await,
        Err(EngineError::UnknownTunnel(_))
    ));
}

#[tokio::test]
async fn should_restart_the_tunnel_on_the_new_port_when_the_file_changes_its_listen_port() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let old_port = harness.port("alpha");
    let new_port = free_port().await;

    harness.specs[0].listen_port = new_port;
    harness.rewrite_store();
    harness
        .wait_for("alpha", |state| matches!(state, TunnelState::Starting))
        .await;
    harness.wait_active("alpha").await;

    assert_eq!(echo_through(new_port, PAYLOAD).await, PAYLOAD);
    assert!(can_bind(old_port).await);
}

#[tokio::test]
async fn should_not_restart_the_tunnel_when_the_file_only_renames_it() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let mut client = TcpStream::connect((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();

    harness.specs[0].name = "renamed".to_owned();
    harness.rewrite_store();
    harness.wait_for_config_reload().await;

    client.write_all(PAYLOAD).await.unwrap();
    let mut echoed = vec![0_u8; PAYLOAD.len()];
    timeout(WAIT, client.read_exact(&mut echoed))
        .await
        .unwrap()
        .unwrap();
    let snapshot = harness.engine.snapshot();
    assert_eq!(snapshot.tunnels[0].definition.name, "renamed");
    assert!(matches!(snapshot.tunnels[0].state, TunnelState::Active));
    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_stop_the_tunnel_when_the_file_deletes_it() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.start_and_wait_active("beta").await;
    let port = harness.port("alpha");

    harness.specs.remove(0);
    harness.rewrite_store();
    harness.wait_for_config_reload().await;

    let remaining: Vec<String> = harness
        .engine
        .snapshot()
        .tunnels
        .into_iter()
        .map(|tunnel| tunnel.definition.name)
        .collect();
    assert_eq!(remaining, vec!["beta".to_owned()]);
    assert!(can_bind(port).await);
    assert_eq!(echo_through(harness.port("beta"), PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_add_a_stopped_tunnel_and_save_its_id_when_the_file_gains_a_tunnel_without_id() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let text = format!(
        "{}[[tunnels]]\nname = \"added\"\nhost = \"{}\"\nkind = \"local\"\nlisten_port = {}\n",
        store_text(&harness.specs, &harness.timing),
        support::ALIAS,
        free_port().await
    );

    std::fs::write(harness.store_path(), text).unwrap();
    harness.wait_for_config_reload().await;

    let snapshot = harness.engine.snapshot();
    assert_eq!(snapshot.tunnels[1].definition.name, "added");
    assert_eq!(snapshot.tunnels[1].state, TunnelState::Stopped);
    let path = harness.store_path();
    wait_until(WAIT, "the generated id to be saved", || {
        std::fs::read_to_string(&path)
            .unwrap()
            .matches("id = ")
            .count()
            == 2
    })
    .await;
}

#[tokio::test]
async fn should_report_the_file_as_invalid_and_keep_running_when_the_file_breaks() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    std::fs::write(harness.store_path(), "[[tunnels]\n").unwrap();
    let message = wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::ConfigInvalid { message } => Some(message.clone()),
        _ => None,
    })
    .await;

    assert_ne!(message, "");
    assert!(matches!(harness.state_of("alpha"), TunnelState::Active));
    assert_eq!(echo_through(harness.port("alpha"), PAYLOAD).await, PAYLOAD);
}

fn new_tunnel(name: &str, listen_port: u16) -> NewTunnel {
    NewTunnel {
        name: name.to_owned(),
        host: support::ALIAS.to_owned(),
        kind: TunnelKind::Local,
        listen_port,
        listen_address: None,
        target_host: None,
        target_port: None,
        tags: vec!["fresh".to_owned()],
    }
}

fn saved_tunnel_names(harness: &Harness) -> Vec<String> {
    let (store, _) = Store::open(harness.store_path()).unwrap();
    store
        .document()
        .tunnels
        .into_iter()
        .map(|tunnel| tunnel.name)
        .collect()
}

#[tokio::test]
async fn should_persist_the_tunnel_when_it_is_created() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let id = harness
        .engine
        .create_tunnel(new_tunnel("created", free_port().await))
        .await
        .unwrap();

    assert_eq!(saved_tunnel_names(&harness), ["alpha", "created"]);
    let created = harness.engine.snapshot().tunnels.pop().unwrap();
    assert_eq!(
        (created.definition.id, created.state),
        (id, TunnelState::Stopped)
    );
}

#[tokio::test]
async fn should_refuse_to_create_a_tunnel_when_the_name_is_taken() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let outcome = harness
        .engine
        .create_tunnel(new_tunnel("alpha", free_port().await))
        .await;

    assert!(matches!(outcome, Err(EngineError::Store(_))), "{outcome:?}");
    assert_eq!(harness.engine.snapshot().tunnels.len(), 1);
}

#[tokio::test]
async fn should_persist_the_change_when_a_tunnel_is_updated() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let mut definition = harness.engine.snapshot().tunnels.remove(0).definition;

    definition.name = "renamed".to_owned();
    harness.engine.update_tunnel(definition).await.unwrap();

    assert_eq!(saved_tunnel_names(&harness), ["renamed"]);
    assert_eq!(
        harness.engine.snapshot().tunnels[0].definition.name,
        "renamed"
    );
}

#[tokio::test]
async fn should_restart_a_running_tunnel_on_the_new_port_when_it_is_updated() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let mut definition = harness.engine.snapshot().tunnels.remove(0).definition;
    let new_port = free_port().await;

    definition.listen_port = new_port;
    harness.engine.update_tunnel(definition).await.unwrap();
    harness
        .wait_for("alpha", |state| matches!(state, TunnelState::Starting))
        .await;
    harness.wait_active("alpha").await;

    assert_eq!(echo_through(new_port, PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_fail_with_unknown_tunnel_when_updating_a_tunnel_that_does_not_exist() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let mut definition = harness.engine.snapshot().tunnels.remove(0).definition;
    definition.id = TunnelId::generate();

    let outcome = harness.engine.update_tunnel(definition).await;

    assert!(matches!(outcome, Err(EngineError::UnknownTunnel(_))));
}

#[tokio::test]
async fn should_persist_and_stop_the_tunnel_when_it_is_deleted() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let port = harness.port("alpha");

    harness
        .engine
        .delete_tunnel(harness.id("alpha"))
        .await
        .unwrap();

    assert_eq!(saved_tunnel_names(&harness), ["beta"]);
    assert_eq!(harness.engine.snapshot().tunnels.len(), 1);
    assert!(can_bind(port).await);
}

#[tokio::test]
async fn should_not_reload_again_when_the_engine_writes_its_own_file() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness
        .engine
        .create_tunnel(new_tunnel("created", free_port().await))
        .await
        .unwrap();
    harness.wait_for_config_reload().await;

    let outcome = timeout(Duration::from_millis(800), harness.wait_for_config_reload()).await;

    assert!(outcome.is_err(), "the engine reloaded its own write");
}

#[tokio::test]
async fn should_stop_everything_and_close_the_session_when_the_engine_shuts_down() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.start_and_wait_active("beta").await;
    let ports = [harness.port("alpha"), harness.port("beta")];

    harness.engine.shutdown().await;

    let server = &harness.server;
    wait_until(WAIT, "the server to see the session close", || {
        server.open_connection_count() == 0
    })
    .await;
    assert!(can_bind(ports[0]).await && can_bind(ports[1]).await);
    assert_eq!(harness.engine.active_tunnel_count(), 0);
    assert!(matches!(
        harness.engine.start_tunnel(harness.id("alpha")),
        Err(EngineError::ShutDown)
    ));
}

#[tokio::test]
async fn should_publish_stopped_for_every_tunnel_when_the_engine_shuts_down() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.engine.shutdown().await;

    harness.wait_stopped("alpha").await;
}

#[tokio::test]
async fn should_log_state_transitions_and_emit_each_entry_when_a_tunnel_starts() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let id = harness.id("alpha");
    let mut log_events = harness.engine.subscribe();

    harness.start_and_wait_active("alpha").await;

    let messages: Vec<String> = harness
        .engine
        .logs(id)
        .unwrap()
        .into_iter()
        .map(|entry| entry.message)
        .collect();
    assert_eq!(messages, ["state: starting", "state: active"]);
    let emitted = wait_for_event(&mut log_events, WAIT, |event| match event {
        EngineEvent::TunnelLog { entry, .. } if entry.message == "state: starting" => {
            Some(entry.level)
        }
        _ => None,
    })
    .await;
    assert_eq!(emitted, LogLevel::Info);
}

#[tokio::test]
async fn should_log_the_reconnection_when_the_session_is_lost() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness.server.cut_connections();
    harness
        .wait_for("alpha", |state| reconnecting_attempt(state).is_some())
        .await;

    let logs = harness.engine.logs(harness.id("alpha")).unwrap();
    assert!(
        logs.iter()
            .any(|entry| entry.level == LogLevel::Warn
                && entry.message.contains("reconnect attempt 1")),
        "{logs:#?}"
    );
}

#[tokio::test]
async fn should_stamp_logs_with_the_injected_clock_when_a_tunnel_starts() {
    let clock = Arc::new(support::ManualClock::new());
    let mut harness = HarnessBuilder::new(&["alpha"])
        .clock(Arc::clone(&clock) as _)
        .start()
        .await;

    harness.start_and_wait_active("alpha").await;

    let logs = harness.engine.logs(harness.id("alpha")).unwrap();
    assert!(logs.iter().all(|entry| entry.at == clock.now()));
}

#[tokio::test]
async fn should_not_log_anything_secret_when_authentication_fails() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .with_unauthorized_client_key()
        .start()
        .await;

    harness.start_tunnel("alpha");
    harness
        .wait_for("alpha", |state| matches!(state, TunnelState::Failed { .. }))
        .await;

    assert_eq!(harness.prompter.request_count(), 0);
    let logs = harness.engine.logs(harness.id("alpha")).unwrap();
    assert!(logs.iter().all(|entry| !entry.message.contains("BEGIN")));
}

#[tokio::test]
async fn should_report_the_state_and_the_settings_when_a_snapshot_is_taken() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    let snapshot = harness.engine.snapshot();

    let by_name: HashMap<String, TunnelState> = snapshot
        .tunnels
        .into_iter()
        .map(|tunnel| (tunnel.definition.name, tunnel.state))
        .collect();
    assert!(matches!(by_name["alpha"], TunnelState::Active));
    assert_eq!(snapshot.settings.reconnect_initial_delay, RECONNECT_INITIAL);
}

#[tokio::test]
async fn should_reconnect_when_the_server_stops_answering_keepalives_set_in_the_settings() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .keepalive(Duration::from_millis(100), 2)
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;

    harness.server.stall_connections();

    let reconnecting = harness
        .wait_for("alpha", |state| reconnecting_attempt(state).is_some())
        .await;
    assert_eq!(reconnecting_attempt(&reconnecting).unwrap().0, 1);
}

#[tokio::test]
async fn should_ignore_the_settings_keepalive_when_ssh_config_disables_it() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .keepalive(Duration::from_millis(100), 2)
        .host_line("ServerAliveInterval 0")
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;

    harness.server.stall_connections();
    sleep(Duration::from_millis(800)).await;

    assert!(matches!(harness.state_of("alpha"), TunnelState::Active));
}

#[tokio::test]
async fn should_save_the_generated_ids_when_the_engine_starts_on_a_file_without_ids() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tunnels.toml");
    std::fs::write(
        &path,
        format!(
            "[[tunnels]]\nname = \"hand\"\nhost = \"{}\"\nkind = \"local\"\nlisten_port = {}\n",
            support::ALIAS,
            free_port().await
        ),
    )
    .unwrap();
    let (store, _) = Store::open(path.clone()).unwrap();
    let workspace = Workspace::new();
    let config_path = workspace.write_config(1, &[]);
    let connector = workspace.connector_dependencies(
        &config_path,
        support::quiet_prompter(),
        support::rejecting_verifier(),
        Arc::new(support::NoAgentLocator),
    );

    let engine = Engine::start(EngineDependencies {
        connector,
        clock: Arc::new(SystemClock),
        store,
    })
    .unwrap();

    let engine_id = engine.snapshot().tunnels[0].definition.id;
    let (reopened, outcome) = Store::open(path).unwrap();
    assert!(!outcome.was_normalized);
    assert_eq!(reopened.document().tunnels[0].id, engine_id);
}
