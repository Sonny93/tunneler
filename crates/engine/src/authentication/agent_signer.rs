use std::time::Duration;

use russh::keys::HashAlg;
use russh::keys::agent::AgentIdentity;
use russh::{AgentAuthError, Error as RusshError, Signer};
use tokio::time::timeout;

use crate::agent::AgentClientConnection;
use crate::bounded::DeadlinePause;
use crate::error::ConnectError;

/// Why the agent could not sign.
#[derive(Debug)]
pub(crate) enum AgentSignError {
    /// The agent did not answer in time.
    Unresponsive,
    /// The agent or the SSH connection failed.
    Failed(AgentAuthError),
}

impl From<russh::SendError> for AgentSignError {
    fn from(error: russh::SendError) -> Self {
        Self::Failed(AgentAuthError::Send(error))
    }
}

impl AgentSignError {
    pub(crate) fn into_connect_error(self, response_timeout: Duration) -> ConnectError {
        match self {
            Self::Unresponsive => ConnectError::AgentUnresponsive {
                timeout: response_timeout,
            },
            Self::Failed(AgentAuthError::Send(_)) => ConnectError::Protocol(RusshError::SendError),
            Self::Failed(AgentAuthError::Key(error)) => ConnectError::Agent(Box::new(error)),
        }
    }
}

/// Signs through the SSH agent, giving up when it does not answer.
///
/// The agent may wait for the user to confirm, so the authentication
/// deadline is paused while it signs: this bound replaces it.
pub(crate) struct BoundedAgentSigner<'agent> {
    agent: &'agent mut AgentClientConnection,
    response_timeout: Duration,
    authentication_pause: &'agent DeadlinePause,
}

impl<'agent> BoundedAgentSigner<'agent> {
    pub(crate) fn new(
        agent: &'agent mut AgentClientConnection,
        response_timeout: Duration,
        authentication_pause: &'agent DeadlinePause,
    ) -> Self {
        Self {
            agent,
            response_timeout,
            authentication_pause,
        }
    }
}

impl Signer for BoundedAgentSigner<'_> {
    type Error = AgentSignError;

    #[allow(clippy::manual_async_fn)]
    fn auth_sign(
        &mut self,
        key: &AgentIdentity,
        hash_alg: Option<HashAlg>,
        to_sign: Vec<u8>,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> + Send {
        async move {
            let _pause = self.authentication_pause.pause();
            let signing = self.agent.auth_sign(key, hash_alg, to_sign);
            timeout(self.response_timeout, signing)
                .await
                .map_err(|_| AgentSignError::Unresponsive)?
                .map_err(AgentSignError::Failed)
        }
    }
}
