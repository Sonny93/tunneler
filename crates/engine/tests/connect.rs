#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::{
    AuthenticationMethod, ConnectError, Connector, HostKeyDecision, HostKeyFailure,
    RejectionReason, SshSession, TransportError,
};
use pretty_assertions::assert_eq;
use russh::MethodKind;
use russh::keys::{Algorithm, PrivateKey};
use support::{
    ALIAS, EchoServer, ScriptedPrompter, ScriptedVerifier, SilentListener, TestServer, TestUser,
    Workspace, fingerprint_of, fixture_key, generate_key, write_private_key,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const PASSWORD: &str = "s3cret";
const PASSPHRASE: &str = "correct horse";
const KEYBOARD_QUESTION: &str = "Code: ";
const KEYBOARD_ANSWER: &str = "123456";
const GENEROUS_TIMEOUT_SECONDS: &str = "ConnectTimeout 1";
const TIMEOUT_BOUND: Duration = Duration::from_secs(3);

struct Scenario {
    workspace: Workspace,
    server: TestServer,
    prompter: Arc<ScriptedPrompter>,
    verifier: Arc<ScriptedVerifier>,
    connector: Connector,
}

struct ScenarioBuilder {
    workspace: Workspace,
    server: TestServer,
    prompter: ScriptedPrompter,
    verifier: ScriptedVerifier,
    config_lines: Vec<String>,
}

impl ScenarioBuilder {
    fn new(server: TestServer, workspace: Workspace) -> Self {
        Self {
            workspace,
            server,
            prompter: ScriptedPrompter::default(),
            verifier: ScriptedVerifier::default(),
            config_lines: Vec::new(),
        }
    }

    fn config(mut self, line: &str) -> Self {
        self.config_lines.push(line.to_owned());
        self
    }

    fn identity_file(self, path: &std::path::Path) -> Self {
        self.config(&format!("IdentityFile {}", path.display()))
    }

    fn prompter(mut self, prompter: ScriptedPrompter) -> Self {
        self.prompter = prompter;
        self
    }

    fn verifier(mut self, verifier: ScriptedVerifier) -> Self {
        self.verifier = verifier;
        self
    }

    fn build(self) -> Scenario {
        let lines: Vec<&str> = self.config_lines.iter().map(String::as_str).collect();
        let config_path = self.workspace.write_config(self.server.port, &lines);
        let prompter = Arc::new(self.prompter);
        let verifier = Arc::new(self.verifier);
        let connector = self.workspace.connector(
            &config_path,
            Arc::clone(&prompter) as _,
            Arc::clone(&verifier) as _,
        );
        Scenario {
            workspace: self.workspace,
            server: self.server,
            prompter,
            verifier,
            connector,
        }
    }
}

impl Scenario {
    async fn connect(&self) -> Result<SshSession, ConnectError> {
        self.connector.connect(ALIAS).await
    }

    async fn connect_successfully(&self) -> SshSession {
        self.connect().await.expect("the connection should succeed")
    }

    async fn connect_failing(&self) -> ConnectError {
        self.connect()
            .await
            .expect_err("the connection should fail")
    }

    fn lookup_name(&self) -> String {
        format!("[127.0.0.1]:{}", self.server.port)
    }
}

fn user_with_key(key: &PrivateKey) -> TestUser {
    TestUser {
        authorized_keys: vec![key.public_key().clone()],
        ..TestUser::default()
    }
}

fn user_with_password() -> TestUser {
    TestUser {
        password: Some(PASSWORD.to_owned()),
        ..TestUser::default()
    }
}

async fn server_for(user: TestUser) -> TestServer {
    TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(support::REMOTE_USER, user)
        .start()
        .await
}

async fn password_only_server() -> TestServer {
    TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(support::REMOTE_USER, user_with_password())
        .methods(&[MethodKind::Password])
        .start()
        .await
}

fn tried_methods(error: &ConnectError) -> Vec<AuthenticationMethod> {
    match error {
        ConnectError::Authentication { tried } => tried.clone(),
        other => panic!("expected an authentication failure, got {other:?}"),
    }
}

#[tokio::test]
async fn should_authenticate_with_key_when_the_server_authorizes_it() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let key_path = write_private_key(workspace.path(), "id_test", &key, None);
    let scenario = ScenarioBuilder::new(server_for(user_with_key(&key)).await, workspace)
        .identity_file(&key_path)
        .build();

    let session = scenario.connect_successfully().await;

    assert_eq!(session.alias(), ALIAS);
    assert_eq!(session.resolved().port, scenario.server.port);
    assert_eq!(scenario.prompter.request_count(), 0);
    session.disconnect().await.unwrap();
}

