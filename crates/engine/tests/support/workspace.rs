use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use engine::{
    AgentEndpoint, AgentLocator, Connector, ConnectorDependencies, CredentialPrompter,
    HostKeyVerifier, SshConfigResolver,
};
use ssh_config::{AgentSocket, ConfigSources};

use super::environment::FakeEnvironment;
use super::scripted::{ScriptedPrompter, ScriptedVerifier};

/// No agent anywhere.
pub struct NoAgentLocator;

impl AgentLocator for NoAgentLocator {
    fn locate(&self, _socket: Option<&AgentSocket>) -> Vec<AgentEndpoint> {
        Vec::new()
    }
}

/// The client-side workspace of `test_support`, able to build engine
/// connectors that read its `ssh_config`.
pub struct Workspace(test_support::Workspace);

impl Deref for Workspace {
    type Target = test_support::Workspace;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Workspace {
    pub fn new() -> Self {
        Self(test_support::Workspace::new())
    }

    pub fn connector(
        &self,
        config_path: &Path,
        prompter: Arc<dyn CredentialPrompter>,
        verifier: Arc<dyn HostKeyVerifier>,
    ) -> Connector {
        self.connector_with_agent(config_path, prompter, verifier, Arc::new(NoAgentLocator))
    }

    pub fn connector_with_agent(
        &self,
        config_path: &Path,
        prompter: Arc<dyn CredentialPrompter>,
        verifier: Arc<dyn HostKeyVerifier>,
        agent_locator: Arc<dyn AgentLocator>,
    ) -> Connector {
        Connector::new(self.connector_dependencies(config_path, prompter, verifier, agent_locator))
    }

    pub fn connector_dependencies(
        &self,
        config_path: &Path,
        prompter: Arc<dyn CredentialPrompter>,
        verifier: Arc<dyn HostKeyVerifier>,
        agent_locator: Arc<dyn AgentLocator>,
    ) -> ConnectorDependencies {
        let environment = FakeEnvironment {
            home: self.path().join("home"),
        };
        let resolver = SshConfigResolver::new(ConfigSources::user_only(config_path), environment);
        ConnectorDependencies {
            resolver: Arc::new(resolver),
            prompter,
            verifier,
            agent_locator,
        }
    }
}

pub fn quiet_prompter() -> Arc<ScriptedPrompter> {
    Arc::new(ScriptedPrompter::default())
}

pub fn rejecting_verifier() -> Arc<ScriptedVerifier> {
    Arc::new(ScriptedVerifier::default())
}
