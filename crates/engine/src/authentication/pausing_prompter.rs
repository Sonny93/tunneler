use std::sync::Arc;

use async_trait::async_trait;
use zeroize::Zeroizing;

use crate::bounded::DeadlinePause;
use crate::prompter::{
    CredentialPrompter, KeyboardInteractiveRequest, PassphraseRequest, PasswordRequest, PromptError,
};

/// Pauses a deadline for as long as a question to the user is pending, so
/// that time spent typing a secret never counts against it.
pub(crate) struct PausingPrompter {
    inner: Arc<dyn CredentialPrompter>,
    pause: Arc<DeadlinePause>,
}

impl PausingPrompter {
    pub(crate) fn new(inner: Arc<dyn CredentialPrompter>, pause: Arc<DeadlinePause>) -> Self {
        Self { inner, pause }
    }
}

#[async_trait]
impl CredentialPrompter for PausingPrompter {
    async fn passphrase(
        &self,
        request: PassphraseRequest,
    ) -> Result<Option<Zeroizing<String>>, PromptError> {
        let _pause = self.pause.pause();
        self.inner.passphrase(request).await
    }

    async fn password(
        &self,
        request: PasswordRequest,
    ) -> Result<Option<Zeroizing<String>>, PromptError> {
        let _pause = self.pause.pause();
        self.inner.password(request).await
    }

    async fn keyboard_interactive(
        &self,
        request: KeyboardInteractiveRequest,
    ) -> Result<Option<Vec<Zeroizing<String>>>, PromptError> {
        let _pause = self.pause.pause();
        self.inner.keyboard_interactive(request).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;

    const DECISION_DELAY: Duration = Duration::from_millis(100);

    struct SlowPrompter;

    #[async_trait]
    impl CredentialPrompter for SlowPrompter {
        async fn passphrase(
            &self,
            _request: PassphraseRequest,
        ) -> Result<Option<Zeroizing<String>>, PromptError> {
            sleep(DECISION_DELAY).await;
            Ok(None)
        }

        async fn password(
            &self,
            _request: PasswordRequest,
        ) -> Result<Option<Zeroizing<String>>, PromptError> {
            sleep(DECISION_DELAY).await;
            Ok(None)
        }

        async fn keyboard_interactive(
            &self,
            _request: KeyboardInteractiveRequest,
        ) -> Result<Option<Vec<Zeroizing<String>>>, PromptError> {
            sleep(DECISION_DELAY).await;
            Ok(None)
        }
    }

    fn password_request() -> PasswordRequest {
        PasswordRequest {
            alias: "web".to_owned(),
            user: "alice".to_owned(),
            host: "web.example".to_owned(),
        }
    }

    #[tokio::test]
    async fn should_pause_the_deadline_while_a_prompt_is_pending() {
        let pause = Arc::new(DeadlinePause::default());
        let prompter = PausingPrompter::new(Arc::new(SlowPrompter), Arc::clone(&pause));

        let pending_prompt = prompter.password(password_request());
        tokio::pin!(pending_prompt);
        let is_paused_midway = tokio::select! {
            _ = &mut pending_prompt => false,
            () = sleep(DECISION_DELAY / 4) => pause.is_paused(),
        };

        assert!(is_paused_midway);
    }

    #[tokio::test]
    async fn should_resume_the_deadline_when_the_prompt_is_answered() {
        let pause = Arc::new(DeadlinePause::default());
        let prompter = PausingPrompter::new(Arc::new(SlowPrompter), Arc::clone(&pause));

        let _ = prompter.password(password_request()).await;

        assert!(!pause.is_paused());
    }

    #[tokio::test]
    async fn should_resume_the_deadline_when_the_prompt_is_dropped() {
        let pause = Arc::new(DeadlinePause::default());
        let prompter = PausingPrompter::new(Arc::new(SlowPrompter), Arc::clone(&pause));

        drop(tokio::time::timeout(DECISION_DELAY / 4, prompter.password(password_request())).await);

        assert!(!pause.is_paused());
    }
}
