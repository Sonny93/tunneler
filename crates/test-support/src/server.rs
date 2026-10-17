use std::borrow::Cow;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::keys::{HashAlg, PrivateKey, PublicKey};
use russh::server::{self, Auth, Config, Msg, Server, Session};
use russh::{Channel, ChannelOpenFailure, MethodKind, MethodSet};
use tokio::io::{copy_bidirectional, duplex};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::{AbortHandle, JoinHandle};
use tokio::time::sleep;

const REJECTION_DELAY: Duration = Duration::from_millis(1);
const MAXIMUM_AUTH_ATTEMPTS: usize = 100;
const BRIDGE_BUFFER_SIZE: usize = 64 * 1024;
const FORWARD_BIND_ATTEMPTS: usize = 20;
const FORWARD_BIND_RETRY_DELAY: Duration = Duration::from_millis(10);
const FORWARD_BIND_ADDRESS: &str = "127.0.0.1";

/// What one account of the test server accepts.
#[derive(Clone, Default)]
pub struct TestUser {
    /// The password the account accepts.
    pub password: Option<String>,
    /// The public keys the account accepts.
    pub authorized_keys: Vec<PublicKey>,
    /// The question and expected answer of each keyboard-interactive prompt.
    pub keyboard_interactive: Vec<(String, String)>,
}

struct Behavior {
    offered_methods: MethodSet,
    users: HashMap<String, TestUser>,
    requires_publickey_then_password: bool,
    allows_direct_tcpip: bool,
    allows_remote_forwards: bool,
    connection_count: Arc<AtomicUsize>,
    remote_forward_requests: Arc<Mutex<Vec<(String, u16)>>>,
    direct_tcpip_targets: Arc<Mutex<Vec<(String, u16)>>>,
    direct_tcpip_originators: Arc<Mutex<Vec<(String, u32)>>>,
    offered_fingerprints: Arc<Mutex<Vec<String>>>,
}

/// The live connections of the server, so that they can be cut.
struct Connections {
    abort_handles: Mutex<Vec<AbortHandle>>,
    open: AtomicUsize,
    stall_generation: watch::Sender<u64>,
}

impl Default for Connections {
    fn default() -> Self {
        Self {
            abort_handles: Mutex::new(Vec::new()),
            open: AtomicUsize::new(0),
            stall_generation: watch::channel(0).0,
        }
    }
}

impl Connections {
    fn track(&self, handle: AbortHandle) {
        let mut handles = self.abort_handles.lock().unwrap();
        handles.retain(|tracked| !tracked.is_finished());
        handles.push(handle);
    }

    fn cut_all(&self) {
        for handle in self.abort_handles.lock().unwrap().drain(..) {
            handle.abort();
        }
    }
}

/// Counts a connection as open until it is dropped.
struct OpenConnection(Arc<Connections>);

impl OpenConnection {
    fn begin(connections: &Arc<Connections>) -> Self {
        connections.open.fetch_add(1, Ordering::SeqCst);
        Self(Arc::clone(connections))
    }
}