#[tokio::test]
async fn should_ask_the_passphrase_when_the_key_is_encrypted() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let key_path = write_private_key(workspace.path(), "id_test", &key, Some(PASSPHRASE));
    let scenario = ScenarioBuilder::new(server_for(user_with_key(&key)).await, workspace)
        .identity_file(&key_path)
        .prompter(ScriptedPrompter::default().with_passphrases(&[Some(PASSPHRASE)]))
        .build();

    scenario.connect_successfully().await;

    let requests = scenario.prompter.passphrase_requests();
    assert_eq!(
        (
            requests.len(),
            requests[0].key_path.clone(),
            requests[0].alias.clone()
        ),
        (1, key_path, ALIAS.to_owned())
    );
}

#[tokio::test]
async fn should_fall_through_to_password_when_the_passphrase_prompt_is_cancelled() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let key_path = write_private_key(workspace.path(), "id_test", &key, Some(PASSPHRASE));
    let user = TestUser {
        password: Some(PASSWORD.to_owned()),
        ..user_with_key(&key)
    };
    let scenario = ScenarioBuilder::new(server_for(user).await, workspace)
        .identity_file(&key_path)
        .prompter(
            ScriptedPrompter::default()
                .with_passphrases(&[None])
                .with_passwords(&[Some(PASSWORD)]),
        )
        .build();

    scenario.connect_successfully().await;

    assert_eq!(scenario.prompter.password_requests().len(), 1);
}

#[tokio::test]
async fn should_ask_again_when_the_first_passphrase_is_wrong() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let key_path = write_private_key(workspace.path(), "id_test", &key, Some(PASSPHRASE));
    let scenario = ScenarioBuilder::new(server_for(user_with_key(&key)).await, workspace)
        .identity_file(&key_path)
        .prompter(ScriptedPrompter::default().with_passphrases(&[Some("wrong"), Some(PASSPHRASE)]))
        .build();

    scenario.connect_successfully().await;

    assert_eq!(scenario.prompter.passphrase_requests().len(), 2);
}

#[tokio::test]
async fn should_authenticate_with_password_when_the_user_types_it() {
    let scenario = ScenarioBuilder::new(server_for(user_with_password()).await, Workspace::new())
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .build();

    scenario.connect_successfully().await;

    let requests = scenario.prompter.password_requests();
    assert_eq!(
        (requests[0].user.as_str(), requests[0].host.as_str()),
        (support::REMOTE_USER, "127.0.0.1")
    );
}

#[tokio::test]
async fn should_list_tried_methods_and_stop_asking_the_password_when_every_password_is_wrong() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("NumberOfPasswordPrompts 2")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some("a"), Some("b"), Some("c")]))
        .build();

    let error = scenario.connect_failing().await;

    assert_eq!(tried_methods(&error), vec![AuthenticationMethod::Password]);
    assert_eq!(scenario.prompter.password_requests().len(), 2);
}

#[tokio::test]
async fn should_authenticate_with_keyboard_interactive_when_the_answers_are_right() {
    let user = TestUser {
        keyboard_interactive: vec![(KEYBOARD_QUESTION.to_owned(), KEYBOARD_ANSWER.to_owned())],
        ..TestUser::default()
    };
    let scenario = ScenarioBuilder::new(server_for(user).await, Workspace::new())
        .prompter(
            ScriptedPrompter::default().with_keyboard_interactive(&[Some(&[KEYBOARD_ANSWER])]),
        )
        .build();

    scenario.connect_successfully().await;

    let requests = scenario.prompter.keyboard_interactive_requests();
    let prompts: Vec<(String, bool)> = requests[0]
        .prompts
        .iter()
        .map(|prompt| (prompt.text.clone(), prompt.echo))
        .collect();
    assert_eq!(prompts, vec![(KEYBOARD_QUESTION.to_owned(), false)]);
}

