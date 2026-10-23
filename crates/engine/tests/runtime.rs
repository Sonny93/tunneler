#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use engine::{
    AppState, Clock, ConnectTimeouts, Engine, EngineDependencies, EngineError, EngineEvent,
    EngineOptions, FailureReason, HostKeyDecision, LaunchCommand, LogLevel, NewTunnel, OpenHost,
    OpenOptions, OpenPlan, OpenUrlError, Opener, PortOwner, PortScanner, Settings, Store,
    StoreError, SystemClock, SystemPortScanner, SystemScan, TagProblem, TargetHealth, TunnelId,
    TunnelKind, TunnelState, UnmanagedProcess,
};
use fast_socks5::client::{Config as SocksConfig, Socks5Stream};
use fast_socks5::{ReplyError, SocksError};
use russh::keys::{Algorithm, PrivateKey};
use support::{
    EchoServer, FakePortScanner, FakeX11Environment, MemoryStateStore, ScriptedPrompter,
    ScriptedVerifier, TestServer, TestUser, Workspace, foreign_scan, free_port, free_ports,
    generate_key, identity_of, ssh_scan, wait_for_event, wait_for_state, wait_until,
    write_private_key,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Receiver;
use tokio::time::{sleep, timeout};

const WAIT: Duration = Duration::from_secs(5);
const RECONNECT_INITIAL: Duration = Duration::from_millis(20);
const RECONNECT_MAXIMUM: Duration = Duration::from_millis(80);
const SESSION_GRACE: Duration = Duration::from_millis(100);
const STANDBY_IDLE: Duration = Duration::from_secs(60);
const SHORT_STANDBY_IDLE: Duration = Duration::from_millis(150);
const TICK_INTERVAL: Duration = Duration::from_millis(50);
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
    standby: Option<bool>,
}

#[derive(Clone, Copy)]
enum RemoteForwardPolicy {
    Allowed,
    Refused,
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
    standby: bool,
    standby_idle: Duration,
}

