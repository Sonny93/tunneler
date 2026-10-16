//! The prompt broker: asks the connected clients for secrets and host key
//! decisions on behalf of the engine.
//!
//! A prompt goes to every connection that declared `can_prompt`. The first
//! answer wins, the others are told through `prompt.resolved`. A prompt nobody
//! answers within [`PROMPT_TIMEOUT`], or whose responders all disconnect, is
//! cancelled. Secrets stay in wiping containers and are never logged.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use engine::{
    CredentialPrompter, HostKeyDecision, HostKeyVerifier, KeyboardInteractiveRequest,
    PassphraseRequest, PasswordRequest, PromptError, RejectedHostKey, UnknownHostKey,
};
use ipc::protocol::{
    CredentialAnswer, CredentialRequestDto, CredentialRequestEvent, CredentialResponseParams,
    EventMessage, HostKeyDecisionDto, HostKeyRequestEvent, HostKeyResponseParams, IpcError,
    PromptDto, PromptResolvedEvent,
};
use ipc::{ConnectionId, ConnectionNotification, ServerHandle};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, oneshot};
use ulid::Ulid;
use zeroize::Zeroizing;

use super::mapping::{host_key_decision_from_dto, rejection_to_alert};

/// How long a prompt waits for an answer before it is cancelled.
pub(crate) const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

struct PendingPrompt<Answer> {
    responders: Vec<ConnectionId>,
    answer: oneshot::Sender<Answer>,
}

/// The prompts waiting for an answer, by identifier.
struct PendingPrompts<Answer> {
    entries: Mutex<HashMap<String, PendingPrompt<Answer>>>,
}

impl<Answer> Default for PendingPrompts<Answer> {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl<Answer> PendingPrompts<Answer> {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, PendingPrompt<Answer>>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn register(
        &self,
        prompt_id: &str,
        responders: Vec<ConnectionId>,
    ) -> oneshot::Receiver<Answer> {
        let (answer, receiver) = oneshot::channel();
        self.lock()
            .insert(prompt_id.to_owned(), PendingPrompt { responders, answer });
        receiver
    }

    fn resolve(&self, prompt_id: &str, answer: Answer) -> Result<Vec<ConnectionId>, IpcError> {
        let pending = self
            .lock()
            .remove(prompt_id)
            .ok_or_else(|| IpcError::prompt_not_pending(prompt_id))?;
        let _ = pending.answer.send(answer);
        Ok(pending.responders)
    }

    fn withdraw(&self, prompt_id: &str) -> Vec<ConnectionId> {
        self.lock()
            .remove(prompt_id)
            .map(|pending| pending.responders)
            .unwrap_or_default()
    }
}

/// Answers the engine's prompts through the connected clients.
pub(crate) struct PromptBroker {
    server: ServerHandle,
    events: broadcast::Sender<EventMessage>,
    timeout: Duration,
    credentials: PendingPrompts<Option<CredentialAnswer>>,
    host_keys: PendingPrompts<HostKeyDecisionDto>,
}

impl PromptBroker {
    /// A broker that talks to clients through `server`; `events` carries the
    /// alerts that are broadcast to every subscriber.
    pub(crate) fn new(server: ServerHandle, events: broadcast::Sender<EventMessage>) -> Self {
        Self {
            server,
            events,
            timeout: PROMPT_TIMEOUT,
            credentials: PendingPrompts::default(),
            host_keys: PendingPrompts::default(),
        }
    }

    #[cfg(test)]
    #[cfg(unix)]
    pub(crate) fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Delivers the answer of a client to the credential prompt waiting for it.
    ///
    /// `answerer` is left out of the `prompt.resolved` notifications.
    pub(crate) fn respond_credential(
        &self,
        answerer: Option<ConnectionId>,
        params: CredentialResponseParams,
    ) -> Result<(), IpcError> {
        let responders = self.credentials.resolve(&params.prompt_id, params.answer)?;
        self.notify_resolved(&params.prompt_id, &responders, answerer);
        Ok(())
    }

