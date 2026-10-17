use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use known_hosts::{HostLookup, KnownHosts};
use ssh_config::{JumpHost, ResolvedConfig};
use tokio::sync::watch;
use tracing::{debug, warn};

use crate::agent::AgentLocator;
use crate::authentication::{
    AgentIdentityOptions, AgentIdentitySource, AuthenticationContext, FileIdentityOptions,
    FileIdentitySource, IdentitySource, OfferedKeys, authenticate,
};
use crate::blocking::run_blocking;
use crate::client_config::{KeepaliveDefaults, build_client_config};
use crate::connection_chain::ConnectionChain;
use crate::error::ConnectError;
use crate::handshake::perform_handshake;
use crate::host_config::HostConfigResolver;
use crate::host_key::{
    AcceptedOnceStore, ClientHandler, HandshakeState, HostKeyChecker, HostKeyPolicy,
    HostKeyVerifier,
};
use crate::prompter::CredentialPrompter;
use crate::remote_forwards::RemoteForwardRegistry;
use crate::session::SshSession;
use crate::store::Settings;
use crate::transport::{BoxedTransport, open_transport};

type ConnectFuture<'a> =
    Pin<Box<dyn Future<Output = Result<SshSession, ConnectError>> + Send + 'a>>;

/// The collaborators a [`Connector`] works with.
pub struct ConnectorDependencies {
    /// Where host aliases are resolved.
    pub resolver: Arc<dyn HostConfigResolver>,
    /// Where secrets are asked.
    pub prompter: Arc<dyn CredentialPrompter>,
    /// Where unknown host keys are decided.
    pub verifier: Arc<dyn HostKeyVerifier>,
    /// Where the SSH agent is found.
    pub agent_locator: Arc<dyn AgentLocator>,
}

/// Opens authenticated SSH sessions.
///
/// Host keys accepted once are remembered for the lifetime of the connector,
/// so a reconnection does not ask again.
pub struct Connector {
    dependencies: ConnectorDependencies,
    accepted_once: Arc<AcceptedOnceStore>,
    settings: watch::Receiver<Settings>,
}

impl Connector {
    /// A connector using these collaborators and the default settings.
    pub fn new(dependencies: ConnectorDependencies) -> Self {
        let (_, settings) = watch::channel(Settings::default());
        Self::with_settings(dependencies, settings)
    }

    /// A connector that takes the keepalive defaults, for hosts whose
    /// `ssh_config` sets none, from `settings`. The settings in force when a
    /// session is opened apply to that session only.
    pub fn with_settings(
        dependencies: ConnectorDependencies,
        settings: watch::Receiver<Settings>,
    ) -> Self {
        Self {
            dependencies,
            accepted_once: Arc::new(AcceptedOnceStore::default()),
            settings,
        }
    }

    /// Opens a session to the host `alias` and authenticates it.
    ///
    /// The host is reached directly, through its `ProxyCommand`, or through
    /// its `ProxyJump` hosts, each of which is fully verified and
    /// authenticated. The returned session owns its jump sessions.
    pub async fn connect(&self, alias: &str) -> Result<SshSession, ConnectError> {
        let resolved = self.dependencies.resolver.resolve(alias)?;
        self.connect_resolved(alias.to_owned(), resolved, ConnectionChain::default())
            .await
    }

