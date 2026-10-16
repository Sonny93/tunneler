use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use engine::{
    CredentialPrompter, HostKeyDecision, HostKeyVerifier, KeyboardInteractiveRequest,
    PassphraseRequest, PasswordRequest, PromptError, RejectedHostKey, UnknownHostKey,
};
use zeroize::Zeroizing;

/// Answers prompts from scripts and records every request.
///
/// An exhausted script cancels the prompt.
#[derive(Default)]
pub struct ScriptedPrompter {
    passphrase_answers: Mutex<VecDeque<Option<String>>>,
    password_answers: Mutex<VecDeque<Option<String>>>,
    keyboard_interactive_answers: Mutex<VecDeque<Option<Vec<String>>>>,
    passphrase_requests: Mutex<Vec<PassphraseRequest>>,
    password_requests: Mutex<Vec<PasswordRequest>>,
    keyboard_interactive_requests: Mutex<Vec<KeyboardInteractiveRequest>>,
}

impl ScriptedPrompter {
    pub fn with_passphrases(self, answers: &[Option<&str>]) -> Self {
        *self.passphrase_answers.lock().unwrap() = owned_answers(answers);
        self
    }

    pub fn with_passwords(self, answers: &[Option<&str>]) -> Self {
        *self.password_answers.lock().unwrap() = owned_answers(answers);
        self
    }

    pub fn with_keyboard_interactive(self, rounds: &[Option<&[&str]>]) -> Self {
        *self.keyboard_interactive_answers.lock().unwrap() = rounds
            .iter()
            .map(|round| round.map(|answers| answers.iter().map(|a| (*a).to_owned()).collect()))
            .collect();
        self
    }

    pub fn passphrase_requests(&self) -> Vec<PassphraseRequest> {
        self.passphrase_requests.lock().unwrap().clone()
    }

    pub fn password_requests(&self) -> Vec<PasswordRequest> {
        self.password_requests.lock().unwrap().clone()
    }

    pub fn keyboard_interactive_requests(&self) -> Vec<KeyboardInteractiveRequest> {
        self.keyboard_interactive_requests.lock().unwrap().clone()
    }

    pub fn request_count(&self) -> usize {
        self.passphrase_requests().len()
            + self.password_requests().len()
            + self.keyboard_interactive_requests().len()
    }
}

fn owned_answers(answers: &[Option<&str>]) -> VecDeque<Option<String>> {
    answers
        .iter()
        .map(|answer| answer.map(str::to_owned))
        .collect()
}

#[async_trait]
impl CredentialPrompter for ScriptedPrompter {
    async fn passphrase(
        &self,
        request: PassphraseRequest,
    ) -> Result<Option<Zeroizing<String>>, PromptError> {
        self.passphrase_requests.lock().unwrap().push(request);
        let answer = self
            .passphrase_answers
            .lock()
            .unwrap()
            .pop_front()
            .flatten();
        Ok(answer.map(Zeroizing::new))
    }

    async fn password(
        &self,
        request: PasswordRequest,
    ) -> Result<Option<Zeroizing<String>>, PromptError> {
        self.password_requests.lock().unwrap().push(request);
        let answer = self.password_answers.lock().unwrap().pop_front().flatten();
        Ok(answer.map(Zeroizing::new))
    }

    async fn keyboard_interactive(
        &self,
        request: KeyboardInteractiveRequest,
    ) -> Result<Option<Vec<Zeroizing<String>>>, PromptError> {
        self.keyboard_interactive_requests
            .lock()
            .unwrap()
            .push(request);
        let answers = self
            .keyboard_interactive_answers
            .lock()
            .unwrap()
            .pop_front()
            .flatten();
        Ok(answers.map(|answers| answers.into_iter().map(Zeroizing::new).collect()))
    }
}

/// Decides about unknown host keys from a script and records every call.
///
/// An exhausted script rejects.
#[derive(Default)]
pub struct ScriptedVerifier {
    decisions: Mutex<VecDeque<HostKeyDecision>>,
    decision_delay: Mutex<Duration>,
    unknown_requests: Mutex<Vec<UnknownHostKey>>,
    rejections: Mutex<Vec<RejectedHostKey>>,
}

impl ScriptedVerifier {
    pub fn with_decisions(self, decisions: &[HostKeyDecision]) -> Self {
        *self.decisions.lock().unwrap() = decisions.iter().copied().collect();
        self
    }

    pub fn with_decision_delay(self, delay: Duration) -> Self {
        *self.decision_delay.lock().unwrap() = delay;
        self
    }

    pub fn unknown_requests(&self) -> Vec<UnknownHostKey> {
        self.unknown_requests.lock().unwrap().clone()
    }

    pub fn rejections(&self) -> Vec<RejectedHostKey> {
        self.rejections.lock().unwrap().clone()
    }
}

#[async_trait]
impl HostKeyVerifier for ScriptedVerifier {
    async fn decide_unknown(&self, host_key: UnknownHostKey) -> HostKeyDecision {
        self.unknown_requests.lock().unwrap().push(host_key);
        let delay = *self.decision_delay.lock().unwrap();
        tokio::time::sleep(delay).await;
        self.decisions
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(HostKeyDecision::Reject)
    }

    async fn report_rejected(&self, rejection: RejectedHostKey) {
        self.rejections.lock().unwrap().push(rejection);
    }
}