struct HarnessBuilder {
    tunnels: Vec<(&'static str, Vec<&'static str>)>,
    kind: &'static str,
    kinds: Vec<(&'static str, &'static str)>,
    timing: Timing,
    remote_forwards: RemoteForwardPolicy,
    keepalive: Option<Keepalive>,
    is_client_key_authorized: bool,
    is_target_up: bool,
    host_lines: Vec<&'static str>,
    verifier: ScriptedVerifier,
    clock: Arc<dyn Clock>,
    port_scanner: Option<Arc<dyn PortScanner>>,
    previously_running: Vec<&'static str>,
    standby_overrides: Vec<(&'static str, bool)>,
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
    scanner: Arc<FakePortScanner>,
    state_store: Arc<MemoryStateStore>,
}

impl HarnessBuilder {
    fn new(tunnels: &[&'static str]) -> Self {
        Self {
            tunnels: tunnels.iter().map(|name| (*name, Vec::new())).collect(),
            kind: "local",
            kinds: Vec::new(),
            timing: Timing {
                reconnect_initial: RECONNECT_INITIAL,
                reconnect_maximum: RECONNECT_MAXIMUM,
                session_grace: SESSION_GRACE,
                standby: false,
                standby_idle: STANDBY_IDLE,
            },
            remote_forwards: RemoteForwardPolicy::Allowed,
            keepalive: None,
            is_client_key_authorized: true,
            is_target_up: true,
            host_lines: Vec::new(),
            verifier: ScriptedVerifier::default(),
            clock: Arc::new(SystemClock),
            port_scanner: None,
            previously_running: Vec::new(),
            standby_overrides: Vec::new(),
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

    fn kind_of(mut self, name: &'static str, kind: &'static str) -> Self {
        self.kinds.push((name, kind));
        self
    }

    fn with_remote_forwards_refused(mut self) -> Self {
        self.remote_forwards = RemoteForwardPolicy::Refused;
        self
    }

    fn reconnect_delays(mut self, initial: Duration, maximum: Duration) -> Self {
        self.timing.reconnect_initial = initial;
        self.timing.reconnect_maximum = maximum;
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

    fn port_scanner(mut self, scanner: Arc<dyn PortScanner>) -> Self {
        self.port_scanner = Some(scanner);
        self
    }

    fn previously_running(mut self, names: &[&'static str]) -> Self {
        self.previously_running = names.to_vec();
        self
    }

    fn standby_by_default(mut self) -> Self {
        self.timing.standby = true;
        self
    }

    fn standby_idle(mut self, idle: Duration) -> Self {
        self.timing.standby_idle = idle;
        self
    }

    fn standby_override(mut self, name: &'static str, is_standby: bool) -> Self {
        self.standby_overrides.push((name, is_standby));
        self
    }

    async fn start(self) -> Harness {
        let client_key = generate_key(Algorithm::Ed25519);
        let server = server_authorizing(
            &client_key,
            self.is_client_key_authorized,
            self.remote_forwards,
        )
        .await;
        let echo = EchoServer::start().await;
        let down_target_count = usize::from(!self.is_target_up);
        let mut free = free_ports(self.tunnels.len() + down_target_count).await;
        let target_port = if self.is_target_up {
            echo.port
        } else {
            free.pop().unwrap()
        };
        let specs = self.specs(target_port, &free);
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
        apply_sub_second_settings(&store, &self.timing, self.keepalive);
        let state_store = Arc::new(MemoryStateStore::holding(AppState {
            running: self
                .previously_running
                .iter()
                .map(|name| {
                    specs
                        .iter()
                        .find(|spec| spec.name == *name)
                        .map_or_else(TunnelId::generate, |spec| spec.id)
                })
                .collect(),
            ..AppState::default()
        }));
        let scanner = Arc::new(FakePortScanner::default());
        let port_scanner = self
            .port_scanner
            .unwrap_or_else(|| Arc::clone(&scanner) as Arc<dyn PortScanner>);
        let engine = Engine::start_with_options(
            EngineDependencies {
                connector: dependencies,
                clock: self.clock,
                store,
                port_scanner,
                x11_environment: Arc::new(FakeX11Environment::default()),
                state_store: state_store.clone(),
                keyring: None,
                detected_openers: Vec::new(),
            },
            EngineOptions {
                metrics_event_interval: TICK_INTERVAL,
                port_scan_interval: TICK_INTERVAL,
                connect_timeouts: ConnectTimeouts::default(),
                tunnel_ready_timeout: WAIT,
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
            scanner,
            state_store,
        }
    }

    fn specs(&self, target_port: u16, listen_ports: &[u16]) -> Vec<TunnelSpec> {
        let mut specs = Vec::new();
        for ((name, tags), listen_port) in self.tunnels.iter().zip(listen_ports) {
            specs.push(TunnelSpec {
                id: TunnelId::generate(),
                name: (*name).to_owned(),
                kind: self
                    .kinds
                    .iter()
                    .find(|(kinded, _)| kinded == name)
                    .map_or(self.kind, |(_, kind)| *kind),
                listen_port: *listen_port,
                target_port,
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                standby: self
                    .standby_overrides
                    .iter()
                    .find(|(overridden, _)| overridden == name)
                    .map(|(_, is_standby)| *is_standby),
            });
        }
        specs
    }
}

async fn server_authorizing(
    client_key: &PrivateKey,
    is_authorized: bool,
    remote_forwards: RemoteForwardPolicy,
) -> TestServer {
    let authorized_key = if is_authorized {
        client_key.clone()
    } else {
        generate_key(Algorithm::Ed25519)
    };
    let user = TestUser {
        authorized_keys: vec![authorized_key.public_key().clone()],
        ..TestUser::default()
    };
    let builder = TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(support::REMOTE_USER, user);
    match remote_forwards {
        RemoteForwardPolicy::Allowed => builder.start().await,
        RemoteForwardPolicy::Refused => builder.refusing_remote_forwards().start().await,
    }
}

fn store_text(specs: &[TunnelSpec], timing: &Timing) -> String {
    let mut text = format!(
        "version = 1\n\n[settings]\nreconnect_initial_delay_ms = {}\nreconnect_max_delay_ms = {}\nsession_idle_grace_s = 0\nstandby = {}\n\n",
        timing.reconnect_initial.as_millis(),
        timing.reconnect_maximum.as_millis(),
        timing.standby
    );
    for spec in specs {
        text.push_str(&tunnel_block(spec));
    }
    text
}

fn tunnel_block(spec: &TunnelSpec) -> String {
    let tags: Vec<String> = spec.tags.iter().map(|tag| format!("\"{tag}\"")).collect();
    let target_line = if spec.kind == "dynamic" {
        String::new()
    } else {
        format!("target_port = {}\n", spec.target_port)
    };
    let standby_line = spec.standby.map_or_else(String::new, |is_standby| {
        format!("standby = {is_standby}\n")
    });
    format!(
        "[[tunnels]]\nid = \"{}\"\nhost = \"{}\"\nkind = \"{}\"\nlisten_port = {}\n{target_line}{standby_line}tags = [{}]\n\n",
        spec.id,
        support::ALIAS,
        spec.kind,
        spec.listen_port,
        tags.join(", ")
    )
}

fn apply_sub_second_settings(store: &Store, timing: &Timing, keepalive: Option<Keepalive>) {
    let mut document = store.document();
    if let Some(keepalive) = keepalive {
        document.settings.keepalive_interval = keepalive.interval;
        document.settings.keepalive_max = keepalive.maximum_unanswered;
    }
    document.settings.session_idle_grace = timing.session_grace;
    document.settings.standby_idle = timing.standby_idle;
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

    fn is_awake(&self, name: &str) -> bool {
        self.engine
            .snapshot()
            .tunnels
            .into_iter()
            .find(|tunnel| tunnel.definition.id == self.id(name))
            .unwrap()
            .is_awake
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

    async fn wait_standby(&mut self, name: &str) {
        self.wait_for(name, |state| {
            matches!(state, TunnelState::Standby { last_failure: None })
        })
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

    fn set_standby_override(&mut self, name: &str, is_standby: Option<bool>) {
        let position = self
            .specs
            .iter()
            .position(|spec| spec.name == name)
            .unwrap();
        self.specs[position].standby = is_standby;
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

async fn socks_connect(
    proxy_port: u16,
    host: &str,
    target_port: u16,
) -> Result<Socks5Stream<TcpStream>, SocksError> {
    Socks5Stream::connect(
        (LOOPBACK, proxy_port),
        host.to_owned(),
        target_port,
        SocksConfig::default(),
    )
    .await
}

async fn echo_through_socks(
    proxy_port: u16,
    host: &str,
    target_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut stream = socks_connect(proxy_port, host, target_port).await.unwrap();
    stream.write_all(payload).await.unwrap();
    let mut received = vec![0_u8; payload.len()];
    timeout(WAIT, stream.read_exact(&mut received))
        .await
        .unwrap()
        .unwrap();
    received
}

async fn socks_reply_code_to(proxy_port: u16, request: &[u8]) -> u8 {
    let mut stream = TcpStream::connect((LOOPBACK, proxy_port)).await.unwrap();
    stream.write_all(&[5, 1, 0]).await.unwrap();
    let mut method = [0_u8; 2];
    stream.read_exact(&mut method).await.unwrap();
    stream.write_all(request).await.unwrap();
    let mut reply = [0_u8; 10];
    timeout(WAIT, stream.read_exact(&mut reply))
        .await
        .unwrap()
        .unwrap();
    reply[1]
}

async fn wait_for_exact_bytes(harness: &mut Harness, name: &str, length: usize) {
    let expected = u64::try_from(length).unwrap();
    let id = harness.id(name);
    wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelMetrics {
            id: changed,
            metrics,
        } if *changed == id && metrics.bytes_out == expected && metrics.bytes_in == expected => {
            Some(())
        }
        _ => None,
    })
    .await;
}

async fn is_refused(port: u16) -> bool {
    TcpStream::connect((LOOPBACK, port)).await.is_err()
}

fn is_unreachable_event(event: &EngineEvent) -> Option<TunnelState> {
    match event {
        EngineEvent::TunnelChanged {
            state,
            health: TargetHealth::Unreachable { .. },
            ..
        } => Some(state.clone()),
        _ => None,
    }
}

fn is_reachable_event(event: &EngineEvent) -> Option<()> {
    matches!(
        event,
        EngineEvent::TunnelChanged {
            health: TargetHealth::Reachable,
            ..
        }
    )
    .then_some(())
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
async fn should_keep_the_port_bound_and_close_connections_immediately_and_log_them_when_the_server_is_down()
 {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.server.stop().await;
    harness
        .wait_for("alpha", |state| reconnecting_attempt(state).is_some())
        .await;

    assert!(!can_bind(harness.port("alpha")).await);
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
async fn should_report_a_conflict_without_any_ssh_connection_when_the_port_is_taken() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let port = harness.port("alpha");
    let _squatter = TcpListener::bind((LOOPBACK, port)).await.unwrap();

    harness.start_tunnel("alpha");
    let state = harness.wait_for("alpha", is_conflict).await;

    assert_eq!(state, TunnelState::Conflict { owner: None });
    assert_eq!(harness.server.connection_count(), 0);
}

#[tokio::test]
async fn should_start_again_when_the_port_is_freed_after_a_failure() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let squatter = TcpListener::bind((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();
    harness.start_tunnel("alpha");
    harness.wait_for("alpha", is_conflict).await;

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

const SOCKS_BIND_REQUEST: [u8; 10] = [5, 2, 0, 1, 0, 0, 0, 0, 0, 0];
const SOCKS_REPLY_COMMAND_NOT_SUPPORTED: u8 = 7;

#[tokio::test]
async fn should_reach_the_target_when_a_socks_client_connects_by_ip_address() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;

    let echoed =
        echo_through_socks(harness.port("socks"), LOOPBACK, harness.echo.port, PAYLOAD).await;

    assert_eq!(echoed, PAYLOAD);
}

#[tokio::test]
async fn should_reach_the_target_with_the_domain_name_unresolved_when_a_socks_client_connects_by_domain_name()
 {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;

    let echoed = echo_through_socks(
        harness.port("socks"),
        "localhost",
        harness.echo.port,
        PAYLOAD,
    )
    .await;

    assert_eq!(echoed, PAYLOAD);
    assert_eq!(
        harness.server.direct_tcpip_targets(),
        vec![("localhost".to_owned(), harness.echo.port)]
    );
}

#[tokio::test]
async fn should_report_the_socks_client_address_to_the_server_when_forwarding() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;

    echo_through_socks(harness.port("socks"), LOOPBACK, harness.echo.port, PAYLOAD).await;

    let originators = harness.server.direct_tcpip_originators();
    assert_eq!(originators.len(), 1);
    assert_eq!(originators[0].0, LOOPBACK);
    assert_ne!(originators[0].1, 0);
}

#[tokio::test]
async fn should_reply_connection_refused_when_the_socks_target_is_down() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("socks").await;
    let target_port = harness.spec("socks").target_port;

    let failure = socks_connect(harness.port("socks"), LOOPBACK, target_port)
        .await
        .err();

    assert!(
        matches!(
            failure,
            Some(SocksError::ReplyError(ReplyError::ConnectionRefused))
        ),
        "{failure:?}"
    );
}

#[tokio::test]
async fn should_stay_active_when_a_socks_target_is_refused() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("socks").await;
    let target_port = harness.spec("socks").target_port;
    let _ = socks_connect(harness.port("socks"), LOOPBACK, target_port).await;

    echo_through_socks(harness.port("socks"), LOOPBACK, harness.echo.port, PAYLOAD).await;

    assert_eq!(harness.state_of("socks"), TunnelState::Active);
}

#[tokio::test]
async fn should_log_a_warning_when_a_socks_target_is_refused() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("socks").await;
    let target_port = harness.spec("socks").target_port;
    let _ = socks_connect(harness.port("socks"), LOOPBACK, target_port).await;

    let id = harness.id("socks");
    let engine = &harness.engine;
    wait_until(WAIT, "the refusal to be logged", || {
        engine
            .logs(id)
            .unwrap()
            .iter()
            .any(|entry| entry.level == LogLevel::Warn && entry.message.contains("cannot reach"))
    })
    .await;
}

#[tokio::test]
async fn should_keep_the_target_health_unknown_when_a_socks_target_is_refused() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("socks").await;
    let target_port = harness.spec("socks").target_port;
    let _ = socks_connect(harness.port("socks"), LOOPBACK, target_port).await;

    sleep(QUIET_PERIOD).await;

    let snapshot = harness.engine.snapshot();
    assert_eq!(snapshot.tunnels[0].health, TargetHealth::Unknown);
}

#[tokio::test]
async fn should_reply_command_not_supported_and_open_no_channel_when_a_socks_client_asks_for_bind()
{
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;

    let code = socks_reply_code_to(harness.port("socks"), &SOCKS_BIND_REQUEST).await;

    assert_eq!(code, SOCKS_REPLY_COMMAND_NOT_SUPPORTED);
    assert_eq!(harness.server.direct_tcpip_targets().len(), 0);
}

#[tokio::test]
async fn should_close_the_connection_when_a_client_does_not_speak_socks() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;
    let mut client = TcpStream::connect((LOOPBACK, harness.port("socks")))
        .await
        .unwrap();

    client.write_all(b"GET / HTTP/1.1\r\n\r\n").await.unwrap();

    let mut buffer = [0_u8; 16];
    let outcome = timeout(WAIT, client.read(&mut buffer)).await.unwrap();
    assert!(matches!(outcome, Ok(0) | Err(_)), "{outcome:?}");
}

#[tokio::test]
async fn should_keep_the_local_tunnel_forwarding_when_a_dynamic_tunnel_on_the_same_host_is_toggled()
{
    let mut harness = HarnessBuilder::new(&["alpha", "socks"])
        .kind_of("socks", "dynamic")
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;
    harness.start_and_wait_active("socks").await;

    harness.stop_tunnel("socks");
    harness.wait_stopped("socks").await;
    harness.start_and_wait_active("socks").await;

    assert_eq!(echo_through(harness.port("alpha"), PAYLOAD).await, PAYLOAD);
    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_release_the_port_when_a_dynamic_tunnel_is_stopped() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;

    harness.stop_tunnel("socks");
    harness.wait_stopped("socks").await;

    assert!(can_bind(harness.port("socks")).await);
}

#[tokio::test]
async fn should_keep_the_socks_port_bound_when_the_connection_is_cut() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;

    harness.server.stop().await;
    harness
        .wait_for("socks", |state| reconnecting_attempt(state).is_some())
        .await;

    assert!(!can_bind(harness.port("socks")).await);
}

#[tokio::test]
async fn should_reach_the_target_again_when_a_dynamic_tunnel_reconnects() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;

    harness.server.cut_connections();
    harness
        .wait_for("socks", |state| reconnecting_attempt(state).is_some())
        .await;
    harness.wait_active("socks").await;

    let echoed =
        echo_through_socks(harness.port("socks"), LOOPBACK, harness.echo.port, PAYLOAD).await;
    assert_eq!(echoed, PAYLOAD);
}

#[tokio::test]
async fn should_count_the_bytes_when_data_goes_through_a_dynamic_tunnel() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;

    echo_through_socks(harness.port("socks"), LOOPBACK, harness.echo.port, PAYLOAD).await;

    wait_for_exact_bytes(&mut harness, "socks", PAYLOAD.len()).await;
}

#[tokio::test]
async fn should_reach_the_local_target_and_ask_for_the_default_bind_address_when_a_client_connects_to_the_remote_port()
 {
    let mut harness = HarnessBuilder::new(&["rev"]).kind("remote").start().await;
    harness.start_and_wait_active("rev").await;

    assert_eq!(echo_through(harness.port("rev"), PAYLOAD).await, PAYLOAD);
    assert_eq!(
        harness.server.remote_forward_requests(),
        vec![("localhost".to_owned(), harness.port("rev"))]
    );
}

#[tokio::test]
async fn should_fail_with_remote_forward_refused_when_the_server_refuses_to_listen() {
    let mut harness = HarnessBuilder::new(&["rev"])
        .kind("remote")
        .with_remote_forwards_refused()
        .start()
        .await;

    harness.start_tunnel("rev");
    let state = harness
        .wait_for("rev", |state| matches!(state, TunnelState::Failed { .. }))
        .await;

    assert_eq!(
        state,
        TunnelState::Failed {
            reason: FailureReason::RemoteForwardRefused {
                address: format!("localhost:{}", harness.port("rev"))
            }
        }
    );
}

#[tokio::test]
async fn should_not_retry_when_the_server_refuses_to_listen() {
    let mut harness = HarnessBuilder::new(&["rev"])
        .kind("remote")
        .with_remote_forwards_refused()
        .start()
        .await;
    harness.start_tunnel("rev");
    harness
        .wait_for("rev", |state| matches!(state, TunnelState::Failed { .. }))
        .await;

    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.server.remote_forward_requests().len(), 1);
    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_stop_listening_on_the_server_when_the_remote_tunnel_is_stopped() {
    let mut harness = HarnessBuilder::new(&["rev"]).kind("remote").start().await;
    harness.start_and_wait_active("rev").await;

    harness.stop_tunnel("rev");
    harness.wait_stopped("rev").await;

    assert!(is_refused(harness.port("rev")).await);
}

#[tokio::test]
async fn should_listen_on_the_server_again_when_the_remote_tunnel_is_restarted() {
    let mut harness = HarnessBuilder::new(&["rev"]).kind("remote").start().await;
    harness.start_and_wait_active("rev").await;
    harness.stop_tunnel("rev");
    harness.wait_stopped("rev").await;

    harness.start_and_wait_active("rev").await;

    assert_eq!(echo_through(harness.port("rev"), PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_drop_live_connections_when_the_remote_tunnel_is_stopped() {
    let mut harness = HarnessBuilder::new(&["rev"]).kind("remote").start().await;
    harness.start_and_wait_active("rev").await;
    let mut client = TcpStream::connect((LOOPBACK, harness.port("rev")))
        .await
        .unwrap();
    client.write_all(PAYLOAD).await.unwrap();
    let mut echoed = vec![0_u8; PAYLOAD.len()];
    client.read_exact(&mut echoed).await.unwrap();

    harness.stop_tunnel("rev");
    harness.wait_stopped("rev").await;

    let mut buffer = [0_u8; 1];
    let outcome = timeout(WAIT, client.read(&mut buffer)).await.unwrap();
    assert!(matches!(outcome, Ok(0) | Err(_)), "{outcome:?}");
}

#[tokio::test]
async fn should_forward_again_when_the_connection_of_a_remote_tunnel_is_cut() {
    let mut harness = HarnessBuilder::new(&["rev"]).kind("remote").start().await;
    harness.start_and_wait_active("rev").await;

    harness.server.cut_connections();
    harness
        .wait_for("rev", |state| reconnecting_attempt(state).is_some())
        .await;
    harness.wait_active("rev").await;

    assert_eq!(echo_through(harness.port("rev"), PAYLOAD).await, PAYLOAD);
    assert_eq!(harness.server.connection_count(), 2);
}

#[tokio::test]
async fn should_share_one_session_when_a_remote_and_a_local_tunnel_use_the_same_host() {
    let mut harness = HarnessBuilder::new(&["alpha", "rev"])
        .kind_of("rev", "remote")
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;
    harness.start_and_wait_active("rev").await;

    echo_through(harness.port("alpha"), PAYLOAD).await;
    echo_through(harness.port("rev"), PAYLOAD).await;

    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_keep_the_other_remote_tunnel_forwarding_when_one_is_stopped() {
    let mut harness = HarnessBuilder::new(&["first", "second"])
        .kind("remote")
        .start()
        .await;
    harness.start_and_wait_active("first").await;
    harness.start_and_wait_active("second").await;

    harness.stop_tunnel("first");
    harness.wait_stopped("first").await;

    assert_eq!(echo_through(harness.port("second"), PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_mark_the_target_unreachable_while_the_tunnel_stays_active_when_the_remote_target_is_down()
 {
    let mut harness = HarnessBuilder::new(&["rev"])
        .kind("remote")
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("rev").await;

    assert!(is_closed_by_the_tunnel(harness.port("rev")).await);

    let state = wait_for_event(&mut harness.events, WAIT, is_unreachable_event).await;
    assert_eq!(state, TunnelState::Active);
}

#[tokio::test]
async fn should_mark_the_target_reachable_when_the_remote_target_answers() {
    let mut harness = HarnessBuilder::new(&["rev"]).kind("remote").start().await;
    harness.start_and_wait_active("rev").await;

    echo_through(harness.port("rev"), PAYLOAD).await;

    wait_for_event(&mut harness.events, WAIT, is_reachable_event).await;
}

#[tokio::test]
async fn should_count_the_bytes_when_data_goes_through_a_remote_tunnel() {
    let mut harness = HarnessBuilder::new(&["rev"]).kind("remote").start().await;
    harness.start_and_wait_active("rev").await;

    echo_through(harness.port("rev"), PAYLOAD).await;

    wait_for_exact_bytes(&mut harness, "rev", PAYLOAD.len()).await;
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
async fn should_not_open_any_connection_to_the_target_while_no_client_connects() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.target_requests(), 0);
}

#[tokio::test]
async fn should_report_the_target_reachable_with_one_request_when_checking_an_active_local_tunnel()
{
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    let health = harness.engine.check_target(harness.id("alpha")).await;

    assert_eq!(
        (health.unwrap(), harness.target_requests()),
        (TargetHealth::Reachable, 1)
    );
}

#[tokio::test]
async fn should_publish_the_new_health_when_checking_the_target() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness
        .engine
        .check_target(harness.id("alpha"))
        .await
        .unwrap();

    wait_for_event(&mut harness.events, WAIT, is_reachable_event).await;
}

#[tokio::test]
async fn should_report_the_target_unreachable_when_checking_a_local_tunnel_with_a_down_target() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("alpha").await;

    let health = harness.engine.check_target(harness.id("alpha")).await;

    assert!(matches!(health, Ok(TargetHealth::Unreachable { .. })));
}

#[tokio::test]
async fn should_fail_when_checking_the_target_of_a_stopped_tunnel() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let outcome = harness.engine.check_target(harness.id("alpha")).await;

    assert!(matches!(outcome, Err(EngineError::TunnelNotActive(_))));
}

#[tokio::test]
async fn should_fail_when_checking_the_target_of_an_unknown_tunnel() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let outcome = harness.engine.check_target(TunnelId::generate()).await;

    assert!(matches!(outcome, Err(EngineError::UnknownTunnel(_))));
}

#[tokio::test]
async fn should_fail_when_checking_the_target_of_a_dynamic_tunnel() {
    let mut harness = HarnessBuilder::new(&["socks"])
        .kind("dynamic")
        .start()
        .await;
    harness.start_and_wait_active("socks").await;

    let outcome = harness.engine.check_target(harness.id("socks")).await;

    assert!(matches!(outcome, Err(EngineError::TunnelHasNoTarget(_))));
}

#[tokio::test]
async fn should_report_the_target_reachable_when_checking_an_active_remote_tunnel() {
    let mut harness = HarnessBuilder::new(&["rev"]).kind("remote").start().await;
    harness.start_and_wait_active("rev").await;

    let health = harness.engine.check_target(harness.id("rev")).await;

    assert_eq!(health.unwrap(), TargetHealth::Reachable);
}

#[tokio::test]
async fn should_report_the_target_unreachable_when_checking_a_remote_tunnel_with_a_down_target() {
    let mut harness = HarnessBuilder::new(&["rev"])
        .kind("remote")
        .with_target_down()
        .start()
        .await;
    harness.start_and_wait_active("rev").await;

    let health = harness.engine.check_target(harness.id("rev")).await;

    assert!(matches!(health, Ok(TargetHealth::Unreachable { .. })));
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

fn tags_of(harness: &Harness, name: &str) -> Vec<String> {
    let id = harness.id(name);
    harness
        .engine
        .snapshot()
        .tunnels
        .into_iter()
        .find(|snapshot| snapshot.definition.id == id)
        .unwrap()
        .definition
        .tags
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[tokio::test]
async fn should_rename_the_tag_in_every_tunnel_that_carries_it() {
    let harness = HarnessBuilder::tagged(&[
        ("alpha", &["dev"]),
        ("beta", &["dev", "web"]),
        ("gamma", &["web"]),
    ])
    .start()
    .await;

    let renamed_count = harness.engine.rename_tag("dev", "staging").await.unwrap();

    assert_eq!(
        (
            renamed_count,
            tags_of(&harness, "alpha"),
            tags_of(&harness, "beta"),
            tags_of(&harness, "gamma"),
        ),
        (
            2,
            strings(&["staging"]),
            strings(&["staging", "web"]),
            strings(&["web"]),
        )
    );
}

#[tokio::test]
async fn should_persist_the_renamed_tag_in_the_store() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;

    harness.engine.rename_tag("dev", "staging").await.unwrap();

    let (reopened, _) = Store::open(harness.store_path()).unwrap();
    assert_eq!(reopened.document().tunnels[0].tags, strings(&["staging"]));
}

#[tokio::test]
async fn should_merge_into_the_existing_tag_when_a_tunnel_already_carries_the_new_name() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev", "web"])])
        .start()
        .await;

    harness.engine.rename_tag("dev", "web").await.unwrap();

    assert_eq!(tags_of(&harness, "alpha"), strings(&["web"]));
}

#[tokio::test]
async fn should_rename_the_tag_in_the_start_on_launch_tags() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;
    configure_launch(&harness, false, &["dev"]).await;

    harness.engine.rename_tag("dev", "staging").await.unwrap();

    assert_eq!(
        harness.engine.snapshot().settings.start_on_launch_tags,
        strings(&["staging"])
    );
}

#[tokio::test]
async fn should_announce_a_reload_when_a_tag_is_renamed() {
    let mut harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;

    harness.engine.rename_tag("dev", "staging").await.unwrap();

    wait_for_event(&mut harness.events, WAIT, |event| {
        (*event == EngineEvent::ConfigReloaded).then_some(())
    })
    .await;
}

#[tokio::test]
async fn should_keep_the_tunnel_running_when_its_tag_is_renamed() {
    let mut harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;
    harness.start_tunnel("alpha");
    harness.wait_all_active(&["alpha"]).await;

    harness.engine.rename_tag("dev", "staging").await.unwrap();

    assert_eq!(harness.engine.active_tunnel_count(), 1);
}

#[tokio::test]
async fn should_do_nothing_when_a_tag_is_renamed_to_itself() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;
    let before = std::fs::read_to_string(harness.store_path()).unwrap();

    let renamed_count = harness.engine.rename_tag("dev", "dev").await.unwrap();

    assert_eq!(
        (
            renamed_count,
            std::fs::read_to_string(harness.store_path()).unwrap()
        ),
        (0, before)
    );
}

#[tokio::test]
async fn should_refuse_to_rename_a_tag_nobody_carries() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;