impl Drop for OpenConnection {
    fn drop(&mut self) {
        self.0.open.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Configures a [`TestServer`].
#[must_use]
pub struct TestServerBuilder {
    port: u16,
    host_keys: Vec<PrivateKey>,
    users: HashMap<String, TestUser>,
    methods: Option<Vec<MethodKind>>,
    requires_publickey_then_password: bool,
    allows_direct_tcpip: bool,
    allows_remote_forwards: bool,
}

impl TestServerBuilder {
    /// Listens on this port instead of a random one.
    pub fn on_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Adds a host key the server presents.
    pub fn host_key(mut self, key: PrivateKey) -> Self {
        self.host_keys.push(key);
        self
    }

    /// Adds an account.
    pub fn user(mut self, name: &str, user: TestUser) -> Self {
        self.users.insert(name.to_owned(), user);
        self
    }

    /// Restricts the authentication methods the server offers.
    pub fn methods(mut self, methods: &[MethodKind]) -> Self {
        self.methods = Some(methods.to_vec());
        self
    }

    /// Requires a public key and then a password.
    pub fn requiring_publickey_then_password(mut self) -> Self {
        self.requires_publickey_then_password = true;
        self
    }

    /// Refuses every `direct-tcpip` channel.
    pub fn refusing_direct_tcpip(mut self) -> Self {
        self.allows_direct_tcpip = false;
        self
    }

    /// Refuses every `tcpip-forward` request.
    pub fn refusing_remote_forwards(mut self) -> Self {
        self.allows_remote_forwards = false;
        self
    }

    /// Starts listening on loopback.
    pub async fn start(self) -> TestServer {
        let listener = TcpListener::bind(("127.0.0.1", self.port)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let connection_count = Arc::new(AtomicUsize::new(0));
        let remote_forward_requests = Arc::new(Mutex::new(Vec::new()));
        let direct_tcpip_targets = Arc::new(Mutex::new(Vec::new()));
        let direct_tcpip_originators = Arc::new(Mutex::new(Vec::new()));
        let offered_fingerprints = Arc::new(Mutex::new(Vec::new()));
        let config = self.server_config();
        let offered_methods = config.methods.clone();
        let behavior = Arc::new(Behavior {
            offered_methods,
            users: self.users,
            requires_publickey_then_password: self.requires_publickey_then_password,
            allows_direct_tcpip: self.allows_direct_tcpip,
            allows_remote_forwards: self.allows_remote_forwards,
            connection_count: Arc::clone(&connection_count),
            remote_forward_requests: Arc::clone(&remote_forward_requests),
            direct_tcpip_targets: Arc::clone(&direct_tcpip_targets),
            direct_tcpip_originators: Arc::clone(&direct_tcpip_originators),
            offered_fingerprints: Arc::clone(&offered_fingerprints),
        });
        let config = Arc::new(config);
        let connections = Arc::new(Connections::default());
        let task = spawn_acceptor(
            listener,
            Arc::clone(&behavior),
            Arc::clone(&config),
            Arc::clone(&connections),
        );
        TestServer {
            port,
            host_keys: self.host_keys,
            connection_count,
            remote_forward_requests,
            direct_tcpip_targets,
            direct_tcpip_originators,
            offered_fingerprints,
            behavior,
            config,
            connections,
            task,
        }
    }

    fn server_config(&self) -> Config {
        let methods = self.methods.clone().unwrap_or_else(|| {
            vec![
                MethodKind::PublicKey,
                MethodKind::KeyboardInteractive,
                MethodKind::Password,
            ]
        });
        Config {
            methods: MethodSet::from(methods.as_slice()),
            auth_rejection_time: REJECTION_DELAY,
            auth_rejection_time_initial: Some(REJECTION_DELAY),
            max_auth_attempts: MAXIMUM_AUTH_ATTEMPTS,
            keys: self.host_keys.clone(),
            inactivity_timeout: None,
            ..Config::default()
        }
    }
}

/// An in-process SSH server on a random loopback port.
pub struct TestServer {
    /// The loopback port it listens on.
    pub port: u16,
    /// The host keys it presents.
    pub host_keys: Vec<PrivateKey>,
    connection_count: Arc<AtomicUsize>,
    remote_forward_requests: Arc<Mutex<Vec<(String, u16)>>>,
    direct_tcpip_targets: Arc<Mutex<Vec<(String, u16)>>>,
    direct_tcpip_originators: Arc<Mutex<Vec<(String, u32)>>>,
    offered_fingerprints: Arc<Mutex<Vec<String>>>,
    behavior: Arc<Behavior>,
    config: Arc<Config>,
    connections: Arc<Connections>,
    task: JoinHandle<()>,
}

impl TestServer {
    /// A builder for a server with no host key and no account.
    pub fn builder() -> TestServerBuilder {
        TestServerBuilder {
            port: 0,
            host_keys: Vec::new(),
            users: HashMap::new(),
            methods: None,
            requires_publickey_then_password: false,
            allows_direct_tcpip: true,
            allows_remote_forwards: true,
        }
    }

    /// The public half of the host key at `index`.
    pub fn public_host_key(&self, index: usize) -> PublicKey {
        self.host_keys[index].public_key().clone()
    }

    /// The connections accepted since the server started.
    pub fn connection_count(&self) -> usize {
        self.connection_count.load(Ordering::SeqCst)
    }

    /// The `direct-tcpip` targets clients asked this server to reach.
    pub fn direct_tcpip_targets(&self) -> Vec<(String, u16)> {
        self.direct_tcpip_targets.lock().unwrap().clone()
    }

    /// The originator address and port clients reported in `direct-tcpip`
    /// requests.
    pub fn direct_tcpip_originators(&self) -> Vec<(String, u32)> {
        self.direct_tcpip_originators.lock().unwrap().clone()
    }

    /// The `tcpip-forward` requests clients made, as the address and port
    /// they asked this server to listen on, in order.
    pub fn remote_forward_requests(&self) -> Vec<(String, u16)> {
        self.remote_forward_requests.lock().unwrap().clone()
    }

    /// The connections that are open right now.
    pub fn open_connection_count(&self) -> usize {
        self.connections.open.load(Ordering::SeqCst)
    }

    /// Drops every live connection without a goodbye, like a network drop.
    /// New connections are still accepted.
    pub fn cut_connections(&self) {
        self.connections.cut_all();
    }

    /// Makes every live connection go silent without closing it: nothing
    /// more is read or written, like a network that swallows packets.
    /// Connections accepted afterwards behave normally.
    pub fn stall_connections(&self) {
        self.connections
            .stall_generation
            .send_modify(|generation| *generation += 1);
    }

    /// Stops accepting and cuts every live connection. The port is free
    /// when this returns.
    pub async fn stop(&mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
        self.connections.cut_all();
    }

    /// Listens again on the same port, with the same keys and users, after
    /// [`TestServer::stop`].
    pub async fn restart(&mut self) {
        let listener = TcpListener::bind(("127.0.0.1", self.port)).await.unwrap();
        self.task = spawn_acceptor(
            listener,
            Arc::clone(&self.behavior),
            Arc::clone(&self.config),
            Arc::clone(&self.connections),
        );
    }

    /// The SHA-256 fingerprints of the public keys clients offered.
    pub fn offered_fingerprints(&self) -> Vec<String> {
        self.offered_fingerprints.lock().unwrap().clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
        self.connections.cut_all();
    }
}

fn spawn_acceptor(
    listener: TcpListener,
    behavior: Arc<Behavior>,
    config: Arc<Config>,
    connections: Arc<Connections>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut acceptor = Acceptor { behavior };
        while let Ok((socket, peer)) = listener.accept().await {
            let handler = acceptor.new_client(Some(peer));
            let open = OpenConnection::begin(&connections);
            let task = tokio::spawn(serve_connection(socket, handler, Arc::clone(&config), open));
            connections.track(task.abort_handle());
        }
    })
}

/// Runs one SSH session over a socket, through an in-memory bridge, so that
/// aborting the task drops the socket and the client sees the connection die.
async fn serve_connection(
    mut socket: TcpStream,
    handler: ConnectionHandler,
    config: Arc<Config>,
    open: OpenConnection,
) {
    let mut stalls = open.0.stall_generation.subscribe();
    let started_at = *stalls.borrow_and_update();
    let (server_side, mut bridge_side) = duplex(BRIDGE_BUFFER_SIZE);
    let session = async {
        if let Ok(running) = server::run_stream(config, server_side, handler).await {
            let _ = running.await;
        }
    };
    tokio::select! {
        _ = copy_bidirectional(&mut socket, &mut bridge_side) => {}
        () = session => {}
        () = wait_for_stall(&mut stalls, started_at) => {
            std::future::pending::<()>().await;
        }
    }
}

struct Acceptor {
    behavior: Arc<Behavior>,
}

impl Server for Acceptor {
    type Handler = ConnectionHandler;

