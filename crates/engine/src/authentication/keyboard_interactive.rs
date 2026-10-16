use russh::client::{Handle, KeyboardInteractiveAuthResponse};
use tracing::warn;

use super::flow::AuthenticationContext;
use super::outcome::MethodOutcome;
use crate::error::ConnectError;
use crate::host_key::ClientHandler;
use crate::prompter::{KeyboardInteractiveRequest, Prompt, PromptError};

const MAXIMUM_INFO_REQUEST_ROUNDS: usize = 10;

pub(crate) async fn attempt(
    handle: &mut Handle<ClientHandler>,
    context: &AuthenticationContext,
) -> Result<MethodOutcome, ConnectError> {
    let mut response = handle
        .authenticate_keyboard_interactive_start(context.user.as_str(), None)
        .await?;
    for _ in 0..MAXIMUM_INFO_REQUEST_ROUNDS {
        match response {
            KeyboardInteractiveAuthResponse::Success => return Ok(MethodOutcome::Authenticated),
            KeyboardInteractiveAuthResponse::Failure {
                remaining_methods,
                partial_success,
            } => {
                return Ok(MethodOutcome::Rejected {
                    remaining_methods,
                    partial_success,
                });
            }
            KeyboardInteractiveAuthResponse::InfoRequest {
                name,
                instructions,
                prompts,
            } => {
                let request = KeyboardInteractiveRequest {
                    alias: context.alias.clone(),
                    name,
                    instructions,
                    prompts: prompts
                        .into_iter()
                        .map(|prompt| Prompt {
                            text: prompt.prompt,
                            echo: prompt.echo,
                        })
                        .collect(),
                };
                let Some(answers) = answer(context, request).await? else {
                    return Ok(MethodOutcome::NotAttempted);
                };
                response = handle
                    .authenticate_keyboard_interactive_respond(answers)
                    .await?;
            }
        }
    }
    warn!(alias = %context.alias, "the server keeps asking keyboard-interactive questions");
    Ok(MethodOutcome::NotAttempted)
}

async fn answer(
    context: &AuthenticationContext,
    request: KeyboardInteractiveRequest,
) -> Result<Option<Vec<String>>, PromptError> {
    if request.prompts.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let expected = request.prompts.len();
    let Some(answers) = context.prompter.keyboard_interactive(request).await? else {
        return Ok(None);
    };
    if answers.len() != expected {
        return Err(PromptError::AnswerCountMismatch {
            expected,
            actual: answers.len(),
        });
    }
    Ok(Some(
        answers
            .iter()
            .map(|answer| answer.as_str().to_owned())
            .collect(),
    ))
}