    let outcome = harness.engine.rename_tag("missing", "staging").await;

    assert!(matches!(outcome, Err(EngineError::UnknownTag(tag)) if tag == "missing"));
}

#[tokio::test]
async fn should_refuse_a_new_tag_name_that_breaks_the_tag_syntax() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;

    let outcome = harness.engine.rename_tag("dev", "two words").await;

    assert!(matches!(
        outcome,
        Err(EngineError::InvalidTag {
            problem: TagProblem::ContainsWhitespace,
            ..
        })
    ));
}

#[tokio::test]
async fn should_remove_the_deleted_tag_from_every_tunnel_that_carries_it() {
    let harness = HarnessBuilder::tagged(&[
        ("alpha", &["dev"]),
        ("beta", &["dev", "web"]),
        ("gamma", &["web"]),
    ])
    .start()
    .await;

    let removed_count = harness.engine.delete_tag("dev").await.unwrap();

    assert_eq!(
        (
            removed_count,
            tags_of(&harness, "alpha"),
            tags_of(&harness, "beta"),
            tags_of(&harness, "gamma"),
        ),
        (2, strings(&[]), strings(&["web"]), strings(&["web"]),)
    );
}

#[tokio::test]
async fn should_keep_the_tunnels_when_their_tag_is_deleted() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"]), ("beta", &["dev"])])
        .start()
        .await;

    harness.engine.delete_tag("dev").await.unwrap();

    assert_eq!(harness.engine.snapshot().tunnels.len(), 2);
}

#[tokio::test]
async fn should_persist_the_tunnels_and_the_launch_tags_in_one_write_when_a_tag_is_deleted() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev", "web"])])
        .start()
        .await;
    configure_launch(&harness, false, &["dev", "web"]).await;

    harness.engine.delete_tag("dev").await.unwrap();

    let (reopened, _) = Store::open(harness.store_path()).unwrap();
    let document = reopened.document();
    assert_eq!(
        (
            document.tunnels[0].tags.clone(),
            document.settings.start_on_launch_tags
        ),
        (strings(&["web"]), strings(&["web"]))
    );
}

#[tokio::test]
async fn should_remove_the_deleted_tag_from_the_start_on_launch_tags() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;
    configure_launch(&harness, false, &["dev"]).await;

    harness.engine.delete_tag("dev").await.unwrap();

    assert_eq!(
        harness.engine.snapshot().settings.start_on_launch_tags,
        strings(&[])
    );
}

#[tokio::test]
async fn should_announce_a_reload_when_a_tag_is_deleted() {
    let mut harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;

    harness.engine.delete_tag("dev").await.unwrap();

    wait_for_event(&mut harness.events, WAIT, |event| {
        (*event == EngineEvent::ConfigReloaded).then_some(())
    })
    .await;
}

#[tokio::test]
async fn should_keep_the_tunnel_running_when_its_tag_is_deleted() {
    let mut harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;
    harness.start_tunnel("alpha");
    harness.wait_all_active(&["alpha"]).await;

    harness.engine.delete_tag("dev").await.unwrap();

    assert_eq!(harness.engine.active_tunnel_count(), 1);
}

