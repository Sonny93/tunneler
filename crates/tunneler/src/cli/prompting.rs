//! Answers the daemon's prompts on the terminal while `up` waits.
//!
//! Questions go to stderr and hidden input is read from the TTY. A prompt that
//! another client answers first is dropped, and a late answer that the daemon
//! refuses is ignored. The warnings about refused host keys are not prompts:
//! `up` shows them whether or not it can ask.

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use ipc::protocol::{
    CredentialAnswer, CredentialRequestDto, CredentialRequestEvent, ErrorCode, EventMessage,
    HostKeyAlertReasonDto, HostKeyDecisionDto, HostKeyRequestEvent, Secret,
};
use ipc::{ClientError, IpcClient};
use tokio::sync::Mutex as AsyncMutex;

use super::terminal::Terminal;

const PROMPT_ANSWERED_ELSEWHERE_MESSAGE: &str =
    "The prompt was answered elsewhere or expired; no input is needed.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Waiting,
    AnsweredHere,
    ResolvedElsewhere,
}

/// Tells whether a prompt still needs this terminal's answer.
#[derive(Debug, Clone)]
struct PromptSlot {
    state: Arc<Mutex<SlotState>>,
}

impl PromptSlot {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SlotState::Waiting)),
        }
    }

    fn transition(&self, to: SlotState) -> bool {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if *state != SlotState::Waiting {
            return false;
        }
        *state = to;
        true
    }

    fn claim_for_local_answer(&self) -> bool {
        self.transition(SlotState::AnsweredHere)
    }

    fn mark_resolved_elsewhere(&self) -> bool {
        self.transition(SlotState::ResolvedElsewhere)
    }

    fn is_waiting(&self) -> bool {
        *self.state.lock().unwrap_or_else(PoisonError::into_inner) == SlotState::Waiting
    }
}

/// Reads the answer to a credential request; `None` when the user declines by
/// leaving an answer empty.
pub fn collect_credential_answer(
    terminal: &dyn Terminal,
    request: &CredentialRequestDto,
) -> io::Result<Option<CredentialAnswer>> {
    match request {
        CredentialRequestDto::Passphrase { alias, key_path } => single_secret(
            terminal,
            &format!("Enter passphrase for key '{key_path}' ({alias}): "),
        ),
        CredentialRequestDto::Password { alias, user, host } => {
            single_secret(terminal, &format!("Password for {user}@{host} ({alias}): "))
        }
        CredentialRequestDto::KeyboardInteractive {
            name,
            instructions,
            prompts,
            ..
        } => {
            for heading in [name, instructions]
                .into_iter()
                .filter(|text| !text.is_empty())
            {
                terminal.write(heading);
            }
            let mut answers = Vec::with_capacity(prompts.len());
            for prompt in prompts {
                answers.push(read_answer(terminal, &prompt.text, prompt.echo)?);
            }
            Ok(Some(CredentialAnswer::Multiple(answers)))
        }
    }
}

fn single_secret(terminal: &dyn Terminal, prompt: &str) -> io::Result<Option<CredentialAnswer>> {
    let secret = terminal.read_secret(prompt)?;
    if secret.is_empty() {
        return Ok(None);
    }
    Ok(Some(CredentialAnswer::Single(Secret::from(secret))))
}

fn read_answer(terminal: &dyn Terminal, prompt: &str, is_echoed: bool) -> io::Result<Secret> {
    if is_echoed {
        return Ok(Secret::new(terminal.read_line(prompt)?));
    }
    Ok(Secret::from(terminal.read_secret(prompt)?))
}

/// What the typed text decides about an unknown host key: `y` saves, `o`
/// trusts once, anything else rejects.
pub fn parse_host_key_decision(answer: &str) -> HostKeyDecisionDto {
    match answer.trim().to_lowercase().as_str() {
        "y" | "yes" => HostKeyDecisionDto::AcceptAndSave,
        "o" | "once" => HostKeyDecisionDto::AcceptOnce,
        _ => HostKeyDecisionDto::Reject,
    }
}