    fn connect_resolved(
        &self,
        alias: String,
        resolved: ResolvedConfig,
        chain: ConnectionChain,
    ) -> ConnectFuture<'_> {
        Box::pin(async move {
            let chain = chain.enter(&alias)?;
            if resolved.proxy_jump.is_empty() {
                let transport = open_transport(&resolved).await?;
                return self.establish(alias, resolved, transport, Vec::new()).await;
            }
            self.connect_through_jumps(alias, resolved, &chain).await
        })
    }

    async fn connect_through_jumps(
        &self,
        alias: String,
        resolved: ResolvedConfig,
        chain: &ConnectionChain,
    ) -> Result<SshSession, ConnectError> {
        let mut jumps: Vec<SshSession> = Vec::new();
        for hop in &resolved.proxy_jump {
            let session = self
                .open_hop(hop, jumps.last(), chain)
                .await
                .map_err(|source| ConnectError::Jump {
                    hop: hop.host.clone(),
                    source: Box::new(source),
                })?;
            jumps.push(session);
        }
        let transport = self.transport_over(jumps.last(), &resolved).await?;
        self.establish(alias, resolved, transport, jumps).await
    }

    async fn open_hop(
        &self,
        hop: &JumpHost,
        previous: Option<&SshSession>,
        chain: &ConnectionChain,
    ) -> Result<SshSession, ConnectError> {
        let resolved = apply_hop_overrides(self.dependencies.resolver.resolve(&hop.host)?, hop);
        let Some(previous) = previous else {
            return self
                .connect_resolved(hop.host.clone(), resolved, chain.clone())
                .await;
        };
        let transport = self.transport_over(Some(previous), &resolved).await?;
        self.establish(hop.host.clone(), resolved, transport, Vec::new())
            .await
    }

    async fn transport_over(
        &self,
        previous: Option<&SshSession>,
        resolved: &ResolvedConfig,
    ) -> Result<BoxedTransport, ConnectError> {
        let Some(previous) = previous else {
            return Ok(open_transport(resolved).await?);
        };
        let channel = previous
            .open_direct_tcpip(&resolved.hostname, resolved.port, None)
            .await?;
        Ok(Box::new(channel))
    }

    async fn establish(
        &self,
        alias: String,
        resolved: ResolvedConfig,
        transport: BoxedTransport,
        jumps: Vec<SshSession>,
    ) -> Result<SshSession, ConnectError> {
        let lookup = host_lookup(&resolved);
        let known_hosts = load_known_hosts(&resolved).await?;
        let client_config = Arc::new(build_client_config(
            &resolved,
            &known_hosts.known_algorithms(&lookup),
            KeepaliveDefaults::from(&*self.settings.borrow()),
        ));
        let state = Arc::new(HandshakeState::default());
        let checker = HostKeyChecker {
            alias: alias.clone(),
            lookup,
            known_hosts,
            policy: HostKeyPolicy::from_config(&resolved),
            verifier: Arc::clone(&self.dependencies.verifier),
            accepted_once: Arc::clone(&self.accepted_once),
            state: Arc::clone(&state),
        };
        let (closed_signal, closed) = watch::channel(false);
        let remote_forwards = Arc::new(RemoteForwardRegistry::default());
        let mut handle = perform_handshake(
            client_config,
            transport,
            ClientHandler::new(checker, closed_signal, Arc::clone(&remote_forwards)),
            &state,
            resolved.connect_timeout,
        )
        .await?;
        let context =
            AuthenticationContext::from_config(&resolved, Arc::clone(&self.dependencies.prompter));
        authenticate(&mut handle, &context, self.identity_sources(&resolved)).await?;
        debug!(alias = %alias, "authenticated");
        Ok(SshSession::new(
            alias,
            resolved,
            handle,
            closed,
            remote_forwards,
            jumps,
        ))
    }

    fn identity_sources(&self, resolved: &ResolvedConfig) -> Vec<Box<dyn IdentitySource>> {
        let offered = Arc::new(OfferedKeys::default());
        let agent = AgentIdentitySource::new(AgentIdentityOptions {
            alias: resolved.alias.clone(),
            locator: Arc::clone(&self.dependencies.agent_locator),
            identity_agent: resolved.identity_agent.clone(),
            identities_only: resolved.identities_only,
            identity_files: resolved.identity_files.clone(),
            offered: Arc::clone(&offered),
        });
        let files = FileIdentitySource::new(FileIdentityOptions {
            alias: resolved.alias.clone(),
            key_paths: resolved.identity_files.clone(),
            certificate_paths: resolved.certificate_files.clone(),
            prompter: Arc::clone(&self.dependencies.prompter),
            is_prompting_allowed: !resolved.batch_mode,
            passphrase_attempts: resolved.number_of_password_prompts,
            offered,
        });
        vec![Box::new(agent), Box::new(files)]
    }
}

fn apply_hop_overrides(mut resolved: ResolvedConfig, hop: &JumpHost) -> ResolvedConfig {
    if let Some(user) = &hop.user {
        resolved.user.clone_from(user);
    }
    if let Some(port) = hop.port {
        resolved.port = port;
    }
    resolved
}

fn host_lookup(resolved: &ResolvedConfig) -> HostLookup {
    HostLookup {
        hostname: resolved.hostname.clone(),
        port: resolved.port,
        host_key_alias: resolved.host_key_alias.clone(),
    }
}

async fn load_known_hosts(resolved: &ResolvedConfig) -> Result<KnownHosts, ConnectError> {
    let user_files = resolved.user_known_hosts_files.clone();
    let global_files = resolved.global_known_hosts_files.clone();
    let loaded = run_blocking(move || KnownHosts::load(&user_files, &global_files)).await?;
    for warning in &loaded.warnings {
        warn!(%warning, "known_hosts warning");
    }
    Ok(loaded.known_hosts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<Value: Send>(_value: &Value) {}

    #[test]
    fn should_produce_a_send_future_when_connecting() {
        fn check(connector: &Connector) {
            assert_send(&connector.connect("alias"));
        }
        let _ = check;
    }
}
