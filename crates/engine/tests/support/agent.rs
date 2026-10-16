use std::path::PathBuf;

use engine::{AgentEndpoint, AgentLocator};
use ssh_config::AgentSocket;

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
