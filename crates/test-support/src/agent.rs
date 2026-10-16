use std::path::{Path, PathBuf};

use russh::keys::PrivateKey;
use russh::keys::agent::client::AgentClient;
use russh::keys::agent::server::serve;
use tokio::net::UnixListener;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnixListenerStream;

/// An SSH agent on a Unix socket, served in-process by russh.
pub struct TestAgent {
    /// The socket the agent listens on.
    pub socket_path: PathBuf,
    task: JoinHandle<()>,
}

impl TestAgent {
    /// Starts an agent listening on `agent.sock` inside `directory`.
    pub fn start(directory: &Path) -> Self {
        let socket_path = directory.join("agent.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let task = tokio::spawn(async move {
            let _ = serve(UnixListenerStream::new(listener), ()).await;
        });
        Self { socket_path, task }
    }

    /// Adds `key` to the agent.
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