    /// Delivers the decision of a client to the host key prompt waiting for it.
    ///
    /// `answerer` is left out of the `prompt.resolved` notifications.
    pub(crate) fn respond_host_key(
        &self,
        answerer: Option<ConnectionId>,
        params: &HostKeyResponseParams,
    ) -> Result<(), IpcError> {
        let responders = self.host_keys.resolve(&params.prompt_id, params.decision)?;
        self.notify_resolved(&params.prompt_id, &responders, answerer);
        Ok(())
    }

    async fn ask<Answer>(
        &self,
        pending: &PendingPrompts<Answer>,
        build_event: impl FnOnce(String) -> EventMessage,
    ) -> Option<Answer> {
        let responders = self.prompt_responders();
        if responders.is_empty() {
            return without_responder();
        }
        let mut notifications = self.server.subscribe_connections();
        let prompt_id = Ulid::generate().to_string();
        let receiver = pending.register(&prompt_id, responders.clone());
        let event = build_event(prompt_id.clone());
        let reached = self.deliver(&event, &responders);
        if reached.is_empty() {
            pending.withdraw(&prompt_id);
            return None;
        }
        let answer = tokio::time::timeout(self.timeout, async {
            tokio::select! {
                answer = receiver => answer.ok(),
                () = self.wait_until_disconnected(&reached, &mut notifications) => None,
            }
        })
        .await
        .unwrap_or(None);
        if answer.is_none() {
            let remaining = pending.withdraw(&prompt_id);
            self.notify_resolved(&prompt_id, &remaining, None);
        }
        answer
    }

    fn prompt_responders(&self) -> Vec<ConnectionId> {
        self.server
            .connections()
            .into_iter()
            .filter(|connection| connection.can_prompt)
            .map(|connection| connection.id)
            .collect()
    }

    fn deliver(&self, event: &EventMessage, responders: &[ConnectionId]) -> Vec<ConnectionId> {
        responders
            .iter()
            .copied()
            .filter(|responder| self.server.send_to(*responder, event).is_ok())
            .collect()
    }

    fn notify_resolved(
        &self,
        prompt_id: &str,
        responders: &[ConnectionId],
        answerer: Option<ConnectionId>,
    ) {
        let event = EventMessage::PromptResolved(PromptResolvedEvent {
            prompt_id: prompt_id.to_owned(),
        });
        let others: Vec<ConnectionId> = responders
            .iter()
            .copied()
            .filter(|responder| Some(*responder) != answerer)
            .collect();
        self.deliver(&event, &others);
    }

    async fn wait_until_disconnected(
        &self,
        responders: &[ConnectionId],
        notifications: &mut broadcast::Receiver<ConnectionNotification>,
    ) {
        loop {
            if !self.is_any_connected(responders) {
                return;
            }
            if matches!(notifications.recv().await, Err(RecvError::Closed)) {
                std::future::pending::<()>().await;
            }
        }
    }

    fn is_any_connected(&self, responders: &[ConnectionId]) -> bool {
        self.server
            .connections()
            .iter()
            .any(|connection| responders.contains(&connection.id))
    }