    fn new_client(&mut self, _peer_address: Option<SocketAddr>) -> Self::Handler {
        self.behavior
            .connection_count
            .fetch_add(1, Ordering::SeqCst);
        ConnectionHandler {
            behavior: Arc::clone(&self.behavior),
            has_passed_publickey: false,
            forwards: HashMap::new(),
        }
    }
}

struct ConnectionHandler {
    behavior: Arc<Behavior>,
    has_passed_publickey: bool,
    forwards: HashMap<u16, JoinHandle<()>>,
}

impl Drop for ConnectionHandler {
    fn drop(&mut self) {
        for task in self.forwards.values() {
            task.abort();
        }
    }
}

impl ConnectionHandler {
    fn is_authorized(&self, user: &str, key: &PublicKey) -> bool {
        self.behavior.users.get(user).is_some_and(|account| {
            account
                .authorized_keys
                .iter()
                .any(|authorized| authorized.key_data() == key.key_data())
        })
    }

    fn reject(&self) -> Auth {
        Auth::Reject {
            proceed_with_methods: Some(self.behavior.offered_methods.clone()),
            partial_success: false,
        }
    }

    fn continue_with(methods: &[MethodKind]) -> Auth {
        Auth::Reject {
            proceed_with_methods: Some(MethodSet::from(methods)),
            partial_success: true,
        }
    }
}

#[allow(
    clippy::unused_async_trait_impl,
    reason = "the russh handler trait requires async methods even when nothing awaits"
)]
impl server::Handler for ConnectionHandler {
    type Error = russh::Error;

