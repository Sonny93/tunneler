use std::collections::HashSet;
use std::sync::Arc;

use russh::MethodSet;
use russh::client::{AuthResult, Handle};
use ssh_config::ResolvedConfig;
use tracing::debug;

use super::identity::IdentitySource;
use super::method::{AuthenticationMethod, MethodPolicy};
use super::outcome::MethodOutcome;
use super::{keyboard_interactive, password, publickey};
use crate::error::ConnectError;
use crate::host_key::ClientHandler;
use crate::prompter::CredentialPrompter;

pub(crate) struct AuthenticationContext {
    pub(crate) alias: String,
    pub(crate) user: String,
    pub(crate) host: String,
    pub(crate) prompter: Arc<dyn CredentialPrompter>,
    pub(crate) number_of_password_prompts: u32,
    pub(crate) planned_methods: Vec<AuthenticationMethod>,
}

impl AuthenticationContext {
    pub(crate) fn from_config(
        config: &ResolvedConfig,
        prompter: Arc<dyn CredentialPrompter>,
    ) -> Self {
        Self {
            alias: config.alias.clone(),
            user: config.user.clone(),
            host: config.hostname.clone(),
            prompter,
            number_of_password_prompts: config.number_of_password_prompts,
            planned_methods: MethodPolicy::from_config(config).planned_methods(),
        }
    }
}

pub(crate) async fn authenticate(
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