#[tokio::test]
async fn should_skip_the_key_when_preferred_authentications_is_password() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let key_path = write_private_key(workspace.path(), "id_test", &key, Some(PASSPHRASE));
    let user = TestUser {
        password: Some(PASSWORD.to_owned()),
        ..user_with_key(&key)
    };
    let scenario = ScenarioBuilder::new(server_for(user).await, workspace)
        .identity_file(&key_path)
        .config("PreferredAuthentications password")
        .prompter(
            ScriptedPrompter::default()
                .with_passphrases(&[Some(PASSPHRASE)])
                .with_passwords(&[Some(PASSWORD)]),
        )
        .build();

    scenario.connect_successfully().await;

    assert_eq!(scenario.prompter.passphrase_requests().len(), 0);
}

#[tokio::test]
async fn should_complete_publickey_then_password_when_the_server_requires_both() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let key_path = write_private_key(workspace.path(), "id_test", &key, None);
    let server = TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(
            support::REMOTE_USER,
            TestUser {
                password: Some(PASSWORD.to_owned()),
                ..user_with_key(&key)
            },
        )
        .methods(&[MethodKind::PublicKey, MethodKind::Password])
        .requiring_publickey_then_password()
        .start()
        .await;
    let scenario = ScenarioBuilder::new(server, workspace)
        .identity_file(&key_path)
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .build();

    scenario.connect_successfully().await;

    assert_eq!(scenario.prompter.password_requests().len(), 1);
}

#[tokio::test]
async fn should_ask_then_record_the_key_when_the_user_accepts_and_saves() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("StrictHostKeyChecking ask")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD), Some(PASSWORD)]))
        .verifier(ScriptedVerifier::default().with_decisions(&[HostKeyDecision::AcceptAndSave]))
        .build();

    scenario.connect_successfully().await;

    let requests = scenario.verifier.unknown_requests();
    assert_eq!(
        (
            requests[0].lookup_name.clone(),
            requests[0].algorithm.clone(),
            requests[0].fingerprint_sha256.clone(),
            requests[0].alias.clone(),
        ),
        (
            scenario.lookup_name(),
            "ssh-ed25519".to_owned(),
            fingerprint_of(&scenario.server.public_host_key(0)),
            ALIAS.to_owned(),
        )
    );
    assert!(
        scenario
            .workspace
            .known_hosts_content()
            .starts_with(&format!("{} ssh-ed25519 ", scenario.lookup_name()))
    );
}

#[tokio::test]
async fn should_not_ask_again_when_reconnecting_after_accept_and_save() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("StrictHostKeyChecking ask")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD), Some(PASSWORD)]))
        .verifier(ScriptedVerifier::default().with_decisions(&[HostKeyDecision::AcceptAndSave]))
        .build();
    scenario.connect_successfully().await;

    scenario.connect_successfully().await;

    assert_eq!(scenario.verifier.unknown_requests().len(), 1);
}

#[tokio::test]
async fn should_not_record_the_key_when_the_user_accepts_once() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("StrictHostKeyChecking ask")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .verifier(ScriptedVerifier::default().with_decisions(&[HostKeyDecision::AcceptOnce]))
        .build();

    scenario.connect_successfully().await;

    assert_eq!(scenario.workspace.known_hosts_content(), "");
}

#[tokio::test]
async fn should_not_ask_again_when_reconnecting_after_accept_once() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("StrictHostKeyChecking ask")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD), Some(PASSWORD)]))
        .verifier(ScriptedVerifier::default().with_decisions(&[HostKeyDecision::AcceptOnce]))
        .build();
    scenario.connect_successfully().await;

    scenario.connect_successfully().await;

    assert_eq!(scenario.verifier.unknown_requests().len(), 1);
}

#[tokio::test]
async fn should_fail_when_the_user_rejects_the_unknown_key() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("StrictHostKeyChecking ask")
        .verifier(ScriptedVerifier::default().with_decisions(&[HostKeyDecision::Reject]))
        .build();

    let error = scenario.connect_failing().await;

    assert!(matches!(
        error,
        ConnectError::HostKey(HostKeyFailure::RejectedByUser { .. })
    ));
}