    async fn auth_publickey_offered(
        &mut self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        self.behavior
            .offered_fingerprints
            .lock()
            .unwrap()
            .push(public_key.fingerprint(HashAlg::Sha256).to_string());
        if self.is_authorized(user, public_key) {
            return Ok(Auth::Accept);
        }
        Ok(self.reject())
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        if !self.is_authorized(user, public_key) {
            return Ok(self.reject());
        }
        if self.behavior.requires_publickey_then_password {
            self.has_passed_publickey = true;
            return Ok(Self::continue_with(&[MethodKind::Password]));
        }
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        let is_correct = self
            .behavior
            .users
            .get(user)
            .and_then(|account| account.password.as_deref())
            .is_some_and(|expected| expected == password);
        if !is_correct {
            return Ok(self.reject());
        }
        if self.behavior.requires_publickey_then_password && !self.has_passed_publickey {
            return Ok(Auth::Reject {
                proceed_with_methods: Some(MethodSet::from(&[MethodKind::PublicKey][..])),
                partial_success: false,
            });
        }
        Ok(Auth::Accept)
    }

    async fn auth_keyboard_interactive<'a>(
        &'a mut self,
        user: &str,
        _submethods: &str,
        response: Option<server::Response<'a>>,
    ) -> Result<Auth, Self::Error> {
        let Some(account) = self.behavior.users.get(user) else {
            return Ok(self.reject());
        };
        if account.keyboard_interactive.is_empty() {
            return Ok(self.reject());
        }
        let Some(response) = response else {
            let prompts: Vec<(Cow<'static, str>, bool)> = account
                .keyboard_interactive
                .iter()
                .map(|(question, _)| (Cow::Owned(question.clone()), false))
                .collect();
            return Ok(Auth::Partial {
                name: "test".into(),
                instructions: "answer the questions".into(),
                prompts: Cow::Owned(prompts),
            });
        };
        let answers: Vec<String> = response
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .collect();
        let expected: Vec<&String> = account
            .keyboard_interactive
            .iter()
            .map(|(_, a)| a)
            .collect();
        if answers.iter().eq(expected) {
            return Ok(Auth::Accept);
        }
        Ok(self.reject())
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        originator_address: &str,
        originator_port: u32,
        reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.behavior
            .direct_tcpip_originators
            .lock()
            .unwrap()
            .push((originator_address.to_owned(), originator_port));
        if let Ok(port) = u16::try_from(port_to_connect) {
            self.behavior
                .direct_tcpip_targets
                .lock()
                .unwrap()
                .push((host_to_connect.to_owned(), port));
        }
        if !self.behavior.allows_direct_tcpip {
            reply
                .reject(ChannelOpenFailure::AdministrativelyProhibited)
                .await;
            return Ok(());
        }
        let Ok(port) = u16::try_from(port_to_connect) else {
            reply.reject(ChannelOpenFailure::ConnectFailed).await;
            return Ok(());
        };
        match TcpStream::connect((host_to_connect, port)).await {
            Ok(mut target) => {
                reply.accept().await;
                tokio::spawn(async move {
                    let mut stream = channel.into_stream();
                    let _ = copy_bidirectional(&mut stream, &mut target).await;
                });
            }
            Err(_) => reply.reject(ChannelOpenFailure::ConnectFailed).await,
        }
        Ok(())
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let Ok(requested_port) = u16::try_from(*port) else {
            return Ok(false);
        };
        self.behavior
            .remote_forward_requests
            .lock()
            .unwrap()
            .push((address.to_owned(), requested_port));
        if !self.behavior.allows_remote_forwards {
            return Ok(false);
        }
        let Some(listener) = bind_forward_listener(requested_port).await else {
            return Ok(false);
        };
        let bound_port = listener.local_addr().unwrap().port();
        *port = u32::from(bound_port);
        let task = tokio::spawn(accept_forwarded(
            listener,
            session.handle(),
            address.to_owned(),
        ));
        self.forwards.insert(bound_port, task);
        Ok(true)
    }

    async fn cancel_tcpip_forward(
        &mut self,
        _address: &str,
        port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let Some(task) = u16::try_from(port)
            .ok()
            .and_then(|port| self.forwards.remove(&port))
        else {
            return Ok(false);
        };
        task.abort();
        let _ = task.await;
        Ok(true)
    }
}