#[tokio::test]
async fn should_refuse_to_delete_a_tag_nobody_carries() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;

    let outcome = harness.engine.delete_tag("missing").await;

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
async fn should_not_restart_the_tunnel_when_the_file_only_changes_its_tags() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    let mut client = TcpStream::connect((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();

    harness.specs[0].tags = vec!["renamed".to_owned()];
    harness.rewrite_store();
    harness.wait_for_config_reload().await;

    client.write_all(PAYLOAD).await.unwrap();
    let mut echoed = vec![0_u8; PAYLOAD.len()];
    timeout(WAIT, client.read_exact(&mut echoed))
        .await
        .unwrap()
        .unwrap();
    let snapshot = harness.engine.snapshot();
    assert_eq!(snapshot.tunnels[0].definition.tags, ["renamed"]);
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

    let remaining: Vec<TunnelId> = harness
        .engine
        .snapshot()
        .tunnels
        .into_iter()
        .map(|tunnel| tunnel.definition.id)
        .collect();
    assert_eq!(remaining, vec![harness.id("beta")]);
    assert!(can_bind(port).await);
    assert_eq!(echo_through(harness.port("beta"), PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_add_a_stopped_tunnel_and_save_its_id_when_the_file_gains_a_tunnel_without_id() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let added_port = free_port().await;
    let text = format!(
        "{}[[tunnels]]\nhost = \"{}\"\nkind = \"local\"\nlisten_port = {added_port}\n",
        store_text(&harness.specs, &harness.timing),
        support::ALIAS,
    );

    std::fs::write(harness.store_path(), text).unwrap();
    harness.wait_for_config_reload().await;

    let snapshot = harness.engine.snapshot();
    assert_eq!(snapshot.tunnels[1].definition.listen_port, added_port);
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

fn new_tunnel(listen_port: u16) -> NewTunnel {
    NewTunnel {
        host: support::ALIAS.to_owned(),
        kind: TunnelKind::Local,
        listen_port,
        listen_address: None,
        target_host: None,
        target_port: None,
        tags: vec!["fresh".to_owned()],
        open: OpenOptions::default(),
        standby: None,
    }
}

fn saved_tunnel_ports(harness: &Harness) -> Vec<u16> {
    let (store, _) = Store::open(harness.store_path()).unwrap();
    store
        .document()
        .tunnels
        .into_iter()
        .map(|tunnel| tunnel.listen_port)
        .collect()
}

#[tokio::test]
async fn should_persist_the_tunnel_when_it_is_created() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let created_port = free_port().await;
    let id = harness
        .engine
        .create_tunnel(new_tunnel(created_port))
        .await
        .unwrap();

    assert_eq!(
        saved_tunnel_ports(&harness),
        [harness.port("alpha"), created_port]
    );
    let created = harness.engine.snapshot().tunnels.pop().unwrap();
    assert_eq!(
        (created.definition.id, created.state),
        (id, TunnelState::Stopped)
    );
}

#[tokio::test]
async fn should_refuse_to_create_a_tunnel_when_the_listen_port_is_taken() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let outcome = harness
        .engine
        .create_tunnel(new_tunnel(harness.port("alpha")))
        .await;

    assert!(matches!(outcome, Err(EngineError::Store(_))), "{outcome:?}");
    assert_eq!(harness.engine.snapshot().tunnels.len(), 1);
}

#[tokio::test]
async fn should_persist_the_change_when_a_tunnel_is_updated() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let mut definition = harness.engine.snapshot().tunnels.remove(0).definition;

    definition.tags = vec!["renamed".to_owned()];
    harness.engine.update_tunnel(definition).await.unwrap();

    assert_eq!(
        harness.engine.snapshot().tunnels[0].definition.tags,
        ["renamed"]
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

    assert_eq!(saved_tunnel_ports(&harness), [harness.port("beta")]);
    assert_eq!(harness.engine.snapshot().tunnels.len(), 1);
    assert!(can_bind(port).await);
}

#[tokio::test]
async fn should_not_reload_again_when_the_engine_writes_its_own_file() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness
        .engine
        .create_tunnel(new_tunnel(free_port().await))
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
async fn should_keep_the_tunnel_wanted_and_log_nothing_secret_when_authentication_fails() {
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
    assert!(wanted_ids(&harness).contains(&harness.id("alpha")));
}

#[tokio::test]
async fn should_report_the_state_and_the_settings_when_a_snapshot_is_taken() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    let snapshot = harness.engine.snapshot();

    let alpha = harness.id("alpha");
    let alpha_state = snapshot
        .tunnels
        .into_iter()
        .find(|tunnel| tunnel.definition.id == alpha)
        .map(|tunnel| tunnel.state);
    assert!(matches!(alpha_state, Some(TunnelState::Active)));
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
        port_scanner: Arc::new(FakePortScanner::default()),
        x11_environment: Arc::new(FakeX11Environment::default()),
        state_store: Arc::new(MemoryStateStore::default()),
        keyring: None,
        detected_openers: Vec::new(),
    })
    .unwrap();

    let engine_id = engine.snapshot().tunnels[0].definition.id;
    let (reopened, outcome) = Store::open(path).unwrap();
    assert!(!outcome.was_normalized);
    assert_eq!(reopened.document().tunnels[0].id, engine_id);
}

#[tokio::test]
async fn should_persist_and_apply_the_settings_when_they_are_updated() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let settings = Settings {
        keepalive_max: 9,
        ..harness.engine.snapshot().settings
    };

    harness
        .engine
        .update_configuration(settings, Vec::new())
        .await
        .unwrap();

    let stored = std::fs::read_to_string(harness.store_path()).unwrap();
    assert_eq!(
        (
            harness.engine.snapshot().settings.keepalive_max,
            stored.contains("keepalive_max = 9")
        ),
        (9, true)
    );
    wait_for_event(&mut harness.events, WAIT, |event| {
        (*event == EngineEvent::ConfigReloaded).then_some(())
    })
    .await;
}

#[tokio::test]
async fn should_write_nothing_when_the_updated_settings_are_invalid() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let before = std::fs::read_to_string(harness.store_path()).unwrap();
    let settings = Settings {
        default_bind_address: String::new(),
        ..harness.engine.snapshot().settings
    };

    let error = harness
        .engine
        .update_configuration(settings, Vec::new())
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::Store(StoreError::Validation(_))
    ));
    assert_eq!(
        std::fs::read_to_string(harness.store_path()).unwrap(),
        before
    );
}

const SSH_PID: u32 = 424_242;
const OTHER_SSH_PID: u32 = 424_243;
const NGINX_PID: u32 = 424_244;
const UNMANAGED_PORT: u16 = 59_999;
const SSH_NAME: &str = "ssh";
const REAL_SSH_NAME: &str = if cfg!(windows) { "ssh.exe" } else { "ssh" };

impl Harness {
    fn forward_of(&self, name: &str) -> (u16, String) {
        let port = self.port(name);
        (port, format!("{port}:{LOOPBACK}:{}", self.echo.port))
    }

    fn scan_with_ssh_providing(&self, pid: u32, names: &[&str]) -> SystemScan {
        let forwards: Vec<(u16, String)> = names.iter().map(|name| self.forward_of(name)).collect();
        ssh_scan(pid, &forwards)
    }

    fn set_scan(&self, scan: SystemScan) {
        self.scanner.set_scan(scan);
    }

    async fn wait_for_externals(
        &mut self,
        predicate: impl Fn(&[UnmanagedProcess]) -> bool,
    ) -> Vec<UnmanagedProcess> {
        wait_for_event(&mut self.events, WAIT, |event| match event {
            EngineEvent::ExternalsChanged { processes } if predicate(processes) => {
                Some(processes.clone())
            }
            _ => None,
        })
        .await
    }
}

fn external_owner(pid: u32) -> TunnelState {
    external_owner_named(pid, SSH_NAME)
}

fn real_external_owner(pid: u32) -> TunnelState {
    external_owner_named(pid, REAL_SSH_NAME)
}

fn external_owner_named(pid: u32, process: &str) -> TunnelState {
    TunnelState::External {
        owner: PortOwner {
            pid,
            process: process.to_owned(),
        },
    }
}

fn is_external(state: &TunnelState) -> bool {
    matches!(state, TunnelState::External { .. })
}

fn is_conflict(state: &TunnelState) -> bool {
    matches!(state, TunnelState::Conflict { .. })
}

fn unmanaged_ssh_scan(base: SystemScan) -> SystemScan {
    let unmanaged = ssh_scan(
        OTHER_SSH_PID,
        &[(UNMANAGED_PORT, format!("{UNMANAGED_PORT}:db:5432"))],
    );
    SystemScan {
        listeners: [base.listeners, unmanaged.listeners].concat(),
        processes: [base.processes, unmanaged.processes].concat(),
    }
}

#[tokio::test]
async fn should_report_external_when_scanning_finds_a_matching_ssh_forward() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(harness.scan_with_ssh_providing(SSH_PID, &["alpha"]));

    harness.engine.set_port_scanning(true);
    let state = harness.wait_for("alpha", is_external).await;

    assert_eq!(state, external_owner(SSH_PID));
}

#[tokio::test]
async fn should_report_conflict_when_scanning_finds_another_program_on_the_port() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(foreign_scan(NGINX_PID, "nginx", harness.port("alpha")));

    harness.engine.set_port_scanning(true);
    let state = harness.wait_for("alpha", is_conflict).await;

    assert_eq!(
        state,
        TunnelState::Conflict {
            owner: Some(PortOwner {
                pid: NGINX_PID,
                process: "nginx".to_owned()
            })
        }
    );
}

#[tokio::test]
async fn should_return_to_stopped_when_the_external_listener_disappears() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(harness.scan_with_ssh_providing(SSH_PID, &["alpha"]));
    harness.engine.set_port_scanning(true);
    harness.wait_for("alpha", is_external).await;

    harness.set_scan(SystemScan::default());

    harness.wait_stopped("alpha").await;
}

#[tokio::test]
async fn should_not_scan_when_port_scanning_is_off() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.scanner.scan_count(), 0);
}

#[tokio::test]
async fn should_scan_repeatedly_when_port_scanning_is_on() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    harness.engine.set_port_scanning(true);

    wait_until(WAIT, "several scans", || harness.scanner.scan_count() >= 3).await;
}

#[tokio::test]
async fn should_stop_scanning_when_port_scanning_is_turned_off() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.engine.set_port_scanning(true);
    wait_until(WAIT, "a scan", || harness.scanner.scan_count() >= 1).await;
    harness.engine.set_port_scanning(false);
    sleep(TICK_INTERVAL * 2).await;
    let count = harness.scanner.scan_count();

    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.scanner.scan_count(), count);
}

#[tokio::test]
async fn should_keep_the_states_when_a_scan_fails() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(harness.scan_with_ssh_providing(SSH_PID, &["alpha"]));
    harness.engine.set_port_scanning(true);
    harness.wait_for("alpha", is_external).await;

    harness.scanner.fail_scans();
    let count = harness.scanner.scan_count();
    wait_until(WAIT, "failing scans", || {
        harness.scanner.scan_count() > count + 1
    })
    .await;

    assert_eq!(harness.state_of("alpha"), external_owner(SSH_PID));
}

async fn start_on_taken_port(harness: &mut Harness) -> TunnelState {
    let squatter = TcpListener::bind((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();
    harness.start_tunnel("alpha");
    let state = harness
        .wait_for("alpha", |state| {
            is_external(state) || is_conflict(state) || matches!(state, TunnelState::Failed { .. })
        })
        .await;
    drop(squatter);
    state
}

#[tokio::test]
async fn should_report_external_when_starting_on_a_port_held_by_a_matching_ssh() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(harness.scan_with_ssh_providing(SSH_PID, &["alpha"]));

    let state = start_on_taken_port(&mut harness).await;

    assert_eq!(state, external_owner(SSH_PID));
}

#[tokio::test]
async fn should_report_conflict_when_starting_on_a_port_held_by_another_program() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(foreign_scan(NGINX_PID, "nginx", harness.port("alpha")));

    let state = start_on_taken_port(&mut harness).await;

    assert!(matches!(
        state,
        TunnelState::Conflict { owner: Some(owner) } if owner.pid == NGINX_PID
    ));
}

