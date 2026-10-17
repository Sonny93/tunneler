use std::collections::HashSet;
use std::sync::Arc;

use russh::MethodSet;
use russh::client::{AuthResult, Handle};
use ssh_config::ResolvedConfig;
use tracing::debug;

use super::identity::IdentitySource;
use super::method::{AuthenticationMethod, MethodPolicy};
use super::outcome::MethodOutcome;
use super::pausing_prompter::PausingPrompter;
use super::{keyboard_interactive, password, publickey};
use crate::bounded::{DeadlinePause, bounded};
use crate::connect_timeouts::ConnectTimeouts;
use crate::error::ConnectError;
use crate::host_key::ClientHandler;
use crate::prompter::CredentialPrompter;

pub(crate) struct AuthenticationContext {
    pub(crate) alias: String,
    pub(crate) user: String,
    pub(crate) host: String,
    /// Asks the user, pausing [`Self::deadline_pause`] meanwhile.
    pub(crate) prompter: Arc<dyn CredentialPrompter>,
    pub(crate) number_of_password_prompts: u32,
    pub(crate) planned_methods: Vec<AuthenticationMethod>,
    pub(crate) timeouts: ConnectTimeouts,
    /// The clock of the authentication phase; it does not run while the user
    /// is asked something or the SSH agent signs.
    pub(crate) deadline_pause: Arc<DeadlinePause>,
}

impl AuthenticationContext {
    pub(crate) fn from_config(
        config: &ResolvedConfig,
        prompter: Arc<dyn CredentialPrompter>,
        timeouts: ConnectTimeouts,
    ) -> Self {
        let deadline_pause = Arc::new(DeadlinePause::default());
        Self {
            alias: config.alias.clone(),
            user: config.user.clone(),
            host: config.hostname.clone(),
            prompter: Arc::new(PausingPrompter::new(prompter, Arc::clone(&deadline_pause))),
            number_of_password_prompts: config.number_of_password_prompts,
            planned_methods: MethodPolicy::from_config(config).planned_methods(),
            timeouts,
            deadline_pause,
        }
    }
}

/// Authenticates the session, giving up when the server stays silent for
/// longer than the authentication step timeout.
pub(crate) async fn authenticate(
    handle: &mut Handle<ClientHandler>,
    context: &AuthenticationContext,
    identity_sources: Vec<Box<dyn IdentitySource>>,
) -> Result<(), ConnectError> {
    let limit = context.timeouts.authentication_step;
    bounded(
        try_methods(handle, context, identity_sources),
        limit,
        &context.deadline_pause,
    )
    .await
    .map_err(|_| ConnectError::AuthenticationTimeout { timeout: limit })?
}

async fn try_methods(
    handle: &mut Handle<ClientHandler>,
    context: &AuthenticationContext,
    mut identity_sources: Vec<Box<dyn IdentitySource>>,
) -> Result<(), ConnectError> {
    let mut remaining_methods = match handle.authenticate_none(context.user.as_str()).await? {
        AuthResult::Success => return Ok(()),
        AuthResult::Failure {
            remaining_methods, ..
        } => remaining_methods,
    };
    let mut tried = Vec::new();
    let mut finished = HashSet::new();
    while let Some(method) = next_method(context, &remaining_methods, &finished) {
        finished.insert(method);
        let outcome = attempt(handle, context, method, &mut identity_sources).await?;
        match outcome {
            MethodOutcome::Authenticated => return Ok(()),
            MethodOutcome::NotAttempted => debug!(%method, "authentication method skipped"),
            MethodOutcome::Rejected {
                remaining_methods: server_methods,
                partial_success,
            } => {
                debug!(%method, partial_success, "authentication method rejected");
                tried.push(method);
                remaining_methods = server_methods;
            }
        }
    }
    Err(ConnectError::Authentication { tried })
}

fn next_method(
    context: &AuthenticationContext,
    remaining_methods: &MethodSet,
    finished: &HashSet<AuthenticationMethod>,
) -> Option<AuthenticationMethod> {
    context
        .planned_methods
        .iter()
        .copied()
        .find(|method| !finished.contains(method) && remaining_methods.contains(&method.kind()))
}

async fn attempt(
    handle: &mut Handle<ClientHandler>,
    context: &AuthenticationContext,
    method: AuthenticationMethod,
    identity_sources: &mut [Box<dyn IdentitySource>],
) -> Result<MethodOutcome, ConnectError> {
    match method {
        AuthenticationMethod::PublicKey => {
            publickey::attempt(handle, context, identity_sources).await
        }
        AuthenticationMethod::KeyboardInteractive => {
            keyboard_interactive::attempt(handle, context).await
        }
        AuthenticationMethod::Password => password::attempt(handle, context).await,
    }
}
