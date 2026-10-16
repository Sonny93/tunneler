use russh::keys::agent::client::{AgentClient, AgentStream};

use super::locator::AgentEndpoint;

/// An agent client whose transport (socket, pipe, Pageant) is erased.
pub(crate) type AgentClientConnection = AgentClient<Box<dyn AgentStream + Send + Unpin>>;

#[cfg(unix)]
pub(crate) async fn connect_agent(
    endpoint: &AgentEndpoint,
) -> Result<AgentClientConnection, String> {
    match endpoint {
        AgentEndpoint::UnixSocket(path) => AgentClient::connect_uds(path)
            .await
            .map(AgentClient::dynamic)
            .map_err(|error| error.to_string()),
        AgentEndpoint::NamedPipe(_) | AgentEndpoint::Pageant => {
            Err("this endpoint only exists on Windows".to_owned())
        }
    }
}

#[cfg(windows)]
pub(crate) async fn connect_agent(
    endpoint: &AgentEndpoint,
) -> Result<AgentClientConnection, String> {
    match endpoint {
        AgentEndpoint::NamedPipe(path) => AgentClient::connect_named_pipe(path)
            .await
            .map(AgentClient::dynamic)
            .map_err(|error| error.to_string()),
        AgentEndpoint::Pageant => AgentClient::connect_pageant()
            .await
            .map(AgentClient::dynamic)
            .map_err(|error| error.to_string()),
        AgentEndpoint::UnixSocket(_) => Err("Unix sockets are not supported on Windows".to_owned()),
    }
}
