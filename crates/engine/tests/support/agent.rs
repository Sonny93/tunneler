use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use engine::{AgentEndpoint, AgentLocator};
use russh::keys::PrivateKey;
use ssh_config::AgentSocket;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;

/// Finds the agent at a fixed socket, whatever the environment says.
pub struct FixedAgentLocator {
    pub default_socket: Option<PathBuf>,
}

impl AgentLocator for FixedAgentLocator {
    fn locate(&self, socket: Option<&AgentSocket>) -> Vec<AgentEndpoint> {
        match socket {
            Some(AgentSocket::Path(path)) => vec![AgentEndpoint::UnixSocket(path.clone())],
            Some(AgentSocket::Environment(_)) => Vec::new(),
            None => self
                .default_socket
                .iter()
                .cloned()
                .map(AgentEndpoint::UnixSocket)
                .collect(),
        }
    }
}

const REQUEST_IDENTITIES: u8 = 11;
const IDENTITIES_ANSWER: u8 = 12;
const FRAME_HEADER_SIZE: usize = 4;

/// An SSH agent that lists one key and then never answers a signature
/// request, like an agent waiting for a confirmation nobody gives.
pub struct UnresponsiveAgent {
    pub socket_path: PathBuf,
    accepted_connections: Arc<AtomicUsize>,
    open_connections: Arc<AtomicUsize>,
    task: JoinHandle<()>,
}

impl UnresponsiveAgent {
    /// Starts an agent listening on `unresponsive-agent.sock` inside
    /// `directory`, listing the public half of `key`.
    pub fn start(directory: &Path, key: &PrivateKey) -> Self {
        let socket_path = directory.join("unresponsive-agent.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let identities_answer = identities_answer(key);
        let accepted_connections = Arc::new(AtomicUsize::new(0));
        let open_connections = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn(accept_connections(
            listener,
            identities_answer,
            Arc::clone(&accepted_connections),
            Arc::clone(&open_connections),
        ));
        Self {
            socket_path,
            accepted_connections,
            open_connections,
            task,
        }
    }

    /// How many clients ever connected.
    pub fn accepted_connection_count(&self) -> usize {
        self.accepted_connections.load(Ordering::SeqCst)
    }

    /// How many clients are connected right now.
    pub fn open_connection_count(&self) -> usize {
        self.open_connections.load(Ordering::SeqCst)
    }
}

impl Drop for UnresponsiveAgent {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn identities_answer(key: &PrivateKey) -> Vec<u8> {
    let blob = key.public_key().to_bytes().unwrap();
    let mut body = vec![IDENTITIES_ANSWER];
    body.extend_from_slice(&1_u32.to_be_bytes());
    body.extend_from_slice(&u32::try_from(blob.len()).unwrap().to_be_bytes());
    body.extend_from_slice(&blob);
    body.extend_from_slice(&0_u32.to_be_bytes());
    let mut frame = u32::try_from(body.len()).unwrap().to_be_bytes().to_vec();
    frame.extend_from_slice(&body);
    frame
}

async fn accept_connections(
    listener: UnixListener,
    identities_answer: Vec<u8>,
    accepted_connections: Arc<AtomicUsize>,
    open_connections: Arc<AtomicUsize>,
) {
    while let Ok((stream, _)) = listener.accept().await {
        accepted_connections.fetch_add(1, Ordering::SeqCst);
        open_connections.fetch_add(1, Ordering::SeqCst);
        let identities_answer = identities_answer.clone();
        let open_connections = Arc::clone(&open_connections);
        tokio::spawn(async move {
            serve_until_closed(stream, &identities_answer).await;
            open_connections.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

async fn serve_until_closed(mut stream: UnixStream, identities_answer: &[u8]) {
    loop {
        let mut header = [0_u8; FRAME_HEADER_SIZE];
        if stream.read_exact(&mut header).await.is_err() {
            return;
        }
        let mut body = vec![0_u8; u32::from_be_bytes(header) as usize];
        if stream.read_exact(&mut body).await.is_err() {
            return;
        }
        if body.first() == Some(&REQUEST_IDENTITIES)
            && stream.write_all(identities_answer).await.is_err()
        {
            return;
        }
    }
}