    async fn ask_credential(&self, request: CredentialRequestDto) -> Option<CredentialAnswer> {
        let answer = self
            .ask(&self.credentials, |prompt_id| {
                EventMessage::CredentialRequest(CredentialRequestEvent { prompt_id, request })
            })
            .await;
        answer.flatten()
    }
}

/// The single place that decides what happens when no client can be asked.
///
/// Cancelling is the behaviour until the daemon can launch a UI to ask.
fn without_responder<Answer>() -> Option<Answer> {
    tracing::warn!("a prompt was cancelled because no connected client can answer it");
    None
}

fn single_secret(
    answer: Option<CredentialAnswer>,
) -> Result<Option<Zeroizing<String>>, PromptError> {
    match answer {
        None => Ok(None),
        Some(CredentialAnswer::Single(secret)) => Ok(Some(secret.into_zeroizing())),
        Some(CredentialAnswer::Multiple(mut secrets)) if secrets.len() == 1 => {
            Ok(secrets.pop().map(ipc::protocol::Secret::into_zeroizing))
        }
        Some(CredentialAnswer::Multiple(secrets)) => Err(PromptError::AnswerCountMismatch {
            expected: 1,
            actual: secrets.len(),
        }),
    }
}

fn many_secrets(answer: Option<CredentialAnswer>) -> Option<Vec<Zeroizing<String>>> {
    match answer? {
        CredentialAnswer::Single(secret) => Some(vec![secret.into_zeroizing()]),
        CredentialAnswer::Multiple(secrets) => Some(
            secrets
                .into_iter()
                .map(ipc::protocol::Secret::into_zeroizing)
                .collect(),
        ),
    }
}

#[async_trait]
impl CredentialPrompter for PromptBroker {
    async fn passphrase(
        &self,
        request: PassphraseRequest,
    ) -> Result<Option<Zeroizing<String>>, PromptError> {
        let answer = self
            .ask_credential(CredentialRequestDto::Passphrase {
                alias: request.alias,
                key_path: request.key_path.display().to_string(),
            })
            .await;
        single_secret(answer)
    }

    async fn password(
        &self,
        request: PasswordRequest,
    ) -> Result<Option<Zeroizing<String>>, PromptError> {
        let answer = self
            .ask_credential(CredentialRequestDto::Password {
                alias: request.alias,
                user: request.user,
                host: request.host,
            })
            .await;
        single_secret(answer)
    }

    async fn keyboard_interactive(
        &self,
        request: KeyboardInteractiveRequest,
    ) -> Result<Option<Vec<Zeroizing<String>>>, PromptError> {
        let answer = self
            .ask_credential(CredentialRequestDto::KeyboardInteractive {
                alias: request.alias,
                name: request.name,
                instructions: request.instructions,
                prompts: request
                    .prompts
                    .into_iter()
                    .map(|prompt| PromptDto {
                        text: prompt.text,
                        echo: prompt.echo,
                    })
                    .collect(),
            })
            .await;
        Ok(many_secrets(answer))
    }
}

#[async_trait]
impl HostKeyVerifier for PromptBroker {
    async fn decide_unknown(&self, host_key: UnknownHostKey) -> HostKeyDecision {
        let decision = self
            .ask(&self.host_keys, |prompt_id| {
                EventMessage::HostKeyRequest(HostKeyRequestEvent::Prompt {
                    prompt_id,
                    alias: host_key.alias,
                    lookup_name: host_key.lookup_name,
                    algorithm: host_key.algorithm,
                    fingerprint_sha256: host_key.fingerprint_sha256,
                })
            })
            .await;
        decision.map_or(HostKeyDecision::Reject, host_key_decision_from_dto)
    }