#[tokio::test]
async fn should_report_conflict_without_owner_when_starting_on_a_port_the_scan_cannot_see() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;

    let state = start_on_taken_port(&mut harness).await;

    assert_eq!(state, TunnelState::Conflict { owner: None });
}

#[tokio::test]
async fn should_keep_the_port_in_use_failure_when_the_scan_fails_at_start() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.scanner.fail_scans();

    let state = start_on_taken_port(&mut harness).await;

    assert!(matches!(
        state,
        TunnelState::Failed {
            reason: FailureReason::PortInUse { .. }
        }
    ));
}

#[tokio::test]
async fn should_keep_the_external_state_when_the_tunnel_is_stopped() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(harness.scan_with_ssh_providing(SSH_PID, &["alpha"]));
    harness.engine.set_port_scanning(true);
    harness.wait_for("alpha", is_external).await;

    harness.stop_tunnel("alpha");
    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.state_of("alpha"), external_owner(SSH_PID));
}

#[tokio::test]
async fn should_terminate_once_and_start_every_tunnel_of_the_process_when_adopting() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"]).start().await;
    harness.set_scan(harness.scan_with_ssh_providing(SSH_PID, &["alpha", "beta"]));
    harness.engine.set_port_scanning(true);
    harness.wait_for("alpha", is_external).await;
    harness.wait_for("beta", is_external).await;

    let adopted = harness
        .engine
        .adopt_external(harness.id("alpha"))
        .await
        .unwrap();

    harness.wait_all_active(&["alpha", "beta"]).await;
    assert_eq!(adopted, vec![harness.id("alpha"), harness.id("beta")]);
    assert_eq!(harness.scanner.terminated(), vec![identity_of(SSH_PID)]);
}

#[tokio::test]
async fn should_refuse_to_adopt_when_the_tunnel_is_not_external() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let error = harness
        .engine
        .adopt_external(harness.id("alpha"))
        .await
        .unwrap_err();

    assert!(matches!(error, EngineError::NotExternal(id) if id == harness.id("alpha")));
}

#[tokio::test]
async fn should_fail_with_unknown_tunnel_when_adopting_a_tunnel_that_does_not_exist() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let error = harness
        .engine
        .adopt_external(TunnelId::generate())
        .await
        .unwrap_err();

    assert!(matches!(error, EngineError::UnknownTunnel(_)));
}

#[tokio::test]
async fn should_refuse_to_kill_when_the_process_is_not_reported() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let error = harness.engine.kill_external(NGINX_PID).await.unwrap_err();

    assert!(matches!(error, EngineError::UnknownProcess(NGINX_PID)));
    assert_eq!(harness.scanner.terminated(), Vec::new());
}

#[tokio::test]
async fn should_terminate_and_stop_reporting_when_an_unmanaged_process_is_killed() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(unmanaged_ssh_scan(SystemScan::default()));
    harness.engine.set_port_scanning(true);
    harness
        .wait_for_externals(|processes| !processes.is_empty())
        .await;

    harness.engine.kill_external(OTHER_SSH_PID).await.unwrap();

    assert_eq!(
        harness.scanner.terminated(),
        vec![identity_of(OTHER_SSH_PID)]
    );
    harness
        .wait_for_externals(<[UnmanagedProcess]>::is_empty)
        .await;
}

#[tokio::test]
async fn should_publish_unmanaged_processes_when_ssh_forwards_match_no_tunnel() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(unmanaged_ssh_scan(SystemScan::default()));

    harness.engine.set_port_scanning(true);
    let processes = harness
        .wait_for_externals(|processes| !processes.is_empty())
        .await;

    assert_eq!(processes[0].pid, OTHER_SSH_PID);
    assert_eq!(harness.engine.snapshot().externals, processes);
}

#[tokio::test]
async fn should_hide_the_unmanaged_process_when_it_is_ignored() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(unmanaged_ssh_scan(SystemScan::default()));
    harness.engine.set_port_scanning(true);
    harness
        .wait_for_externals(|processes| !processes.is_empty())
        .await;

    harness.engine.ignore_external(OTHER_SSH_PID).await.unwrap();

    harness
        .wait_for_externals(<[UnmanagedProcess]>::is_empty)
        .await;
    assert_eq!(harness.scanner.terminated(), Vec::new());
}

#[tokio::test]
async fn should_stop_the_external_state_when_its_owner_is_ignored() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(harness.scan_with_ssh_providing(SSH_PID, &["alpha"]));
    harness.engine.set_port_scanning(true);
    harness.wait_for("alpha", is_external).await;

    harness.engine.ignore_external(SSH_PID).await.unwrap();

    harness.wait_stopped("alpha").await;
}

#[tokio::test]
async fn should_refuse_to_ignore_when_the_process_is_not_reported() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let error = harness.engine.ignore_external(NGINX_PID).await.unwrap_err();

    assert!(matches!(error, EngineError::UnknownProcess(NGINX_PID)));
}

#[tokio::test]
async fn should_refuse_the_external_actions_when_the_engine_is_shut_down() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.engine.shutdown().await;

    let error = harness.engine.kill_external(SSH_PID).await.unwrap_err();

    assert!(matches!(error, EngineError::ShutDown));
}

struct FakeSsh {
    child: std::process::Child,
    _directory: tempfile::TempDir,
}

impl FakeSsh {
    fn spawn(forwards: &[String]) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let program = directory
            .path()
            .join(format!("ssh{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(env!("CARGO_BIN_EXE_fake-ssh"), &program).unwrap();
        let mut command = std::process::Command::new(&program);
        command.arg("-N");
        for forward in forwards {
            command.args(["-L", forward]);
        }
        command.arg(support::ALIAS);
        let child = command.spawn().unwrap();
        Self {
            child,
            _directory: directory,
        }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn has_exited(&mut self) -> bool {
        self.child.try_wait().unwrap().is_some()
    }
}

impl Drop for FakeSsh {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn wait_until_listening(port: u16) {
    let deadline = tokio::time::Instant::now() + WAIT;
    while TcpStream::connect((LOOPBACK, port)).await.is_err() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "nothing listens on port {port}"
        );
        sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn should_adopt_a_real_ssh_process_when_it_provides_the_tunnel() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .port_scanner(Arc::new(SystemPortScanner))
        .start()
        .await;
    let (port, forward) = harness.forward_of("alpha");
    let mut ssh = FakeSsh::spawn(&[forward]);
    wait_until_listening(port).await;

    harness.engine.set_port_scanning(true);
    let state = harness.wait_for("alpha", is_external).await;
    harness
        .engine
        .adopt_external(harness.id("alpha"))
        .await
        .unwrap();

    assert_eq!(state, real_external_owner(ssh.pid()));
    assert!(ssh.has_exited());
    harness.wait_active("alpha").await;
    assert_eq!(echo_through(port, PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_list_a_real_ssh_process_when_its_forward_matches_no_tunnel() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .port_scanner(Arc::new(SystemPortScanner))
        .start()
        .await;
    let other_port = free_port().await;
    let ssh = FakeSsh::spawn(&[format!("{other_port}:db:5432")]);
    wait_until_listening(other_port).await;

    harness.engine.set_port_scanning(true);
    let pid = ssh.pid();
    let processes = harness
        .wait_for_externals(|processes| processes.iter().any(|process| process.pid == pid))
        .await;

    let listed = processes.iter().find(|process| process.pid == pid).unwrap();
    assert_eq!(listed.forwards[0].listen_port, other_port);
    assert_eq!(harness.state_of("alpha"), TunnelState::Stopped);
}

fn wanted_ids(harness: &Harness) -> Vec<TunnelId> {
    harness.state_store.state().running.into_iter().collect()
}

fn sorted(mut ids: Vec<TunnelId>) -> Vec<TunnelId> {
    ids.sort();
    ids
}

async fn wait_until_wanted(harness: &Harness, name: &str, is_expected: bool) {
    let id = harness.id(name);
    let store = Arc::clone(&harness.state_store);
    wait_until(WAIT, "the saved state to follow the command", move || {
        store.state().running.contains(&id) == is_expected
    })
    .await;
}

async fn configure_launch(harness: &Harness, is_restore_enabled: bool, tags: &[&str]) {
    let settings = Settings {
        restore_on_launch: is_restore_enabled,
        start_on_launch_tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        ..harness.engine.snapshot().settings
    };
    harness
        .engine
        .update_configuration(settings, Vec::new())
        .await
        .unwrap();
}

#[tokio::test]
async fn should_record_the_tunnel_as_wanted_when_it_is_started() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    harness.start_tunnel("alpha");

    wait_until_wanted(&harness, "alpha", true).await;
}

#[tokio::test]
async fn should_forget_the_tunnel_when_it_is_stopped() {
    let harness = HarnessBuilder::new(&["alpha"])
        .previously_running(&["alpha"])
        .start()
        .await;

    harness.stop_tunnel("alpha");

    wait_until_wanted(&harness, "alpha", false).await;
}

#[tokio::test]
async fn should_record_the_started_tunnels_when_a_tag_is_started() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"]), ("beta", &["web"])])
        .start()
        .await;

    harness.engine.start_tag("dev").unwrap();

    wait_until_wanted(&harness, "alpha", true).await;
    assert!(!wanted_ids(&harness).contains(&harness.id("beta")));
}

#[tokio::test]
async fn should_forget_the_stopped_tunnels_when_a_tag_is_stopped() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"]), ("beta", &["web"])])
        .previously_running(&["alpha", "beta"])
        .start()
        .await;

    harness.engine.stop_tag("dev").unwrap();

    wait_until_wanted(&harness, "alpha", false).await;
    assert!(wanted_ids(&harness).contains(&harness.id("beta")));
}

#[tokio::test]
async fn should_forget_the_tunnel_when_it_is_deleted() {
    let harness = HarnessBuilder::new(&["alpha"])
        .previously_running(&["alpha"])
        .start()
        .await;

    harness
        .engine
        .delete_tunnel(harness.id("alpha"))
        .await
        .unwrap();

    wait_until_wanted(&harness, "alpha", false).await;
}

#[tokio::test]
async fn should_keep_the_tunnel_wanted_when_it_is_restarted() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    wait_until_wanted(&harness, "alpha", true).await;

    harness.engine.restart_tunnel(harness.id("alpha")).unwrap();
    harness
        .wait_for("alpha", |state| matches!(state, TunnelState::Active))
        .await;

    assert!(wanted_ids(&harness).contains(&harness.id("alpha")));
}

#[tokio::test]
async fn should_record_the_adopted_tunnels_when_adopting_an_external() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"]).start().await;
    harness.set_scan(harness.scan_with_ssh_providing(SSH_PID, &["alpha", "beta"]));
    harness.engine.set_port_scanning(true);
    harness.wait_for("alpha", is_external).await;
    harness.wait_for("beta", is_external).await;

    harness
        .engine
        .adopt_external(harness.id("alpha"))
        .await
        .unwrap();

    wait_until_wanted(&harness, "alpha", true).await;
    wait_until_wanted(&harness, "beta", true).await;
}

#[tokio::test]
async fn should_not_forget_the_running_tunnels_when_the_engine_shuts_down() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    wait_until_wanted(&harness, "alpha", true).await;

    harness.engine.shutdown().await;

    assert_eq!(wanted_ids(&harness), vec![harness.id("alpha")]);
}