#[tokio::test]
async fn should_reject_the_unknown_key_without_asking_when_strict_checking_is_yes() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("StrictHostKeyChecking yes")
        .build();

    let error = scenario.connect_failing().await;

    assert!(matches!(
        error,
        ConnectError::HostKey(HostKeyFailure::UnknownRejectedByPolicy { .. })
    ));
    assert_eq!(scenario.verifier.unknown_requests().len(), 0);
    assert_eq!(
        scenario.verifier.rejections()[0].reason,
        RejectionReason::UnknownRejectedByPolicy
    );
}

#[tokio::test]
async fn should_reject_the_unknown_key_when_ask_is_combined_with_batch_mode() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("StrictHostKeyChecking ask")
        .config("BatchMode yes")
        .build();

    let error = scenario.connect_failing().await;

    assert!(matches!(
        error,
        ConnectError::HostKey(HostKeyFailure::UnknownRejectedByPolicy { .. })
    ));
    assert_eq!(scenario.verifier.unknown_requests().len(), 0);
}

#[tokio::test]
async fn should_record_the_key_without_asking_when_strict_checking_is_accept_new() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("StrictHostKeyChecking accept-new")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .build();

    scenario.connect_successfully().await;

    assert_eq!(scenario.verifier.unknown_requests().len(), 0);
    assert!(
        scenario
            .workspace
            .known_hosts_content()
            .starts_with(&scenario.lookup_name())
    );
}

#[tokio::test]
async fn should_refuse_a_changed_key_even_when_strict_checking_is_no() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("StrictHostKeyChecking no")
        .build();
    let recorded = generate_key(Algorithm::Ed25519);
    scenario
        .workspace
        .seed_known_hosts(&scenario.lookup_name(), recorded.public_key());

    let error = scenario.connect_failing().await;

    assert!(matches!(
        error,
        ConnectError::HostKey(HostKeyFailure::Changed { line: 1, .. })
    ));
}

#[tokio::test]
async fn should_report_the_rejection_when_the_key_changed() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new()).build();
    let recorded = generate_key(Algorithm::Ed25519);
    scenario
        .workspace
        .seed_known_hosts(&scenario.lookup_name(), recorded.public_key());

    scenario.connect_failing().await;

    let rejections = scenario.verifier.rejections();
    assert_eq!(
        (
            rejections.len(),
            rejections[0].presented_fingerprint.clone()
        ),
        (1, fingerprint_of(&scenario.server.public_host_key(0)))
    );
    assert!(matches!(
        &rejections[0].reason,
        RejectionReason::Changed { known_fingerprint, line: 1, .. }
            if *known_fingerprint == fingerprint_of(recorded.public_key())
    ));
}

#[tokio::test]
async fn should_refuse_a_revoked_key_even_when_strict_checking_is_no() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new()).build();
    scenario
        .workspace
        .seed_revoked_key(&scenario.lookup_name(), &scenario.server.public_host_key(0));

    let error = scenario.connect_failing().await;

    assert!(matches!(
        error,
        ConnectError::HostKey(HostKeyFailure::Revoked { line: 1, .. })
    ));
    assert!(matches!(
        scenario.verifier.rejections()[0].reason,
        RejectionReason::Revoked { line: 1, .. }
    ));
}

#[tokio::test]
async fn should_write_a_hashed_entry_when_hash_known_hosts_is_yes() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("HashKnownHosts yes")
        .config("StrictHostKeyChecking accept-new")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD), Some(PASSWORD)]))
        .build();

    scenario.connect_successfully().await;

    let content = scenario.workspace.known_hosts_content();
    assert!(content.starts_with("|1|") && !content.contains("127.0.0.1"));
}

#[tokio::test]
async fn should_find_a_hashed_entry_when_reconnecting() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("HashKnownHosts yes")
        .config("StrictHostKeyChecking accept-new")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD), Some(PASSWORD)]))
        .build();
    scenario.connect_successfully().await;
    let strict = scenario
        .workspace
        .write_config(scenario.server.port, &["StrictHostKeyChecking yes"]);
    let strict_connector = scenario.workspace.connector(
        &strict,
        Arc::clone(&scenario.prompter) as _,
        Arc::clone(&scenario.verifier) as _,
    );

    let outcome = strict_connector.connect(ALIAS).await;

    assert!(outcome.is_ok());
}