    async fn report_rejected(&self, rejection: RejectedHostKey) {
        tracing::warn!(alias = %rejection.alias, reason = ?rejection.reason, "a host key was refused");
        let _ = self
            .events
            .send(EventMessage::HostKeyRequest(rejection_to_alert(&rejection)));
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use ipc::protocol::{ClientKind, ErrorCode, Secret};
    use ipc::{EventReceiver, IpcClient};
    use pretty_assertions::assert_eq;
    use tokio::task::JoinHandle;
    use tokio::time::timeout;

    use super::*;
    use crate::daemon::testing::TestServer;

    const WAIT: Duration = Duration::from_secs(5);
    const SHORT_TIMEOUT: Duration = Duration::from_millis(100);
    const SECRET_TEXT: &str = "hunter2";

    struct Responder {
        _client: IpcClient,
        events: EventReceiver,
        id: ConnectionId,
    }

    async fn connect_responder(server: &TestServer) -> Responder {
        let mut client = server.connect(ClientKind::Ui, true, true).await;
        let events = client.take_events().unwrap();
        let id = server.handle.connections().last().unwrap().id;
        Responder {
            _client: client,
            events,
            id,
        }
    }

    fn broker(server: &TestServer) -> Arc<PromptBroker> {
        let (events, _) = broadcast::channel(8);
        Arc::new(PromptBroker::new(server.handle.clone(), events))
    }

    fn passphrase_request() -> PassphraseRequest {
        PassphraseRequest {
            alias: "nova".to_owned(),
            key_path: "/home/me/.ssh/id_ed25519".into(),
        }
    }

    fn unknown_host_key() -> UnknownHostKey {
        UnknownHostKey {
            alias: "nova".to_owned(),
            lookup_name: "nova".to_owned(),
            algorithm: "ssh-ed25519".to_owned(),
            fingerprint_sha256: "SHA256:abc".to_owned(),
        }
    }

    fn ask_passphrase(
        broker: &Arc<PromptBroker>,
    ) -> JoinHandle<Result<Option<Zeroizing<String>>, PromptError>> {
        let broker = Arc::clone(broker);
        tokio::spawn(async move { broker.passphrase(passphrase_request()).await })
    }

    async fn next_event(responder: &mut Responder) -> EventMessage {
        timeout(WAIT, responder.events.recv())
            .await
            .unwrap()
            .unwrap()
    }

    async fn received_prompt_id(responder: &mut Responder) -> String {
        match next_event(responder).await {
            EventMessage::CredentialRequest(request) => request.prompt_id,
            EventMessage::HostKeyRequest(HostKeyRequestEvent::Prompt { prompt_id, .. }) => {
                prompt_id
            }
            other => panic!("expected a prompt, got {other:?}"),
        }
    }

    fn answer(prompt_id: &str, text: &str) -> CredentialResponseParams {
        CredentialResponseParams {
            prompt_id: prompt_id.to_owned(),
            answer: Some(CredentialAnswer::Single(Secret::new(text.to_owned()))),
        }
    }

    #[tokio::test]
    async fn should_cancel_a_credential_prompt_when_no_client_can_prompt() {
        let server = TestServer::start();
        let _client = server.connect(ClientKind::Cli, true, false).await;
        let broker = broker(&server);

        let outcome = broker.passphrase(passphrase_request()).await;

        assert_eq!(outcome, Ok(None));
    }

    #[tokio::test]
    async fn should_reject_a_host_key_when_no_client_is_connected() {
        let server = TestServer::start();
        let broker = broker(&server);

        let decision = broker.decide_unknown(unknown_host_key()).await;

        assert_eq!(decision, HostKeyDecision::Reject);
    }

    #[tokio::test]
    async fn should_send_the_request_only_to_clients_that_can_prompt() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let mut bystander = server.connect(ClientKind::Cli, true, false).await;
        let mut bystander_events = bystander.take_events().unwrap();
        let broker = broker(&server);

        let asking = ask_passphrase(&broker);
        let prompt_id = received_prompt_id(&mut responder).await;
        broker
            .respond_credential(None, answer(&prompt_id, SECRET_TEXT))
            .unwrap();
        asking.await.unwrap().unwrap();

        assert!(
            timeout(Duration::from_millis(200), bystander_events.recv())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn should_return_the_secret_of_the_first_responder() {
        let server = TestServer::start();
        let mut first = connect_responder(&server).await;
        let mut second = connect_responder(&server).await;
        let broker = broker(&server);

        let asking = ask_passphrase(&broker);
        let prompt_id = received_prompt_id(&mut first).await;
        received_prompt_id(&mut second).await;
        broker
            .respond_credential(Some(first.id), answer(&prompt_id, SECRET_TEXT))
            .unwrap();

        let secret = asking.await.unwrap().unwrap().unwrap();
        assert_eq!(secret.as_str(), SECRET_TEXT);
    }

    #[tokio::test]
    async fn should_tell_the_other_responders_when_a_prompt_is_answered() {
        let server = TestServer::start();
        let mut first = connect_responder(&server).await;
        let mut second = connect_responder(&server).await;
        let broker = broker(&server);

        let asking = ask_passphrase(&broker);
        let prompt_id = received_prompt_id(&mut first).await;
        received_prompt_id(&mut second).await;
        broker
            .respond_credential(Some(first.id), answer(&prompt_id, SECRET_TEXT))
            .unwrap();
        asking.await.unwrap().unwrap();

        assert_eq!(
            next_event(&mut second).await,
            EventMessage::PromptResolved(PromptResolvedEvent { prompt_id })
        );
    }

    #[tokio::test]
    async fn should_not_tell_the_answerer_that_its_own_prompt_is_resolved() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let broker = broker(&server);

        let asking = ask_passphrase(&broker);
        let prompt_id = received_prompt_id(&mut responder).await;
        broker
            .respond_credential(Some(responder.id), answer(&prompt_id, SECRET_TEXT))
            .unwrap();
        asking.await.unwrap().unwrap();

        assert!(
            timeout(Duration::from_millis(200), responder.events.recv())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn should_answer_prompt_not_pending_when_the_prompt_is_unknown() {
        let server = TestServer::start();
        let broker = broker(&server);

        let error = broker
            .respond_credential(None, answer("missing", SECRET_TEXT))
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::PromptNotPending);
    }

    #[tokio::test]
    async fn should_answer_prompt_not_pending_when_the_prompt_is_already_resolved() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let broker = broker(&server);
        let asking = ask_passphrase(&broker);
        let prompt_id = received_prompt_id(&mut responder).await;
        broker
            .respond_credential(None, answer(&prompt_id, SECRET_TEXT))
            .unwrap();
        asking.await.unwrap().unwrap();

        let error = broker
            .respond_credential(None, answer(&prompt_id, "late"))
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::PromptNotPending);
    }