#[tokio::test]
async fn should_drop_the_ids_that_left_the_configuration_when_the_state_is_next_written() {
    let harness = HarnessBuilder::new(&["alpha", "beta"])
        .previously_running(&["alpha", "ghost"])
        .start()
        .await;

    harness.start_tunnel("beta");

    wait_until_wanted(&harness, "beta", true).await;
    assert_eq!(
        wanted_ids(&harness),
        sorted(vec![harness.id("alpha"), harness.id("beta")])
    );
}

#[tokio::test]
async fn should_start_the_previously_running_tunnels_when_restoring_on_launch() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"])
        .previously_running(&["alpha", "ghost"])
        .start()
        .await;
    configure_launch(&harness, true, &[]).await;

    let started = harness.engine.start_launch_tunnels().unwrap();

    harness.wait_all_active(&["alpha"]).await;
    assert_eq!(started, vec![harness.id("alpha")]);
    assert_eq!(harness.state_of("beta"), TunnelState::Stopped);
}

#[tokio::test]
async fn should_start_nothing_when_restoring_on_launch_is_off() {
    let harness = HarnessBuilder::new(&["alpha"])
        .previously_running(&["alpha"])
        .start()
        .await;
    configure_launch(&harness, false, &[]).await;

    let started = harness.engine.start_launch_tunnels().unwrap();

    assert_eq!(started, Vec::<TunnelId>::new());
}

#[tokio::test]
async fn should_start_the_tagged_tunnels_when_their_tag_starts_on_launch() {
    let mut harness = HarnessBuilder::tagged(&[
        ("alpha", &["dev"]),
        ("beta", &["web"]),
        ("gamma", &["dev", "web"]),
    ])
    .start()
    .await;
    configure_launch(&harness, false, &["dev"]).await;

    let started = harness.engine.start_launch_tunnels().unwrap();

    harness.wait_all_active(&["alpha", "gamma"]).await;
    assert_eq!(started, vec![harness.id("alpha"), harness.id("gamma")]);
}

#[tokio::test]
async fn should_start_each_tunnel_once_when_restored_and_tagged() {
    let mut harness =
        HarnessBuilder::tagged(&[("alpha", &["dev"]), ("beta", &[]), ("gamma", &["dev"])])
            .previously_running(&["alpha", "beta"])
            .start()
            .await;
    configure_launch(&harness, true, &["dev"]).await;

    let started = harness.engine.start_launch_tunnels().unwrap();

    harness.wait_all_active(&["alpha", "beta", "gamma"]).await;
    assert_eq!(
        started,
        vec![harness.id("alpha"), harness.id("beta"), harness.id("gamma")]
    );
}

#[tokio::test]
async fn should_not_record_the_launch_starts_as_wishes_of_the_user() {
    let mut harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;
    configure_launch(&harness, false, &["dev"]).await;

    harness.engine.start_launch_tunnels().unwrap();
    harness.wait_all_active(&["alpha"]).await;
    sleep(QUIET_PERIOD).await;

    assert_eq!(wanted_ids(&harness), Vec::<TunnelId>::new());
}

fn temporary_tunnel(listen_port: u16, target_port: u16) -> NewTunnel {
    NewTunnel {
        tags: Vec::new(),
        target_port: Some(target_port),
        ..new_tunnel(listen_port)
    }
}

fn snapshot_of(harness: &Harness, id: TunnelId) -> Option<engine::TunnelSnapshot> {
    harness
        .engine
        .snapshot()
        .tunnels
        .into_iter()
        .find(|tunnel| tunnel.definition.id == id)
}

async fn start_active_temporary(harness: &mut Harness) -> (TunnelId, u16) {
    let port = free_port().await;
    let id = harness
        .engine
        .start_temporary_tunnel(temporary_tunnel(port, harness.echo.port))
        .await
        .unwrap();
    wait_for_state(
        &mut harness.events,
        id,
        |state| matches!(state, TunnelState::Active),
        WAIT,
    )
    .await;
    (id, port)
}

#[tokio::test]
async fn should_run_a_temporary_tunnel_like_any_other_when_it_is_started() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;

    let (id, port) = start_active_temporary(&mut harness).await;

    assert_eq!(echo_through(port, PAYLOAD).await, PAYLOAD);
    assert!(snapshot_of(&harness, id).unwrap().is_temporary);
}

#[tokio::test]
async fn should_drop_the_tags_when_a_temporary_tunnel_is_started() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let id = harness
        .engine
        .start_temporary_tunnel(new_tunnel(free_port().await))
        .await
        .unwrap();

    assert_eq!(
        snapshot_of(&harness, id).unwrap().definition.tags,
        Vec::<String>::new()
    );
}

#[tokio::test]
async fn should_never_write_a_temporary_tunnel_when_the_store_is_saved_by_other_changes() {
    let mut harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;
    let (_, temporary_port) = start_active_temporary(&mut harness).await;

    let created_port = free_port().await;
    harness
        .engine
        .create_tunnel(new_tunnel(created_port))
        .await
        .unwrap();
    harness.engine.rename_tag("dev", "staging").await.unwrap();
    harness.engine.delete_tag("staging").await.unwrap();
    let settings = harness.engine.snapshot().settings;
    harness
        .engine
        .update_configuration(settings, Vec::new())
        .await
        .unwrap();
    let alpha = harness.engine.snapshot().tunnels.remove(0).definition;
    harness.engine.update_tunnel(alpha).await.unwrap();
    harness
        .engine
        .delete_tunnel(harness.id("alpha"))
        .await
        .unwrap();

    assert_eq!(saved_tunnel_ports(&harness), [created_port]);
    let file_text = std::fs::read_to_string(harness.store_path()).unwrap();
    assert!(!file_text.contains(&temporary_port.to_string()));
}

#[tokio::test]
async fn should_keep_a_temporary_tunnel_when_the_store_file_is_edited() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let (id, _) = start_active_temporary(&mut harness).await;

    harness.rewrite_store();
    std::fs::write(
        harness.store_path(),
        format!(
            "{}\n",
            std::fs::read_to_string(harness.store_path()).unwrap()
        ),
    )
    .unwrap();
    harness.wait_for_config_reload().await;

    assert!(snapshot_of(&harness, id).is_some());
    assert_eq!(harness.engine.snapshot().tunnels.len(), 2);
}

#[tokio::test]
async fn should_not_remember_a_temporary_tunnel_as_wanted_when_it_runs() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    start_active_temporary(&mut harness).await;
    harness.start_tunnel("alpha");
    wait_until_wanted(&harness, "alpha", true).await;

    assert_eq!(wanted_ids(&harness), vec![harness.id("alpha")]);
}

#[tokio::test]
async fn should_not_start_a_temporary_tunnel_when_the_launch_tunnels_start() {
    let mut harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;
    configure_launch(&harness, true, &["dev"]).await;
    let (id, _) = start_active_temporary(&mut harness).await;

    let started = harness.engine.start_launch_tunnels().unwrap();

    assert_eq!(started, vec![harness.id("alpha")]);
    assert!(!started.contains(&id));
}

#[tokio::test]
async fn should_remove_the_tunnel_and_free_its_port_when_a_temporary_tunnel_is_stopped() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let (id, port) = start_active_temporary(&mut harness).await;

    harness.engine.stop_tunnel(id).unwrap();

    harness.wait_for_config_reload().await;
    assert!(snapshot_of(&harness, id).is_none());
    wait_until(WAIT, "the port to be released", || {
        std::net::TcpListener::bind((LOOPBACK, port)).is_ok()
    })
    .await;
}

#[tokio::test]
async fn should_remove_the_tunnel_when_a_temporary_tunnel_is_deleted() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let (id, _) = start_active_temporary(&mut harness).await;

    harness.engine.delete_tunnel(id).await.unwrap();

    assert!(snapshot_of(&harness, id).is_none());
    assert_eq!(saved_tunnel_ports(&harness), [harness.port("alpha")]);
}

#[tokio::test]
async fn should_leave_a_temporary_tunnel_alone_when_a_tag_is_stopped() {
    let mut harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;
    let (id, _) = start_active_temporary(&mut harness).await;

    harness.engine.stop_tag("dev").unwrap();
    sleep(QUIET_PERIOD).await;

    assert!(snapshot_of(&harness, id).is_some());
}

#[tokio::test]
async fn should_keep_a_temporary_tunnel_in_its_failed_state_when_the_connection_fails() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .with_unauthorized_client_key()
        .start()
        .await;

    let id = harness
        .engine
        .start_temporary_tunnel(temporary_tunnel(free_port().await, harness.echo.port))
        .await
        .unwrap();
    wait_for_state(
        &mut harness.events,
        id,
        |state| matches!(state, TunnelState::Failed { .. }),
        WAIT,
    )
    .await;

    let snapshot = snapshot_of(&harness, id).unwrap();
    assert!(snapshot.is_temporary && matches!(snapshot.state, TunnelState::Failed { .. }));
}

#[tokio::test]
async fn should_refuse_a_temporary_tunnel_when_its_definition_is_invalid() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let outcome = harness
        .engine
        .start_temporary_tunnel(temporary_tunnel(0, harness.echo.port))
        .await;

    assert!(matches!(
        outcome,
        Err(EngineError::Store(StoreError::Validation(_)))
    ));
}

#[tokio::test]
async fn should_report_a_conflict_when_the_port_of_a_temporary_tunnel_is_already_taken() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let squatter = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let port = squatter.local_addr().unwrap().port();

    let id = harness
        .engine
        .start_temporary_tunnel(temporary_tunnel(port, harness.echo.port))
        .await
        .unwrap();

    let state = wait_for_state(
        &mut harness.events,
        id,
        |state| matches!(state, TunnelState::Conflict { .. }),
        WAIT,
    )
    .await;
    assert!(matches!(state, TunnelState::Conflict { .. }));
}

#[tokio::test]
async fn should_refuse_to_update_a_tunnel_when_it_is_temporary() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let (id, _) = start_active_temporary(&mut harness).await;
    let definition = snapshot_of(&harness, id).unwrap().definition;

    let outcome = harness.engine.update_tunnel(definition).await;

    assert!(matches!(outcome, Err(EngineError::TunnelIsTemporary(_))));
}

#[tokio::test]
async fn should_save_the_tunnel_once_and_keep_it_running_when_it_is_persisted() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let (id, port) = start_active_temporary(&mut harness).await;

    harness.engine.persist_tunnel(id).await.unwrap();

    assert_eq!(saved_tunnel_ports(&harness), [harness.port("alpha"), port]);
    let snapshot = snapshot_of(&harness, id).unwrap();
    assert!(!snapshot.is_temporary && snapshot.state == TunnelState::Active);
    assert_eq!(echo_through(port, PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_not_restart_the_tunnel_when_it_is_persisted() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let (id, _) = start_active_temporary(&mut harness).await;
    let connections = harness.server.connection_count();

    harness.engine.persist_tunnel(id).await.unwrap();
    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.server.connection_count(), connections);
}

#[tokio::test]
async fn should_remember_the_tunnel_as_wanted_when_a_running_temporary_tunnel_is_persisted() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let (id, _) = start_active_temporary(&mut harness).await;

    harness.engine.persist_tunnel(id).await.unwrap();

    let store = Arc::clone(&harness.state_store);
    wait_until(WAIT, "the saved state to follow the command", move || {
        store.state().running.contains(&id)
    })
    .await;
}