#[tokio::test]
async fn should_record_and_look_up_the_host_key_alias_when_one_is_configured() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config("HostKeyAlias vault-main")
        .config("StrictHostKeyChecking ask")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .verifier(ScriptedVerifier::default().with_decisions(&[HostKeyDecision::AcceptAndSave]))
        .build();

    scenario.connect_successfully().await;

    assert_eq!(
        scenario.verifier.unknown_requests()[0].lookup_name,
        "vault-main"
    );
    assert!(
        scenario
            .workspace
            .known_hosts_content()
            .starts_with("vault-main ssh-ed25519 ")
    );
}

#[tokio::test]
async fn should_trust_the_recorded_rsa_key_when_the_server_also_has_an_ed25519_key() {
    let server = TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .host_key(fixture_key("rsa_host_key"))
        .user(support::REMOTE_USER, user_with_password())
        .methods(&[MethodKind::Password])
        .start()
        .await;
    let scenario = ScenarioBuilder::new(server, Workspace::new())
        .config("StrictHostKeyChecking yes")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .build();
    scenario
        .workspace
        .seed_known_hosts(&scenario.lookup_name(), &scenario.server.public_host_key(1));

    let outcome = scenario.connect().await;

    assert!(outcome.is_ok(), "{outcome:?}");
}

#[tokio::test]
async fn should_time_out_when_the_server_never_speaks() {
    let silent = SilentListener::start().await;
    let workspace = Workspace::new();
    let config_path = workspace.write_config(silent.port, &[GENEROUS_TIMEOUT_SECONDS]);
    let connector = workspace.connector(
        &config_path,
        support::quiet_prompter(),
        support::rejecting_verifier(),
    );
    let started = Instant::now();

    let outcome = connector.connect(ALIAS).await;

    assert!(matches!(
        outcome,
        Err(ConnectError::HandshakeTimeout { .. })
    ));
    assert!(started.elapsed() < TIMEOUT_BOUND);
}

#[tokio::test]
async fn should_not_count_the_time_spent_deciding_about_the_host_key_against_connect_timeout() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .config(GENEROUS_TIMEOUT_SECONDS)
        .config("StrictHostKeyChecking ask")
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .verifier(
            ScriptedVerifier::default()
                .with_decisions(&[HostKeyDecision::AcceptOnce])
                .with_decision_delay(Duration::from_millis(1500)),
        )
        .build();

    let outcome = scenario.connect().await;

    assert!(outcome.is_ok(), "{outcome:?}");
}

#[tokio::test]
async fn should_report_the_address_when_nothing_listens() {
    let workspace = Workspace::new();
    let closed_port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    };
    let config_path = workspace.write_config(closed_port, &[]);
    let connector = workspace.connector(
        &config_path,
        support::quiet_prompter(),
        support::rejecting_verifier(),
    );

    let outcome = connector.connect(ALIAS).await;

    assert!(matches!(
        outcome,
        Err(ConnectError::Transport(TransportError::Connect { address, .. }))
            if address.port() == closed_port
    ));
}

#[tokio::test]
async fn should_never_prompt_when_batch_mode_is_yes() {
    let key = generate_key(Algorithm::Ed25519);
    let workspace = Workspace::new();
    let key_path = write_private_key(workspace.path(), "id_test", &key, Some(PASSPHRASE));
    let user = TestUser {
        password: Some(PASSWORD.to_owned()),
        keyboard_interactive: vec![(KEYBOARD_QUESTION.to_owned(), KEYBOARD_ANSWER.to_owned())],
        ..user_with_key(&key)
    };
    let scenario = ScenarioBuilder::new(server_for(user).await, workspace)
        .identity_file(&key_path)
        .config("BatchMode yes")
        .prompter(
            ScriptedPrompter::default()
                .with_passphrases(&[Some(PASSPHRASE)])
                .with_passwords(&[Some(PASSWORD)])
                .with_keyboard_interactive(&[Some(&[KEYBOARD_ANSWER])]),
        )
        .build();

    let error = scenario.connect_failing().await;

    assert!(matches!(error, ConnectError::Authentication { .. }));
    assert_eq!(scenario.prompter.request_count(), 0);
}

