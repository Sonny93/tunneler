use std::path::{Path, PathBuf};

use engine::{AgentEndpoint, AgentLocator};
use russh::keys::PrivateKey;
use russh::keys::agent::client::AgentClient;
use russh::keys::agent::server::serve;
use ssh_config::AgentSocket;
use tokio::net::UnixListener;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnixListenerStream;

/// An SSH agent on a Unix socket, served in-process by russh.
pub struct TestAgent {
    pub socket_path: PathBuf,
    task: JoinHandle<()>,
}

impl TestAgent {
    pub fn start(directory: &Path) -> Self {
        let socket_path = directory.join("agent.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let task = tokio::spawn(async move {
            let _ = serve(UnixListenerStream::new(listener), ()).await;
        });
        Self { socket_path, task }
    }

    pub async fn add_key(&self, key: &PrivateKey) {
        let mut client = AgentClient::connect_uds(&self.socket_path).await.unwrap();
        client.add_identity(key, &[]).await.unwrap();
    }
}

impl Drop for TestAgent {
    fn drop(&mut self) {
        self.task.abort();
    }
}

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