/// Shows an unknown host key and asks what to do with it.
pub fn collect_host_key_decision(
    terminal: &dyn Terminal,
    alias: &str,
    lookup_name: &str,
    algorithm: &str,
    fingerprint_sha256: &str,
) -> io::Result<HostKeyDecisionDto> {
    terminal.write(&format!(
        "The authenticity of host '{alias}' ({lookup_name}) can't be established.\n\
         {algorithm} key fingerprint is {fingerprint_sha256}."
    ));
    let answer = terminal.read_line("Trust this host key? [y] save / [o] once / [N] reject: ")?;
    Ok(parse_host_key_decision(&answer))
}

/// The warning shown when a host key was refused without asking.
pub fn describe_host_key_alert(
    alias: &str,
    reason: &HostKeyAlertReasonDto,
    presented_fingerprint: &str,
) -> String {
    match reason {
        HostKeyAlertReasonDto::Changed {
            known_fingerprint,
            path,
            line,
        } => format!(
            "WARNING: the host key of '{alias}' has CHANGED. Someone may be intercepting the \
             connection, or the host was reinstalled.\n\
             Presented fingerprint: {presented_fingerprint}\n\
             Recorded fingerprint:  {known_fingerprint} ({path}:{line})\n\
             The connection was refused. Remove the old key from {path} only if you are sure \
             the change is legitimate."
        ),
        HostKeyAlertReasonDto::Revoked { path, line } => format!(
            "WARNING: the host key of '{alias}' is REVOKED ({path}:{line}).\n\
             Presented fingerprint: {presented_fingerprint}\n\
             The connection was refused."
        ),
        HostKeyAlertReasonDto::UnknownRejectedByPolicy => format!(
            "The host key of '{alias}' is not known, and the configuration forbids adding it \
             (StrictHostKeyChecking).\n\
             Presented fingerprint: {presented_fingerprint}\n\
             The connection was refused."
        ),
    }
}

/// Answers the prompts that reach this command, one at a time.
pub struct LocalPrompts {
    client: Arc<IpcClient>,
    terminal: Arc<dyn Terminal>,
    turn: Arc<AsyncMutex<()>>,
    slots: HashMap<String, PromptSlot>,
}

impl LocalPrompts {
    /// Answers through `client`, asking on `terminal`.
    pub fn new(client: Arc<IpcClient>, terminal: Arc<dyn Terminal>) -> Self {
        Self {
            client,
            terminal,
            turn: Arc::new(AsyncMutex::new(())),
            slots: HashMap::new(),
        }
    }

    /// Reacts to an event; events that are not about prompts are ignored.
    pub fn handle_event(&mut self, event: &EventMessage) {
        match event {
            EventMessage::CredentialRequest(request) => self.answer_credential(request.clone()),
            EventMessage::HostKeyRequest(HostKeyRequestEvent::Prompt {
                prompt_id,
                alias,
                lookup_name,
                algorithm,
                fingerprint_sha256,
            }) => {
                self.answer_host_key(prompt_id, alias, lookup_name, algorithm, fingerprint_sha256);
            }
            EventMessage::PromptResolved(resolved) => self.drop_prompt(&resolved.prompt_id),
            EventMessage::HostKeyRequest(HostKeyRequestEvent::Alert { .. })
            | EventMessage::TunnelChanged(_)
            | EventMessage::TunnelMetrics(_)
            | EventMessage::TunnelLog(_)
            | EventMessage::ConfigReloaded {}
            | EventMessage::ConfigInvalid(_)
            | EventMessage::TrayChanged(_)
            | EventMessage::ShowRequested {}
            | EventMessage::QuitRequested {} => {}
        }
    }

    /// Whether a question is still waiting for the user's answer.
    pub fn has_pending(&self) -> bool {
        self.slots.values().any(PromptSlot::is_waiting)
    }

