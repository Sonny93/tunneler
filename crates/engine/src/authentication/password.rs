use russh::client::{AuthResult, Handle};

use super::flow::AuthenticationContext;
use super::outcome::MethodOutcome;
use crate::error::ConnectError;
use crate::host_key::ClientHandler;
use crate::prompter::PasswordRequest;

pub(crate) async fn attempt(
    handle: &mut Handle<ClientHandler>,
    context: &AuthenticationContext,
) -> Result<MethodOutcome, ConnectError> {
    let mut last_rejection = None;
    for _ in 0..context.number_of_password_prompts {
        let request = PasswordRequest {
            alias: context.alias.clone(),
            user: context.user.clone(),
            host: context.host.clone(),
        };
        let Some(password) = context.prompter.password(request).await? else {
            break;
        };
        let result = handle
            .authenticate_password(context.user.as_str(), password.as_str())
            .await?;
        match result {
            AuthResult::Success => return Ok(MethodOutcome::Authenticated),
            AuthResult::Failure {
                remaining_methods,
                partial_success: true,
            } => {
                return Ok(MethodOutcome::Rejected {
                    remaining_methods,
                    partial_success: true,
                });
            }
            AuthResult::Failure {
                remaining_methods, ..
            } => last_rejection = Some(remaining_methods),
        }
    }
    Ok(match last_rejection {
        Some(remaining_methods) => MethodOutcome::Rejected {
            remaining_methods,
            partial_success: false,
        },
        None => MethodOutcome::NotAttempted,
    })
}