#[tokio::test]
async fn should_keep_a_persisted_tunnel_when_it_is_stopped() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let (id, _) = start_active_temporary(&mut harness).await;
    harness.engine.persist_tunnel(id).await.unwrap();

    harness.engine.stop_tunnel(id).unwrap();
    wait_for_state(
        &mut harness.events,
        id,
        |state| matches!(state, TunnelState::Stopped),
        WAIT,
    )
    .await;

    assert!(snapshot_of(&harness, id).is_some());
}

#[tokio::test]
async fn should_fail_with_unknown_tunnel_when_persisting_a_tunnel_that_does_not_exist() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let outcome = harness.engine.persist_tunnel(TunnelId::generate()).await;

    assert!(matches!(outcome, Err(EngineError::UnknownTunnel(_))));
}

#[tokio::test]
async fn should_fail_with_not_temporary_when_persisting_a_saved_tunnel() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let outcome = harness.engine.persist_tunnel(harness.id("alpha")).await;

    assert!(matches!(outcome, Err(EngineError::TunnelIsNotTemporary(_))));
}

#[tokio::test]
async fn should_list_the_temporary_tunnels_last_when_a_tunnel_is_created_after_them() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    let (temporary_id, _) = start_active_temporary(&mut harness).await;

    let created_id = harness
        .engine
        .create_tunnel(new_tunnel(free_port().await))
        .await
        .unwrap();

    let order: Vec<TunnelId> = harness
        .engine
        .snapshot()
        .tunnels
        .iter()
        .map(|tunnel| tunnel.definition.id)
        .collect();
    assert_eq!(order, vec![harness.id("alpha"), created_id, temporary_id]);
}

#[tokio::test]
async fn should_start_a_stopped_tunnel_and_return_once_it_is_active() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    harness
        .engine
        .start_tunnel_and_wait(harness.id("alpha"))
        .await
        .unwrap();

    assert_eq!(harness.state_of("alpha"), TunnelState::Active);
    assert_eq!(echo_through(harness.port("alpha"), PAYLOAD).await, PAYLOAD);
}

#[tokio::test]
async fn should_return_at_once_without_a_new_connection_when_the_tunnel_is_already_active() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    harness
        .engine
        .start_tunnel_and_wait(harness.id("alpha"))
        .await
        .unwrap();

    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_fail_with_the_reason_when_the_tunnel_cannot_start() {
    let harness = HarnessBuilder::new(&["alpha"])
        .with_unauthorized_client_key()
        .start()
        .await;

    let error = harness
        .engine
        .start_tunnel_and_wait(harness.id("alpha"))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, EngineError::TunnelStartFailed(FailureReason::Connect { message }) if message.contains("authentication failed")),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_fail_when_another_program_holds_the_port_of_the_tunnel() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let _squatter = TcpListener::bind((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();

    let error = harness
        .engine
        .start_tunnel_and_wait(harness.id("alpha"))
        .await
        .unwrap_err();

    assert!(
        matches!(error, EngineError::TunnelPortConflict),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_return_at_once_when_an_external_process_provides_the_tunnel() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.set_scan(harness.scan_with_ssh_providing(SSH_PID, &["alpha"]));
    harness.engine.set_port_scanning(true);
    harness.wait_for("alpha", is_external).await;

    harness
        .engine
        .start_tunnel_and_wait(harness.id("alpha"))
        .await
        .unwrap();

    assert_eq!(harness.server.connection_count(), 0);
}

#[tokio::test]
async fn should_fail_when_the_tunnel_to_wait_for_is_unknown() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let error = harness
        .engine
        .start_tunnel_and_wait(TunnelId::generate())
        .await
        .unwrap_err();

    assert!(matches!(error, EngineError::UnknownTunnel(_)), "{error:?}");
}

fn browser_url_of(harness: &Harness) -> String {
    match harness.engine.open_plan(harness.id("alpha"), None).unwrap() {
        OpenPlan::Browser { url } => url,
        other @ OpenPlan::Program { .. } => panic!("expected the browser, got {other:?}"),
    }
}

fn viewer(ports: &[u16]) -> Opener {
    Opener {
        id: "viewer".to_owned(),
        name: "Viewer".to_owned(),
        command: "viewer".to_owned(),
        args: vec!["{host}:{port}".to_owned()],
        ports: ports.to_vec(),
    }
}

#[tokio::test]
async fn should_plan_the_opener_of_the_user_when_it_matches_the_target_port() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let target_port = harness.echo.port;
    harness
        .engine
        .update_configuration(
            harness.engine.snapshot().settings,
            vec![viewer(&[target_port])],
        )
        .await
        .unwrap();

    let plan = harness.engine.open_plan(harness.id("alpha"), None).unwrap();

    assert_eq!(
        plan,
        OpenPlan::Program {
            name: "Viewer".to_owned(),
            command: LaunchCommand {
                program: "viewer".to_owned(),
                args: vec![format!("localhost:{}", harness.port("alpha"))],
            },
        }
    );
}

#[tokio::test]
async fn should_plan_the_browser_when_it_is_asked_for_explicitly() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let target_port = harness.echo.port;
    harness
        .engine
        .update_configuration(
            harness.engine.snapshot().settings,
            vec![viewer(&[target_port])],
        )
        .await
        .unwrap();

    let plan = harness
        .engine
        .open_plan(harness.id("alpha"), Some("browser"))
        .unwrap();

    assert!(matches!(plan, OpenPlan::Browser { .. }), "{plan:?}");
}

#[tokio::test]
async fn should_list_the_available_ids_when_the_opener_is_unknown() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness
        .engine
        .update_configuration(harness.engine.snapshot().settings, vec![viewer(&[])])
        .await
        .unwrap();

    let error = harness
        .engine
        .open_plan(harness.id("alpha"), Some("ghost"))
        .unwrap_err();

    assert!(
        matches!(&error, EngineError::UnknownOpener { available, .. } if available == &["browser", "viewer"]),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_save_the_openers_to_the_store_file() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    harness
        .engine
        .update_configuration(harness.engine.snapshot().settings, vec![viewer(&[5900])])
        .await
        .unwrap();

    let text = std::fs::read_to_string(harness.store_path()).unwrap();
    assert!(text.contains("[[openers]]"), "{text}");
}

#[tokio::test]
async fn should_refuse_openers_that_break_a_rule_and_keep_the_previous_ones() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let invalid = Opener {
        id: "browser".to_owned(),
        ..viewer(&[])
    };

    let error = harness
        .engine
        .update_configuration(harness.engine.snapshot().settings, vec![invalid])
        .await
        .unwrap_err();

    assert!(
        matches!(error, EngineError::Store(StoreError::Validation(_))),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_list_detected_presets_after_the_openers_of_the_user_in_the_snapshot() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness
        .engine
        .update_configuration(harness.engine.snapshot().settings, vec![viewer(&[])])
        .await
        .unwrap();

    let ids: Vec<String> = harness
        .engine
        .snapshot()
        .openers
        .into_iter()
        .map(|available| available.opener.id)
        .collect();

    assert_eq!(ids, ["viewer"]);
}

#[tokio::test]
async fn should_build_the_browser_url_of_a_local_tunnel() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let url = browser_url_of(&harness);

    assert_eq!(url, format!("http://localhost:{}/", harness.port("alpha")));
}

#[tokio::test]
async fn should_refuse_the_browser_url_of_a_remote_tunnel() {
    let harness = HarnessBuilder::new(&["alpha"]).kind("remote").start().await;

    let error = harness
        .engine
        .open_plan(harness.id("alpha"), None)
        .unwrap_err();

    assert!(
        matches!(
            error,
            EngineError::Open(OpenUrlError::NotOpenable {
                kind: TunnelKind::Remote
            })
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_follow_the_open_host_setting_when_building_the_browser_url() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;
    let settings = Settings {
        open_host: OpenHost::Ip,
        ..harness.engine.snapshot().settings
    };
    harness
        .engine
        .update_configuration(settings, Vec::new())
        .await
        .unwrap();

    let url = browser_url_of(&harness);

    assert_eq!(url, format!("http://127.0.0.1:{}/", harness.port("alpha")));
}

#[tokio::test]
async fn should_bind_without_opening_a_session_when_a_tunnel_starts_in_standby() {
    let harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;

    harness
        .engine
        .start_tunnel_and_wait(harness.id("alpha"))
        .await
        .unwrap();

    assert_eq!(
        harness.state_of("alpha"),
        TunnelState::Standby { last_failure: None }
    );
    assert_eq!(harness.server.connection_count(), 0);
    assert!(!can_bind(harness.port("alpha")).await);
}

#[tokio::test]
async fn should_start_in_standby_when_only_the_tunnel_asks_for_it() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"])
        .standby_override("alpha", true)
        .start()
        .await;

    harness.start_tunnel("alpha");
    harness.start_tunnel("beta");

    harness.wait_standby("alpha").await;
    harness.wait_active("beta").await;
}

#[tokio::test]
async fn should_start_active_when_the_tunnel_refuses_the_standby_of_the_settings() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .standby_override("alpha", false)
        .start()
        .await;

    harness.start_and_wait_active("alpha").await;

    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_wake_up_forward_and_publish_a_woke_event_when_a_client_connects_to_a_standby_tunnel()
 {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    let id = harness.id("alpha");

    let echoed = echo_through(harness.port("alpha"), PAYLOAD).await;

    assert_eq!(echoed, PAYLOAD);
    assert_eq!(harness.state_of("alpha"), TunnelState::Active);
    assert_eq!(harness.server.connection_count(), 1);
    wait_for_event(&mut harness.events, WAIT, |event| {
        matches!(event, EngineEvent::TunnelWoke { id: woke } if *woke == id).then_some(())
    })
    .await;
}

#[tokio::test]
async fn should_serve_every_held_connection_when_several_clients_wake_a_standby_tunnel() {
    const CLIENT_COUNT: usize = 5;
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    let port = harness.port("alpha");

    let clients: Vec<_> = (0..CLIENT_COUNT)
        .map(|_| tokio::spawn(echo_through(port, PAYLOAD)))
        .collect();

    for client in clients {
        assert_eq!(client.await.unwrap(), PAYLOAD);
    }
    assert_eq!(harness.target_requests(), CLIENT_COUNT);
    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_wake_up_a_standby_dynamic_tunnel_when_a_socks_client_connects() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .kind("dynamic")
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;

    let echoed =
        echo_through_socks(harness.port("alpha"), LOOPBACK, harness.echo.port, PAYLOAD).await;

    assert_eq!(echoed, PAYLOAD);
}

#[tokio::test]
async fn should_go_back_to_standby_and_release_the_session_when_no_connection_stays_open() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .standby_idle(SHORT_STANDBY_IDLE)
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    echo_through(harness.port("alpha"), PAYLOAD).await;

    harness.wait_standby("alpha").await;

    wait_until(WAIT, "the session to close", || {
        harness.server.open_connection_count() == 0
    })
    .await;
}

#[tokio::test]
async fn should_wake_up_again_when_a_client_connects_after_the_tunnel_went_back_to_standby() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .standby_idle(SHORT_STANDBY_IDLE)
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    echo_through(harness.port("alpha"), PAYLOAD).await;
    harness.wait_standby("alpha").await;

    let echoed = echo_through(harness.port("alpha"), PAYLOAD).await;

    assert_eq!(echoed, PAYLOAD);
}

async fn connect_and_wait_active(harness: &mut Harness, name: &str) -> TcpStream {
    let client = TcpStream::connect((LOOPBACK, harness.port(name)))
        .await
        .unwrap();
    harness.wait_active(name).await;
    client
}