    fn drop_prompt(&mut self, prompt_id: &str) {
        let Some(slot) = self.slots.remove(prompt_id) else {
            return;
        };
        if slot.mark_resolved_elsewhere() {
            self.terminal.write(PROMPT_ANSWERED_ELSEWHERE_MESSAGE);
        }
    }

    fn answer_credential(&mut self, request: CredentialRequestEvent) {
        let slot = self.open_slot(&request.prompt_id);
        let context = self.context(slot);
        tokio::spawn(async move {
            let prompt_id = request.prompt_id;
            let dto = request.request;
            let Some(answer) = context
                .ask(
                    move |terminal| collect_credential_answer(terminal, &dto),
                    None,
                )
                .await
            else {
                return;
            };
            context.send_credential(&prompt_id, answer).await;
        });
    }

    fn answer_host_key(
        &mut self,
        prompt_id: &str,
        alias: &str,
        lookup_name: &str,
        algorithm: &str,
        fingerprint_sha256: &str,
    ) {
        let slot = self.open_slot(prompt_id);
        let context = self.context(slot);
        let prompt_id = prompt_id.to_owned();
        let (alias, lookup_name) = (alias.to_owned(), lookup_name.to_owned());
        let (algorithm, fingerprint) = (algorithm.to_owned(), fingerprint_sha256.to_owned());
        tokio::spawn(async move {
            let Some(decision) = context
                .ask(
                    move |terminal| {
                        collect_host_key_decision(
                            terminal,
                            &alias,
                            &lookup_name,
                            &algorithm,
                            &fingerprint,
                        )
                    },
                    HostKeyDecisionDto::Reject,
                )
                .await
            else {
                return;
            };
            context.send_host_key(&prompt_id, decision).await;
        });
    }

    fn open_slot(&mut self, prompt_id: &str) -> PromptSlot {
        let slot = PromptSlot::new();
        self.slots.insert(prompt_id.to_owned(), slot.clone());
        slot
    }

    fn context(&self, slot: PromptSlot) -> PromptContext {
        PromptContext {
            client: Arc::clone(&self.client),
            terminal: Arc::clone(&self.terminal),
            turn: Arc::clone(&self.turn),
            slot,
        }
    }
}

struct PromptContext {
    client: Arc<IpcClient>,
    terminal: Arc<dyn Terminal>,
    turn: Arc<AsyncMutex<()>>,
    slot: PromptSlot,
}

