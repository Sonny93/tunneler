use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use russh::keys::{PrivateKey, PublicKey};
use russh::server::{self, Auth, Config, Msg, Server, Session};
use russh::{Channel, ChannelOpenFailure, MethodKind, MethodSet};
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

const REJECTION_DELAY: Duration = Duration::from_millis(1);
const MAXIMUM_AUTH_ATTEMPTS: usize = 100;

/// What one account of the test server accepts.
#[derive(Clone, Default)]
pub struct TestUser {
    pub password: Option<String>,
    pub authorized_keys: Vec<PublicKey>,
    pub keyboard_interactive: Vec<(String, String)>,
}

struct Behavior {
    offered_methods: MethodSet,
    users: HashMap<String, TestUser>,
    requires_publickey_then_password: bool,
    allows_direct_tcpip: bool,
    connection_count: Arc<AtomicUsize>,
}

pub struct TestServerBuilder {
    host_keys: Vec<PrivateKey>,
    users: HashMap<String, TestUser>,
    methods: Option<Vec<MethodKind>>,
    requires_publickey_then_password: bool,
    allows_direct_tcpip: bool,
}

impl TestServerBuilder {
    pub fn host_key(mut self, key: PrivateKey) -> Self {
        self.host_keys.push(key);
        self
    }

    pub fn user(mut self, name: &str, user: TestUser) -> Self {
        self.users.insert(name.to_owned(), user);
        self
    }

    pub fn methods(mut self, methods: &[MethodKind]) -> Self {
        self.methods = Some(methods.to_vec());
        self
    }

    pub fn requiring_publickey_then_password(mut self) -> Self {
        self.requires_publickey_then_password = true;
        self
    }

    pub fn refusing_direct_tcpip(mut self) -> Self {
        self.allows_direct_tcpip = false;
        self
    }

    pub async fn start(self) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let connection_count = Arc::new(AtomicUsize::new(0));
        let config = self.server_config();
        let offered_methods = config.methods.clone();
        let config = Arc::new(config);
        let mut acceptor = Acceptor {
            behavior: Arc::new(Behavior {
                offered_methods,
                users: self.users,
                requires_publickey_then_password: self.requires_publickey_then_password,
                allows_direct_tcpip: self.allows_direct_tcpip,
                connection_count: Arc::clone(&connection_count),
            }),
        };
        let task = tokio::spawn(async move {
            let _ = acceptor.run_on_socket(config, &listener).await;
        });
        TestServer {
            port,
            host_keys: self.host_keys,
            connection_count,
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
    pub port: u16,
    pub host_keys: Vec<PrivateKey>,
    connection_count: Arc<AtomicUsize>,
    task: JoinHandle<()>,
}

impl TestServer {
    pub fn builder() -> TestServerBuilder {
        TestServerBuilder {
            host_keys: Vec::new(),
            users: HashMap::new(),
            methods: None,
            requires_publickey_then_password: false,
            allows_direct_tcpip: true,
        }
    }

    pub fn public_host_key(&self, index: usize) -> PublicKey {
        self.host_keys[index].public_key().clone()
    }

    pub fn connection_count(&self) -> usize {
        self.connection_count.load(Ordering::SeqCst)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct Acceptor {
    behavior: Arc<Behavior>,
}

impl Server for Acceptor {
    type Handler = ConnectionHandler;

    fn new_client(&mut self, _peer_address: Option<std::net::SocketAddr>) -> Self::Handler {
        self.behavior
            .connection_count
            .fetch_add(1, Ordering::SeqCst);
        ConnectionHandler {
            behavior: Arc::clone(&self.behavior),
            has_passed_publickey: false,
        }
    }
}

struct ConnectionHandler {
    behavior: Arc<Behavior>,
    has_passed_publickey: bool,
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
        _originator_address: &str,
        _originator_port: u32,
        reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
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
}
