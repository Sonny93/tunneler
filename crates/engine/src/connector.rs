use std::sync::Arc;

use known_hosts::{HostLookup, KnownHosts};
use ssh_config::ResolvedConfig;
use tracing::{debug, warn};

use crate::authentication::{
    AuthenticationContext, FileIdentityOptions, FileIdentitySource, IdentitySource, authenticate,
};
use crate::client_config::build_client_config;
use crate::error::ConnectError;
use crate::handshake::perform_handshake;
use crate::host_config::HostConfigResolver;
use crate::host_key::{
    AcceptedOnceStore, ClientHandler, HandshakeState, HostKeyChecker, HostKeyPolicy,
    HostKeyVerifier,
};
use crate::prompter::CredentialPrompter;
use crate::session::SshSession;
use crate::transport::open_transport;

/// The collaborators a [`Connector`] works with.
pub struct ConnectorDependencies {
    /// Where host aliases are resolved.
    pub resolver: Arc<dyn HostConfigResolver>,
    /// Where secrets are asked.
    pub prompter: Arc<dyn CredentialPrompter>,
    /// Where unknown host keys are decided.
    pub verifier: Arc<dyn HostKeyVerifier>,
}

/// Opens authenticated SSH sessions.
///
/// Host keys accepted once are remembered for the lifetime of the connector,
/// so a reconnection does not ask again.
pub struct Connector {
    dependencies: ConnectorDependencies,
    accepted_once: Arc<AcceptedOnceStore>,
}

impl Connector {
    /// A connector using these collaborators.
    pub fn new(dependencies: ConnectorDependencies) -> Self {
        Self {
            dependencies,
            accepted_once: Arc::new(AcceptedOnceStore::default()),
        }
    }

    /// Opens a session to the host `alias` and authenticates it.
    pub async fn connect(&self, alias: &str) -> Result<SshSession, ConnectError> {
        let resolved = self.dependencies.resolver.resolve(alias)?;
        let lookup = host_lookup(&resolved);
        let known_hosts = load_known_hosts(&resolved)?;
        let client_config = Arc::new(build_client_config(
            &resolved,
            &known_hosts.known_algorithms(&lookup),
        ));
        let state = Arc::new(HandshakeState::default());
        let checker = HostKeyChecker {
            alias: alias.to_owned(),
            lookup,
            known_hosts,
            policy: HostKeyPolicy::from_config(&resolved),
            verifier: Arc::clone(&self.dependencies.verifier),
            accepted_once: Arc::clone(&self.accepted_once),
            state: Arc::clone(&state),
        };
        let transport = open_transport(&resolved).await?;
        debug!(alias, "transport established");
        let mut handle = perform_handshake(
            client_config,
            transport,
            ClientHandler::new(checker),
            &state,
            resolved.connect_timeout,
        )
        .await?;
        let context =
            AuthenticationContext::from_config(&resolved, Arc::clone(&self.dependencies.prompter));
        let identity_sources = self.identity_sources(&resolved);
        authenticate(&mut handle, &context, identity_sources).await?;
        debug!(alias, "authenticated");
        Ok(SshSession::new(alias.to_owned(), resolved, handle))
    }

    fn identity_sources(&self, resolved: &ResolvedConfig) -> Vec<Box<dyn IdentitySource>> {
        let files = FileIdentitySource::new(FileIdentityOptions {
            alias: resolved.alias.clone(),
            key_paths: resolved.identity_files.clone(),
            certificate_paths: resolved.certificate_files.clone(),
            prompter: Arc::clone(&self.dependencies.prompter),
            is_prompting_allowed: !resolved.batch_mode,
            passphrase_attempts: resolved.number_of_password_prompts,
        });
        vec![Box::new(files)]
    }
}

fn host_lookup(resolved: &ResolvedConfig) -> HostLookup {
    HostLookup {
        hostname: resolved.hostname.clone(),
        port: resolved.port,
        host_key_alias: resolved.host_key_alias.clone(),
    }
}

fn load_known_hosts(resolved: &ResolvedConfig) -> Result<KnownHosts, ConnectError> {
    let loaded = KnownHosts::load(
        &resolved.user_known_hosts_files,
        &resolved.global_known_hosts_files,
    )?;
    for warning in &loaded.warnings {
        warn!(%warning, "known_hosts warning");
    }
    Ok(loaded.known_hosts)
}
