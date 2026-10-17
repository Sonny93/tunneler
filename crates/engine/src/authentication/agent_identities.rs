use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use russh::keys::PublicKey;
use russh::keys::agent::AgentIdentity;
use ssh_config::IdentityAgent;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::debug;

use super::identity::{Identity, IdentitySource};
use super::identity_files::identity_file_public_key;
use super::offered_keys::OfferedKeys;
use crate::agent::{AgentClientConnection, AgentEndpoint, AgentLocator, connect_agent};
use crate::blocking::run_blocking;
use crate::bounded::DeadlinePause;
use crate::error::ConnectError;

pub(crate) struct AgentIdentityOptions {
    pub(crate) alias: String,
    pub(crate) locator: Arc<dyn AgentLocator>,
    pub(crate) identity_agent: Option<IdentityAgent>,
    pub(crate) identities_only: bool,
    pub(crate) identity_files: Vec<PathBuf>,
    pub(crate) offered: Arc<OfferedKeys>,
    /// How long the agent may take to answer a request.
    pub(crate) response_timeout: Duration,
    /// The authentication clock, paused while the agent answers: the response
    /// timeout bounds it instead.
    pub(crate) authentication_pause: Arc<DeadlinePause>,
}

enum AgentFailure {
    Unusable(String),
    Unresponsive,
}

enum AgentState {
    NotStarted,
    Ready {
        client: Arc<Mutex<AgentClientConnection>>,
        keys: VecDeque<PublicKey>,
    },
    Unavailable,
}

/// The keys of the SSH agent. The agent is contacted when the first identity
/// is requested; an unreachable agent simply has no identities.
pub(crate) struct AgentIdentitySource {
    options: AgentIdentityOptions,
    state: AgentState,
}

impl AgentIdentitySource {
    pub(crate) fn new(options: AgentIdentityOptions) -> Self {
        Self {
            options,
            state: AgentState::NotStarted,
        }
    }

    async fn start(&self) -> Result<AgentState, ConnectError> {
        let socket = match &self.options.identity_agent {
            Some(IdentityAgent::Disabled) => return Ok(AgentState::Unavailable),
            Some(IdentityAgent::Socket(socket)) => Some(socket),
            None => None,
        };
        for endpoint in self.options.locator.locate(socket) {
            match self.open(&endpoint).await {
                Ok(state) => return Ok(state),
                Err(AgentFailure::Unresponsive) => {
                    return Err(ConnectError::AgentUnresponsive {
                        timeout: self.options.response_timeout,
                    });
                }
                Err(AgentFailure::Unusable(reason)) => {
                    debug!(alias = %self.options.alias, ?endpoint, %reason, "agent not usable");
                }
            }
        }
        Ok(AgentState::Unavailable)
    }

    async fn open(&self, endpoint: &AgentEndpoint) -> Result<AgentState, AgentFailure> {
        let (client, identities) = self.contact(endpoint).await?;
        let keys = self.usable_keys(identities).await;
        for key in &keys {
            self.options.offered.insert(key);
        }
        Ok(AgentState::Ready {
            client: Arc::new(Mutex::new(client)),
            keys: keys.into(),
        })
    }

    async fn contact(
        &self,
        endpoint: &AgentEndpoint,
    ) -> Result<(AgentClientConnection, Vec<AgentIdentity>), AgentFailure> {
        let _pause = self.options.authentication_pause.pause();
        let contacting = async {
            let mut client = connect_agent(endpoint).await?;
            let identities = client
                .request_identities()
                .await
                .map_err(|error| error.to_string())?;
            Ok::<_, String>((client, identities))
        };
        match timeout(self.options.response_timeout, contacting).await {
            Ok(Ok(contacted)) => Ok(contacted),
            Ok(Err(reason)) => Err(AgentFailure::Unusable(reason)),
            Err(_) => Err(AgentFailure::Unresponsive),
        }
    }

    async fn usable_keys(&self, identities: Vec<AgentIdentity>) -> Vec<PublicKey> {
        let keys: Vec<PublicKey> = identities
            .into_iter()
            .filter_map(|identity| match identity {
                AgentIdentity::PublicKey { key, .. } => Some(key),
                AgentIdentity::Certificate { .. } => None,
            })
            .collect();
        if !self.options.identities_only {
            return keys;
        }
        let configured = self.configured_public_keys().await;
        keys.into_iter()
            .filter(|key| {
                configured
                    .iter()
                    .any(|configured_key| configured_key.key_data() == key.key_data())
            })
            .collect()
    }

    async fn configured_public_keys(&self) -> Vec<PublicKey> {
        let paths = self.options.identity_files.clone();
        run_blocking(move || {
            paths
                .iter()
                .filter_map(|path| identity_file_public_key(path))
                .collect()
        })
        .await
    }
}

#[async_trait]
impl IdentitySource for AgentIdentitySource {
    async fn next_identity(&mut self) -> Result<Option<Identity>, ConnectError> {
        if matches!(self.state, AgentState::NotStarted) {
            self.state = self.start().await?;
        }
        let AgentState::Ready { client, keys } = &mut self.state else {
            return Ok(None);
        };
        Ok(keys.pop_front().map(|public_key| Identity::Agent {
            public_key,
            client: Arc::clone(client),
        }))
    }
}