    #[tokio::test]
    async fn should_answer_prompt_not_pending_when_a_host_key_answer_targets_a_credential_prompt() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let broker = broker(&server);
        let asking = ask_passphrase(&broker);
        let prompt_id = received_prompt_id(&mut responder).await;

        let error = broker
            .respond_host_key(
                None,
                &HostKeyResponseParams {
                    prompt_id,
                    decision: HostKeyDecisionDto::AcceptOnce,
                },
            )
            .unwrap_err();

        asking.abort();
        assert_eq!(error.code, ErrorCode::PromptNotPending);
    }

    #[tokio::test]
    async fn should_cancel_the_prompt_when_nobody_answers_before_the_timeout() {
        let server = TestServer::start();
        let _responder = connect_responder(&server).await;
        let broker = Arc::new(
            PromptBroker::new(server.handle.clone(), broadcast::channel(8).0)
                .with_timeout(SHORT_TIMEOUT),
        );

        let outcome = broker.passphrase(passphrase_request()).await;

        assert_eq!(outcome, Ok(None));
    }

    #[tokio::test]
    async fn should_tell_the_responders_when_a_prompt_times_out() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let broker = Arc::new(
            PromptBroker::new(server.handle.clone(), broadcast::channel(8).0)
                .with_timeout(SHORT_TIMEOUT),
        );

        broker.passphrase(passphrase_request()).await.unwrap();

