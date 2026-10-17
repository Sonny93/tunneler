use std::path::PathBuf;

use async_trait::async_trait;
use thiserror::Error;
use zeroize::Zeroizing;

/// Why the passphrase of an identity file is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassphraseRequest {
    /// The host alias being connected to.
    pub alias: String,
    /// The encrypted identity file.
    pub key_path: PathBuf,
}

/// How long the engine keeps a passphrase after it unlocked the identity file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Retention {
    /// Forget it right away; the next connection asks again.
    #[default]
    Never,
    /// Keep it in memory until the daemon stops.
    Session,
    /// Keep it in memory and in the OS keyring, when the keyring is enabled.
    Keyring,
}

/// The passphrase the user typed and how long to remember it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassphraseAnswer {
    /// The passphrase of the identity file.
    pub passphrase: Zeroizing<String>,
    /// Whether and where to remember it once it unlocked the file.
    pub retention: Retention,
}

impl PassphraseAnswer {
    /// An answer that is used once and forgotten.
    pub fn without_retention(passphrase: Zeroizing<String>) -> Self {
        Self {
            passphrase,
            retention: Retention::Never,
        }
    }
}

/// Why a password is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordRequest {
    /// The host alias being connected to.
    pub alias: String,
    /// The remote login name.
    pub user: String,
    /// The host name being connected to.
    pub host: String,
}

/// One question of a keyboard-interactive exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    /// The question to show.
    pub text: String,
    /// Whether the typed answer may be displayed.
    pub echo: bool,
}

/// One round of keyboard-interactive questions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardInteractiveRequest {
    /// The host alias being connected to.
    pub alias: String,
    /// The title sent by the server; may be empty.
    pub name: String,
    /// The instructions sent by the server; may be empty.
    pub instructions: String,
    /// The questions, answered in order.
    pub prompts: Vec<Prompt>,
}

/// Why a prompt could not be answered at all.
///
/// A user who declines to answer is not an error: the prompter returns `None`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PromptError {
    /// The prompter could not reach the user.
    #[error("cannot ask the user: {reason}")]
    Unavailable {
        /// Why the user could not be reached.
        reason: String,
    },
    /// The prompter answered a different number of questions than it was asked.
    #[error("expected {expected} answers but got {actual}")]
    AnswerCountMismatch {
        /// The number of questions.
        expected: usize,
        /// The number of answers.
        actual: usize,
    },
}

/// Where the engine gets secrets from.
///
/// Every method returns `None` when the user cancelled, which makes the engine
/// move on to the next authentication method.
#[async_trait]
pub trait CredentialPrompter: Send + Sync {
    /// The passphrase of an encrypted identity file.
    async fn passphrase(
        &self,
        request: PassphraseRequest,
    ) -> Result<Option<PassphraseAnswer>, PromptError>;

    /// The account password.
    async fn password(
        &self,
        request: PasswordRequest,
    ) -> Result<Option<Zeroizing<String>>, PromptError>;

    /// The answers to a keyboard-interactive round, one per prompt, in order.
    async fn keyboard_interactive(
        &self,
        request: KeyboardInteractiveRequest,
    ) -> Result<Option<Vec<Zeroizing<String>>>, PromptError>;
}