#[tokio::test]
async fn should_round_trip_bytes_when_opening_a_direct_tcpip_channel() {
    let echo = EchoServer::start().await;
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .build();
    let session = scenario.connect_successfully().await;
    let mut stream = session
        .open_direct_tcpip("127.0.0.1", echo.port, None)
        .await
        .unwrap();

    stream.write_all(b"hello tunnel").await.unwrap();
    let mut received = [0_u8; 12];
    stream.read_exact(&mut received).await.unwrap();

    assert_eq!(&received, b"hello tunnel");
}

#[tokio::test]
async fn should_authenticate_with_an_rsa_key_when_the_server_authorizes_it() {
    let key = fixture_key("rsa_client_key");
    let workspace = Workspace::new();
    let key_path = write_private_key(workspace.path(), "id_rsa_test", &key, None);
    let scenario = ScenarioBuilder::new(server_for(user_with_key(&key)).await, workspace)
        .identity_file(&key_path)
        .build();

    let outcome = scenario.connect().await;

    assert!(outcome.is_ok(), "{outcome:?}");
}

#[tokio::test]
async fn should_connect_on_the_second_round_when_the_server_comes_up_during_the_pause() {
    let reserved = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = reserved.local_addr().unwrap().port();
    drop(reserved);
    let workspace = Workspace::new();
    let config_path = workspace.write_config(port, &["ConnectionAttempts 2"]);
    let prompter = Arc::new(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]));
    let connector = workspace.connector(&config_path, prompter, support::rejecting_verifier());
    let late_server = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        TestServer::builder()
            .host_key(generate_key(Algorithm::Ed25519))
            .user(support::REMOTE_USER, user_with_password())
            .methods(&[MethodKind::Password])
            .on_port(port)
            .start()
            .await
    });

    let outcome = connector.connect(ALIAS).await;

    let _server = late_server.await.unwrap();
    assert!(outcome.is_ok(), "{outcome:?}");
}

#[tokio::test]
async fn should_pause_between_rounds_when_connection_attempts_is_two_and_nothing_listens() {
    let workspace = Workspace::new();
    let closed_port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    };
    let config_path = workspace.write_config(closed_port, &["ConnectionAttempts 2"]);
    let connector = workspace.connector(
        &config_path,
        support::quiet_prompter(),
        support::rejecting_verifier(),
    );
    let started = Instant::now();

    let outcome = connector.connect(ALIAS).await;

    assert!(matches!(outcome, Err(ConnectError::Transport(_))));
    assert!(started.elapsed() >= Duration::from_secs(1));
}

#[tokio::test]
async fn should_resolve_closed_when_the_server_cuts_the_connection() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .build();
    let session = scenario.connect_successfully().await;

    scenario.server.cut_connections();

    let outcome = tokio::time::timeout(TIMEOUT_BOUND, session.closed()).await;
    assert!(outcome.is_ok());
}

#[tokio::test]
async fn should_not_resolve_closed_when_the_session_is_alive() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .build();
    let session = scenario.connect_successfully().await;

    let outcome = tokio::time::timeout(Duration::from_millis(200), session.closed()).await;

    assert!(outcome.is_err());
}

#[tokio::test]
async fn should_resolve_closed_when_the_client_disconnects() {
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .build();
    let session = scenario.connect_successfully().await;

    session.disconnect().await.unwrap();

    let outcome = tokio::time::timeout(TIMEOUT_BOUND, session.closed()).await;
    assert!(outcome.is_ok());
}

#[tokio::test]
async fn should_report_the_originator_when_opening_a_direct_tcpip_channel_for_a_client() {
    let echo = EchoServer::start().await;
    let scenario = ScenarioBuilder::new(password_only_server().await, Workspace::new())
        .prompter(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]))
        .build();
    let session = scenario.connect_successfully().await;
    let originator = std::net::SocketAddr::from(([127, 0, 0, 1], 40_000));

    session
        .open_direct_tcpip("127.0.0.1", echo.port, Some(originator))
        .await
        .unwrap();

    assert_eq!(
        scenario.server.direct_tcpip_originators(),
        vec![("127.0.0.1".to_owned(), 40_000)]
    );
}