        let prompt_id = received_prompt_id(&mut responder).await;
        assert_eq!(
            next_event(&mut responder).await,
            EventMessage::PromptResolved(PromptResolvedEvent { prompt_id })
        );
    }

    #[tokio::test]
    async fn should_cancel_the_prompt_when_every_responder_disconnects() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let broker = broker(&server);
        let asking = ask_passphrase(&broker);
        received_prompt_id(&mut responder).await;

        drop(responder);

        let outcome = timeout(WAIT, asking).await.unwrap().unwrap();
        assert_eq!(outcome, Ok(None));
    }

    #[tokio::test]
    async fn should_keep_waiting_when_only_some_responders_disconnect() {
        let server = TestServer::start();
        let leaving = connect_responder(&server).await;
        let mut staying = connect_responder(&server).await;
        let broker = broker(&server);
        let asking = ask_passphrase(&broker);
        let prompt_id = received_prompt_id(&mut staying).await;

        drop(leaving);
        broker
            .respond_credential(Some(staying.id), answer(&prompt_id, SECRET_TEXT))
            .unwrap();

        let secret = asking.await.unwrap().unwrap().unwrap();
        assert_eq!(secret.as_str(), SECRET_TEXT);
    }

    #[tokio::test]
    async fn should_return_one_answer_per_question_when_asking_keyboard_interactive() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let broker = broker(&server);
        let asking = {
            let broker = Arc::clone(&broker);
            tokio::spawn(async move {
                broker
                    .keyboard_interactive(KeyboardInteractiveRequest {
                        alias: "nova".to_owned(),
                        name: String::new(),
                        instructions: String::new(),
                        prompts: vec![engine::Prompt {
                            text: "Code:".to_owned(),
                            echo: false,
                        }],
                    })
                    .await
            })
        };
        let prompt_id = received_prompt_id(&mut responder).await;

        broker
            .respond_credential(
                None,
                CredentialResponseParams {
                    prompt_id,
                    answer: Some(CredentialAnswer::Multiple(vec![Secret::new(
                        "123456".to_owned(),
                    )])),
                },
            )
            .unwrap();

        let answers = asking.await.unwrap().unwrap().unwrap();
        assert_eq!(answers.len(), 1);
    }

    #[tokio::test]
    async fn should_fail_when_a_passphrase_prompt_gets_several_secrets() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let broker = broker(&server);
        let asking = ask_passphrase(&broker);
        let prompt_id = received_prompt_id(&mut responder).await;

        broker
            .respond_credential(
                None,
                CredentialResponseParams {
                    prompt_id,
                    answer: Some(CredentialAnswer::Multiple(vec![
                        Secret::new("a".to_owned()),
                        Secret::new("b".to_owned()),
                    ])),
                },
            )
            .unwrap();

        assert_eq!(
            asking.await.unwrap().unwrap_err(),
            PromptError::AnswerCountMismatch {
                expected: 1,
                actual: 2
            }
        );
    }

    #[tokio::test]
    async fn should_return_the_decision_of_the_responder_when_asking_about_a_host_key() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let broker = broker(&server);
        let asking = {
            let broker = Arc::clone(&broker);
            tokio::spawn(async move { broker.decide_unknown(unknown_host_key()).await })
        };
        let prompt_id = received_prompt_id(&mut responder).await;

        broker
            .respond_host_key(
                None,
                &HostKeyResponseParams {
                    prompt_id,
                    decision: HostKeyDecisionDto::AcceptAndSave,
                },
            )
            .unwrap();

        assert_eq!(asking.await.unwrap(), HostKeyDecision::AcceptAndSave);
    }

    #[tokio::test]
    async fn should_describe_the_host_key_in_the_request() {
        let server = TestServer::start();
        let mut responder = connect_responder(&server).await;
        let broker = broker(&server);
        let asking = {
            let broker = Arc::clone(&broker);
            tokio::spawn(async move { broker.decide_unknown(unknown_host_key()).await })
        };

        let event = next_event(&mut responder).await;

        asking.abort();
        assert!(matches!(
            event,
            EventMessage::HostKeyRequest(HostKeyRequestEvent::Prompt {
                ref fingerprint_sha256, ..
            }) if fingerprint_sha256 == "SHA256:abc"
        ));
    }

    #[tokio::test]
    async fn should_broadcast_an_alert_when_a_host_key_is_rejected() {
        let server = TestServer::start();
        let (events, mut alerts) = broadcast::channel(8);
        let broker = PromptBroker::new(server.handle.clone(), events);

        broker
            .report_rejected(RejectedHostKey {
                alias: "nova".to_owned(),
                reason: engine::RejectionReason::UnknownRejectedByPolicy,
                presented_fingerprint: "SHA256:abc".to_owned(),
            })
            .await;

        assert!(matches!(
            alerts.try_recv().unwrap(),
            EventMessage::HostKeyRequest(HostKeyRequestEvent::Alert { .. })
        ));
    }
}