async fn is_closed_by_the_peer(client: &mut TcpStream) -> bool {
    let mut buffer = [0_u8; 1];
    matches!(
        timeout(WAIT, client.read(&mut buffer)).await,
        Ok(Ok(0) | Err(_))
    )
}

#[tokio::test]
async fn should_go_back_to_standby_and_close_the_open_connection_when_an_awake_tunnel_is_sent_to_standby()
 {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    let mut client = connect_and_wait_active(&mut harness, "alpha").await;

    harness.engine.standby_tunnel(harness.id("alpha")).unwrap();

    harness.wait_standby("alpha").await;
    assert!(is_closed_by_the_peer(&mut client).await);
    assert!(!can_bind(harness.port("alpha")).await);
}

#[tokio::test]
async fn should_wake_up_again_when_a_client_connects_after_the_tunnel_was_sent_to_standby() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    let _client = connect_and_wait_active(&mut harness, "alpha").await;
    harness.engine.standby_tunnel(harness.id("alpha")).unwrap();
    harness.wait_standby("alpha").await;

    let echoed = echo_through(harness.port("alpha"), PAYLOAD).await;

    assert_eq!(echoed, PAYLOAD);
}

#[tokio::test]
async fn should_fail_with_not_awake_when_a_tunnel_on_standby_is_sent_to_standby() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;

    let outcome = harness.engine.standby_tunnel(harness.id("alpha"));

    assert!(matches!(outcome, Err(EngineError::TunnelNotAwake(_))));
}

#[tokio::test]
async fn should_fail_with_not_awake_and_keep_serving_when_a_tunnel_without_standby_is_sent_to_standby()
 {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;

    let outcome = harness.engine.standby_tunnel(harness.id("alpha"));

    assert!(matches!(outcome, Err(EngineError::TunnelNotAwake(_))));
    assert_eq!(echo_through(harness.port("alpha"), PAYLOAD).await, PAYLOAD);
    assert_eq!(harness.state_of("alpha"), TunnelState::Active);
}

#[tokio::test]
async fn should_fail_with_unknown_tunnel_when_an_unknown_tunnel_is_sent_to_standby() {
    let harness = HarnessBuilder::new(&["alpha"]).start().await;

    let outcome = harness.engine.standby_tunnel(TunnelId::generate());

    assert!(matches!(outcome, Err(EngineError::UnknownTunnel(_))));
}

#[tokio::test]
async fn should_be_awake_only_while_a_standby_tunnel_serves() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta"])
        .standby_override("alpha", true)
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    harness.start_and_wait_active("beta").await;
    let is_awake_on_standby = harness.is_awake("alpha");

    let _client = connect_and_wait_active(&mut harness, "alpha").await;

    assert!(!is_awake_on_standby);
    assert!(harness.is_awake("alpha"));
    assert!(!harness.is_awake("beta"));
}

#[tokio::test]
async fn should_send_only_the_awake_tunnels_back_to_standby_when_all_are_sent_to_standby() {
    let mut harness = HarnessBuilder::new(&["alpha", "beta", "gamma"])
        .standby_override("alpha", true)
        .standby_override("beta", true)
        .standby_override("gamma", false)
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.start_tunnel("beta");
    harness.wait_standby("alpha").await;
    harness.wait_standby("beta").await;
    harness.start_and_wait_active("gamma").await;
    let _client = connect_and_wait_active(&mut harness, "alpha").await;

    let sent = harness.engine.standby_all().unwrap();

    assert_eq!(sent, vec![harness.id("alpha")]);
    harness.wait_standby("alpha").await;
    assert_eq!(
        harness.state_of("beta"),
        TunnelState::Standby { last_failure: None }
    );
    assert_eq!(harness.state_of("gamma"), TunnelState::Active);
}

#[tokio::test]
async fn should_send_only_the_awake_tunnels_of_the_tag_back_to_standby() {
    let mut harness =
        HarnessBuilder::tagged(&[("alpha", &["dev"]), ("beta", &["dev"]), ("gamma", &["web"])])
            .standby_by_default()
            .start()
            .await;
    harness.engine.start_tag("dev").unwrap();
    harness.engine.start_tag("web").unwrap();
    harness.wait_standby("alpha").await;
    harness.wait_standby("beta").await;
    harness.wait_standby("gamma").await;
    let _alpha_client = connect_and_wait_active(&mut harness, "alpha").await;
    let _gamma_client = connect_and_wait_active(&mut harness, "gamma").await;

    let sent = harness.engine.standby_tag("dev").unwrap();

    assert_eq!(sent, vec![harness.id("alpha")]);
    harness.wait_standby("alpha").await;
    assert!(harness.is_awake("gamma"));
    assert!(!harness.is_awake("beta"));
}

#[tokio::test]
async fn should_fail_with_unknown_tag_when_no_tunnel_carries_the_tag_sent_to_standby() {
    let harness = HarnessBuilder::tagged(&[("alpha", &["dev"])]).start().await;

    let outcome = harness.engine.standby_tag("missing");

    assert!(matches!(outcome, Err(EngineError::UnknownTag(_))));
}

#[tokio::test]
async fn should_stay_active_when_a_connection_stays_open_longer_than_the_idle_delay() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .standby_idle(SHORT_STANDBY_IDLE)
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    let mut client = TcpStream::connect((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();
    harness.wait_active("alpha").await;

    sleep(SHORT_STANDBY_IDLE * 3).await;

    client.write_all(PAYLOAD).await.unwrap();
    let mut received = vec![0_u8; PAYLOAD.len()];
    timeout(WAIT, client.read_exact(&mut received))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(harness.state_of("alpha"), TunnelState::Active);
}

#[tokio::test]
async fn should_close_held_connections_and_keep_the_reason_when_the_wake_up_fails() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .with_unauthorized_client_key()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;

    assert!(is_closed_by_the_tunnel(harness.port("alpha")).await);

    let state = harness
        .wait_for("alpha", |state| {
            matches!(
                state,
                TunnelState::Standby {
                    last_failure: Some(_)
                }
            )
        })
        .await;
    assert!(matches!(
        state,
        TunnelState::Standby {
            last_failure: Some(FailureReason::Connect { .. })
        }
    ));
}

#[tokio::test]
async fn should_publish_a_wake_failed_event_when_the_wake_up_fails() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .with_unauthorized_client_key()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    let id = harness.id("alpha");

    is_closed_by_the_tunnel(harness.port("alpha")).await;

    let reason = wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelWakeFailed { id: failed, reason } if *failed == id => {
            Some(reason.clone())
        }
        _ => None,
    })
    .await;
    assert!(matches!(reason, FailureReason::Connect { .. }));
}

#[tokio::test]
async fn should_not_retry_on_its_own_when_the_wake_up_fails() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .with_unauthorized_client_key()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    is_closed_by_the_tunnel(harness.port("alpha")).await;
    harness
        .wait_for("alpha", |state| {
            matches!(
                state,
                TunnelState::Standby {
                    last_failure: Some(_)
                }
            )
        })
        .await;
    let attempts = harness.server.connection_count();

    sleep(QUIET_PERIOD).await;

    assert_eq!(harness.server.connection_count(), attempts);
}

#[tokio::test]
async fn should_try_again_when_a_client_connects_after_a_failed_wake_up() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .with_unauthorized_client_key()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    is_closed_by_the_tunnel(harness.port("alpha")).await;
    harness
        .wait_for("alpha", |state| {
            matches!(
                state,
                TunnelState::Standby {
                    last_failure: Some(_)
                }
            )
        })
        .await;
    let attempts = harness.server.connection_count();

    is_closed_by_the_tunnel(harness.port("alpha")).await;

    wait_until(WAIT, "a second connection attempt", || {
        harness.server.connection_count() > attempts
    })
    .await;
}

#[tokio::test]
async fn should_go_straight_back_to_standby_when_the_session_is_lost() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;
    let _client = TcpStream::connect((LOOPBACK, harness.port("alpha")))
        .await
        .unwrap();
    harness.wait_active("alpha").await;
    let id = harness.id("alpha");

    harness.server.cut_connections();

    let mut was_reconnecting = false;
    wait_for_event(&mut harness.events, WAIT, |event| match event {
        EngineEvent::TunnelChanged {
            id: changed, state, ..
        } if *changed == id => {
            was_reconnecting |= matches!(state, TunnelState::Reconnecting { .. });
            matches!(state, TunnelState::Standby { last_failure: None }).then_some(())
        }
        _ => None,
    })
    .await;
    assert!(!was_reconnecting);
}

#[tokio::test]
async fn should_ignore_standby_when_the_tunnel_is_remote() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .kind("remote")
        .standby_by_default()
        .start()
        .await;

    harness.start_and_wait_active("alpha").await;

    assert_eq!(harness.server.remote_forward_requests().len(), 1);
}

#[tokio::test]
async fn should_ignore_standby_when_the_tunnel_is_temporary() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;

    let (_id, _port) = start_active_temporary(&mut harness).await;

    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_keep_running_without_restarting_when_the_standby_choice_changes() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.set_standby_override("alpha", Some(true));

    harness.rewrite_store();
    harness.wait_for_config_reload().await;

    sleep(QUIET_PERIOD).await;
    assert_eq!(harness.state_of("alpha"), TunnelState::Active);
    assert_eq!(harness.server.connection_count(), 1);
}

#[tokio::test]
async fn should_start_in_standby_when_the_standby_choice_changed_while_the_tunnel_ran() {
    let mut harness = HarnessBuilder::new(&["alpha"]).start().await;
    harness.start_and_wait_active("alpha").await;
    harness.set_standby_override("alpha", Some(true));
    harness.rewrite_store();
    harness.wait_for_config_reload().await;

    harness.stop_tunnel("alpha");
    harness.wait_stopped("alpha").await;
    harness.start_tunnel("alpha");

    harness.wait_standby("alpha").await;
}

#[tokio::test]
async fn should_refuse_to_check_the_target_when_the_tunnel_is_on_standby() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;

    let outcome = harness.engine.check_target(harness.id("alpha")).await;

    assert!(matches!(outcome, Err(EngineError::TunnelOnStandby(_))));
}

#[tokio::test]
async fn should_restore_a_standby_tunnel_in_standby_when_restoring_on_launch() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .previously_running(&["alpha"])
        .start()
        .await;
    configure_launch(&harness, true, &[]).await;

    harness.engine.start_launch_tunnels().unwrap();

    harness.wait_standby("alpha").await;
    assert_eq!(harness.server.connection_count(), 0);
}

#[tokio::test]
async fn should_start_a_standby_tunnel_in_standby_when_its_tag_is_started() {
    let mut harness = HarnessBuilder::tagged(&[("alpha", &["dev"])])
        .standby_by_default()
        .start()
        .await;

    harness.engine.start_tag("dev").unwrap();

    harness.wait_standby("alpha").await;
}

#[tokio::test]
async fn should_stop_a_standby_tunnel_and_free_its_port_when_it_is_stopped() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;

    harness.stop_tunnel("alpha");

    harness.wait_stopped("alpha").await;
    assert!(can_bind(harness.port("alpha")).await);
}

#[tokio::test]
async fn should_stay_in_standby_when_the_scanner_sees_the_daemon_on_the_port() {
    let mut harness = HarnessBuilder::new(&["alpha"])
        .standby_by_default()
        .start()
        .await;
    harness.start_tunnel("alpha");
    harness.wait_standby("alpha").await;

    harness.set_scan(foreign_scan(
        std::process::id(),
        "tunneler",
        harness.port("alpha"),
    ));

    sleep(QUIET_PERIOD).await;
    assert_eq!(
        harness.state_of("alpha"),
        TunnelState::Standby { last_failure: None }
    );
}