impl PromptContext {
    async fn ask<Answer: Send + 'static>(
        &self,
        collect: impl FnOnce(&dyn Terminal) -> io::Result<Answer> + Send + 'static,
        on_failure: Answer,
    ) -> Option<Answer> {
        let _turn = self.turn.lock().await;
        if !self.slot.is_waiting() {
            return None;
        }
        let terminal = Arc::clone(&self.terminal);
        let outcome = tokio::task::spawn_blocking(move || collect(terminal.as_ref()))
            .await
            .map_err(io::Error::other)
            .and_then(|collected| collected);
        let answer = outcome.unwrap_or_else(|error| {
            self.terminal
                .write(&format!("cannot read the answer: {error}"));
            on_failure
        });
        self.slot.claim_for_local_answer().then_some(answer)
    }

    async fn send_credential(&self, prompt_id: &str, answer: Option<CredentialAnswer>) {
        let outcome = self.client.respond_credential(prompt_id, answer).await;
        self.report(outcome);
    }

    async fn send_host_key(&self, prompt_id: &str, decision: HostKeyDecisionDto) {
        let outcome = self.client.respond_host_key(prompt_id, decision).await;
        self.report(outcome);
    }

    fn report(&self, outcome: Result<(), ClientError>) {
        match outcome {
            Ok(())
            | Err(ClientError::Remote {
                code: ErrorCode::PromptNotPending,
                ..
            }) => {}
            Err(error) => self
                .terminal
                .write(&format!("cannot send the answer: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use ipc::protocol::PromptDto;
    use zeroize::Zeroizing;

    use super::*;

    #[derive(Default)]
    struct ScriptedTerminal {
        answers: Mutex<VecDeque<String>>,
        asked: Mutex<Vec<String>>,
        written: Mutex<Vec<String>>,
    }

    impl ScriptedTerminal {
        fn answering(answers: &[&str]) -> Self {
            Self {
                answers: Mutex::new(answers.iter().map(|text| (*text).to_owned()).collect()),
                ..Self::default()
            }
        }

        fn next_answer(&self, prompt: &str) -> io::Result<String> {
            self.asked.lock().unwrap().push(prompt.to_owned());
            self.answers
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))
        }

        fn asked(&self) -> Vec<String> {
            self.asked.lock().unwrap().clone()
        }

        fn written(&self) -> Vec<String> {
            self.written.lock().unwrap().clone()
        }
    }

    impl Terminal for ScriptedTerminal {
        fn write(&self, text: &str) {
            self.written.lock().unwrap().push(text.to_owned());
        }

        fn read_secret(&self, prompt: &str) -> io::Result<Zeroizing<String>> {
            self.next_answer(prompt).map(Zeroizing::new)
        }

        fn read_line(&self, prompt: &str) -> io::Result<String> {
            self.next_answer(prompt)
        }
    }

    fn exposed(answer: Option<CredentialAnswer>) -> Vec<String> {
        match answer {
            None => Vec::new(),
            Some(CredentialAnswer::Single(secret)) => vec![secret.expose().to_owned()],
            Some(CredentialAnswer::Multiple(secrets)) => secrets
                .iter()
                .map(|secret| secret.expose().to_owned())
                .collect(),
        }
    }

    #[test]
    fn should_ask_for_the_passphrase_of_the_key_when_the_request_is_a_passphrase() {
        let terminal = ScriptedTerminal::answering(&["hunter2"]);
        let request = CredentialRequestDto::Passphrase {
            alias: "nova".to_owned(),
            key_path: "/home/me/.ssh/id_ed25519".to_owned(),
        };

        let answer = collect_credential_answer(&terminal, &request).unwrap();

        assert_eq!(
            (exposed(answer), terminal.asked()),
            (
                vec!["hunter2".to_owned()],
                vec!["Enter passphrase for key '/home/me/.ssh/id_ed25519' (nova): ".to_owned()]
            )
        );
    }

    #[test]
    fn should_ask_for_the_password_of_the_account_when_the_request_is_a_password() {
        let terminal = ScriptedTerminal::answering(&["s3cret"]);
        let request = CredentialRequestDto::Password {
            alias: "nova".to_owned(),
            user: "alice".to_owned(),
            host: "nova.example".to_owned(),
        };

        collect_credential_answer(&terminal, &request).unwrap();

        assert_eq!(
            terminal.asked(),
            vec!["Password for alice@nova.example (nova): ".to_owned()]
        );
    }

    #[test]
    fn should_cancel_when_the_secret_is_left_empty() {
        let terminal = ScriptedTerminal::answering(&[""]);
        let request = CredentialRequestDto::Password {
            alias: "nova".to_owned(),
            user: "alice".to_owned(),
            host: "nova".to_owned(),
        };

        assert_eq!(
            collect_credential_answer(&terminal, &request).unwrap(),
            None
        );
    }

    #[test]
    fn should_answer_every_question_in_order_when_the_request_is_keyboard_interactive() {
        let terminal = ScriptedTerminal::answering(&["123456", "alice"]);
        let request = CredentialRequestDto::KeyboardInteractive {
            alias: "nova".to_owned(),
            name: "Two factor".to_owned(),
            instructions: String::new(),
            prompts: vec![
                PromptDto {
                    text: "Code: ".to_owned(),
                    echo: false,
                },
                PromptDto {
                    text: "Name: ".to_owned(),
                    echo: true,
                },
            ],
        };

        let answer = collect_credential_answer(&terminal, &request).unwrap();

        assert_eq!(
            (exposed(answer), terminal.written()),
            (
                vec!["123456".to_owned(), "alice".to_owned()],
                vec!["Two factor".to_owned()]
            )
        );
    }

    #[test]
    fn should_fail_when_the_terminal_cannot_be_read() {
        let terminal = ScriptedTerminal::answering(&[]);
        let request = CredentialRequestDto::Password {
            alias: "nova".to_owned(),
            user: "alice".to_owned(),
            host: "nova".to_owned(),
        };

        assert!(collect_credential_answer(&terminal, &request).is_err());
    }

    #[test]
    fn should_parse_the_answer_to_a_host_key_question() {
        assert_eq!(
            ["y", "Y", " yes ", "o", "once", "n", "", "whatever"].map(parse_host_key_decision),
            [
                HostKeyDecisionDto::AcceptAndSave,
                HostKeyDecisionDto::AcceptAndSave,
                HostKeyDecisionDto::AcceptAndSave,
                HostKeyDecisionDto::AcceptOnce,
                HostKeyDecisionDto::AcceptOnce,
                HostKeyDecisionDto::Reject,
                HostKeyDecisionDto::Reject,
                HostKeyDecisionDto::Reject,
            ]
        );
    }

    #[test]
    fn should_show_the_alias_algorithm_and_fingerprint_when_asking_about_a_host_key() {
        let terminal = ScriptedTerminal::answering(&["o"]);

        let decision = collect_host_key_decision(
            &terminal,
            "nova",
            "[nova]:2222",
            "ssh-ed25519",
            "SHA256:abc",
        )
        .unwrap();

        assert_eq!(
            (decision, terminal.written(), terminal.asked()),
            (
                HostKeyDecisionDto::AcceptOnce,
                vec![
                    "The authenticity of host 'nova' ([nova]:2222) can't be established.\n\
                     ssh-ed25519 key fingerprint is SHA256:abc."
                        .to_owned()
                ],
                vec!["Trust this host key? [y] save / [o] once / [N] reject: ".to_owned()]
            )
        );
    }

    #[test]
    fn should_warn_explicitly_when_the_host_key_changed() {
        let warning = describe_host_key_alert(
            "nova",
            &HostKeyAlertReasonDto::Changed {
                known_fingerprint: "SHA256:old".to_owned(),
                path: "/home/me/.ssh/known_hosts".to_owned(),
                line: 4,
            },
            "SHA256:new",
        );

        assert!(
            warning.starts_with("WARNING: the host key of 'nova' has CHANGED.")
                && warning.contains("SHA256:old (/home/me/.ssh/known_hosts:4)")
                && warning.contains("SHA256:new")
                && warning.contains("The connection was refused.")
        );
    }

    #[test]
    fn should_warn_explicitly_when_the_host_key_is_revoked() {
        let warning = describe_host_key_alert(
            "nova",
            &HostKeyAlertReasonDto::Revoked {
                path: "known_hosts".to_owned(),
                line: 2,
            },
            "SHA256:new",
        );

        assert!(warning.contains("REVOKED") && warning.contains("known_hosts:2"));
    }

    #[test]
    fn should_explain_the_policy_when_an_unknown_host_key_is_refused_without_asking() {
        let warning = describe_host_key_alert(
            "nova",
            &HostKeyAlertReasonDto::UnknownRejectedByPolicy,
            "SHA256:new",
        );

        assert!(warning.contains("StrictHostKeyChecking"));
    }

    #[test]
    fn should_let_only_one_party_claim_a_prompt() {
        let slot = PromptSlot::new();

        assert_eq!(
            (
                slot.claim_for_local_answer(),
                slot.mark_resolved_elsewhere(),
                slot.claim_for_local_answer()
            ),
            (true, false, false)
        );
    }

    #[test]
    fn should_stop_waiting_for_an_answer_when_the_prompt_was_resolved_elsewhere() {
        let slot = PromptSlot::new();

        slot.mark_resolved_elsewhere();

        assert!(!slot.is_waiting());
    }
}