async fn bind_forward_listener(port: u16) -> Option<TcpListener> {
    for _ in 0..FORWARD_BIND_ATTEMPTS {
        if let Ok(listener) = TcpListener::bind((FORWARD_BIND_ADDRESS, port)).await {
            return Some(listener);
        }
        sleep(FORWARD_BIND_RETRY_DELAY).await;
    }
    None
}

async fn accept_forwarded(listener: TcpListener, handle: server::Handle, address: String) {
    let connected_port = u32::from(listener.local_addr().unwrap().port());
    while let Ok((stream, peer)) = listener.accept().await {
        tokio::spawn(relay_forwarded(
            stream,
            handle.clone(),
            address.clone(),
            connected_port,
            peer,
        ));
    }
}

async fn relay_forwarded(
    mut stream: TcpStream,
    handle: server::Handle,
    address: String,
    connected_port: u32,
    peer: SocketAddr,
) {
    let opened = handle
        .channel_open_forwarded_tcpip(
            address,
            connected_port,
            peer.ip().to_string(),
            u32::from(peer.port()),
        )
        .await;
    let Ok(channel) = opened else {
        return;
    };
    let mut channel_stream = channel.into_stream();
    let _ = copy_bidirectional(&mut channel_stream, &mut stream).await;
}

async fn wait_for_stall(stalls: &mut watch::Receiver<u64>, started_at: u64) {
    let _ = stalls
        .wait_for(|generation| *generation != started_at)
        .await;
}
