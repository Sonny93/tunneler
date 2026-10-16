#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::{ConnectError, Connector, SshSession};
use pretty_assertions::assert_eq;
use russh::MethodKind;
use russh::keys::{Algorithm, PrivateKey};
use support::{
    ALIAS, FixedAgentLocator, ScriptedPrompter, TestAgent, TestServer, TestUser, Workspace,
    fingerprint_of, generate_key, write_private_key,
};

const PASSPHRASE: &str = "correct horse";

struct Scenario {
    workspace: Workspace,
    server: TestServer,
    agent: TestAgent,
    prompter: Arc<ScriptedPrompter>,
}

impl Scenario {
    async fn new(authorized: Option<&PrivateKey>) -> Self {
        let user = TestUser {
            authorized_keys: authorized
                .map(|key| key.public_key().clone())
                .into_iter()
                .collect(),
            ..TestUser::default()
        };
        let server = TestServer::builder()
            .host_key(generate_key(Algorithm::Ed25519))
            .user(support::REMOTE_USER, user)
            .methods(&[MethodKind::PublicKey])
            .start()
            .await;
        let workspace = Workspace::new();
        let agent = TestAgent::start(workspace.path());
        Self {
            workspace,
            server,
            agent,
            prompter: Arc::new(ScriptedPrompter::default().with_passphrases(&[Some(PASSPHRASE)])),
        }
    }

    fn connector(&self, config_lines: &[&str]) -> Connector {
        let locator = FixedAgentLocator {
            default_socket: Some(self.agent.socket_path.clone()),
        };
        self.connector_with(config_lines, Arc::new(locator))
    }

    fn connector_with(
        &self,
        config_lines: &[&str],
        locator: Arc<dyn engine::AgentLocator>,
    ) -> Connector {
        let config_path = self.workspace.write_config(self.server.port, config_lines);
        self.workspace.connector_with_agent(
            &config_path,
            Arc::clone(&self.prompter) as _,
            support::rejecting_verifier(),
            locator,
        )
    }

    fn write_key_file(&self, key: &PrivateKey, passphrase: Option<&str>) -> PathBuf {
        write_private_key(self.workspace.path(), "id_test", key, passphrase)
    }
}

fn write_public_sibling(private_path: &Path, key: &PrivateKey) {
    let mut sibling = private_path.as_os_str().to_owned();
    sibling.push(".pub");
    std::fs::write(sibling, key.public_key().to_openssh().unwrap()).unwrap();
}

async fn connect(connector: &Connector) -> Result<SshSession, ConnectError> {
    connector.connect(ALIAS).await
}

#[tokio::test]
async fn should_authenticate_with_an_agent_key_when_there_is_no_identity_file() {
    let key = generate_key(Algorithm::Ed25519);
    let scenario = Scenario::new(Some(&key)).await;
    scenario.agent.add_key(&key).await;

    let outcome = connect(&scenario.connector(&[])).await;

    assert!(outcome.is_ok(), "{outcome:?}");
}

#[tokio::test]
async fn should_authenticate_with_the_agent_socket_named_by_identity_agent() {
    let key = generate_key(Algorithm::Ed25519);
    let scenario = Scenario::new(Some(&key)).await;
    scenario.agent.add_key(&key).await;
    let directive = format!("IdentityAgent {}", scenario.agent.socket_path.display());

    let locator = FixedAgentLocator {
        default_socket: None,
    };

    let connector = scenario.connector_with(&[directive.as_str()], Arc::new(locator));

    assert!(connect(&connector).await.is_ok());
}

#[tokio::test]
async fn should_not_use_the_agent_when_identity_agent_is_none() {
    let key = generate_key(Algorithm::Ed25519);
    let scenario = Scenario::new(Some(&key)).await;
    scenario.agent.add_key(&key).await;

    let outcome = connect(&scenario.connector(&["IdentityAgent none"])).await;

    assert!(matches!(outcome, Err(ConnectError::Authentication { .. })));
    assert_eq!(scenario.server.offered_fingerprints(), Vec::<String>::new());
}

#[tokio::test]
async fn should_not_offer_agent_keys_that_match_no_identity_file_when_identities_only_is_yes() {
    let key = generate_key(Algorithm::Ed25519);
    let scenario = Scenario::new(Some(&key)).await;
    scenario.agent.add_key(&key).await;

    let outcome = connect(&scenario.connector(&["IdentitiesOnly yes"])).await;

    assert!(matches!(outcome, Err(ConnectError::Authentication { .. })));
    assert_eq!(scenario.server.offered_fingerprints(), Vec::<String>::new());
}

#[tokio::test]
async fn should_use_the_agent_key_that_matches_an_identity_file_when_identities_only_is_yes() {
    let key = generate_key(Algorithm::Ed25519);
    let scenario = Scenario::new(Some(&key)).await;
    scenario.agent.add_key(&key).await;
    let key_path = scenario.write_key_file(&key, Some(PASSPHRASE));
    write_public_sibling(&key_path, &key);
    let identity = format!("IdentityFile {}", key_path.display());

    let outcome = connect(&scenario.connector(&["IdentitiesOnly yes", identity.as_str()])).await;

    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(scenario.prompter.passphrase_requests().len(), 0);
}

#[tokio::test]
async fn should_offer_an_identity_once_when_the_agent_and_a_file_hold_it() {
    let key = generate_key(Algorithm::Ed25519);
    let scenario = Scenario::new(None).await;
    scenario.agent.add_key(&key).await;
    let key_path = scenario.write_key_file(&key, None);
    let identity = format!("IdentityFile {}", key_path.display());

    let outcome = connect(&scenario.connector(&[identity.as_str()])).await;

    assert!(matches!(outcome, Err(ConnectError::Authentication { .. })));
    assert_eq!(
        scenario.server.offered_fingerprints(),
        vec![fingerprint_of(key.public_key())]
    );
}

#[tokio::test]
async fn should_not_ask_the_passphrase_when_the_agent_already_holds_the_encrypted_key() {
    let key = generate_key(Algorithm::Ed25519);
    let scenario = Scenario::new(None).await;
    scenario.agent.add_key(&key).await;
    let key_path = scenario.write_key_file(&key, Some(PASSPHRASE));
    let identity = format!("IdentityFile {}", key_path.display());

    connect(&scenario.connector(&[identity.as_str()]))
        .await
        .unwrap_err();

    assert_eq!(scenario.prompter.passphrase_requests().len(), 0);
}

#[tokio::test]
async fn should_fall_back_to_identity_files_when_the_agent_is_unreachable() {
    let key = generate_key(Algorithm::Ed25519);
    let scenario = Scenario::new(Some(&key)).await;
    let key_path = scenario.write_key_file(&key, None);
    let identity = format!("IdentityFile {}", key_path.display());
    let missing_socket = scenario.workspace.file("no-agent.sock");
    let locator = FixedAgentLocator {
        default_socket: Some(missing_socket),
    };

    let connector = scenario.connector_with(&[identity.as_str()], Arc::new(locator));

    assert!(connect(&connector).await.is_ok());
}
