#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use engine::{
    Engine, EngineDependencies, EngineEvent, Retention, Store, SystemClock, TunnelId, TunnelState,
};
use russh::keys::Algorithm;
use support::{
    FakeKeyring, FakePortScanner, FakeX11Environment, MemoryStateStore, ScriptedPrompter,
    TestServer, TestUser, Workspace, free_port, generate_key, wait_for_state, wait_until,
    write_private_key,
};
use tokio::sync::broadcast::Receiver;

const WAIT: Duration = Duration::from_secs(5);
const PASSPHRASE: &str = "correct horse";
const WRONG_PASSPHRASE: &str = "battery staple";

struct HarnessBuilder {
    retention: Retention,
    is_keyring_enabled: bool,
    keyring_entry: Option<&'static str>,
}

struct Harness {
    _workspace: Workspace,
    _store_directory: tempfile::TempDir,
    server: TestServer,
    engine: Engine,
    events: Receiver<EngineEvent>,
    prompter: Arc<ScriptedPrompter>,
    keyring: Arc<FakeKeyring>,
    state_store: Arc<MemoryStateStore>,
    key_path: PathBuf,
    id: TunnelId,
}

impl HarnessBuilder {
    fn new(retention: Retention) -> Self {
        Self {
            retention,
            is_keyring_enabled: false,
            keyring_entry: None,
        }
    }

    fn with_keyring_enabled(mut self) -> Self {
        self.is_keyring_enabled = true;
        self
    }

    fn with_keyring_entry(mut self, passphrase: &'static str) -> Self {
        self.keyring_entry = Some(passphrase);
        self
    }

    async fn start(self) -> Harness {
        let client_key = generate_key(Algorithm::Ed25519);
        let server = TestServer::builder()
            .host_key(generate_key(Algorithm::Ed25519))
            .user(
                support::REMOTE_USER,
                TestUser {
                    authorized_keys: vec![client_key.public_key().clone()],
                    ..TestUser::default()
                },
            )
            .start()
            .await;
        let workspace = Workspace::new();
        let written_path =
            write_private_key(workspace.path(), "id_locked", &client_key, Some(PASSPHRASE));
        let key_path = std::fs::canonicalize(&written_path).unwrap();
        let identity_line = format!("IdentityFile {}", written_path.display());
        let config_path = workspace.write_config(
            server.port,
            &[
                identity_line.as_str(),
                "IdentitiesOnly yes",
                "PreferredAuthentications publickey",
            ],
        );
        let prompter = Arc::new(
            ScriptedPrompter::default()
                .with_passphrases(&[Some(PASSPHRASE), Some(PASSPHRASE)])
                .with_passphrase_retention(self.retention),
        );
        let connector = workspace.connector_dependencies(
            &config_path,
            Arc::clone(&prompter) as _,
            support::rejecting_verifier(),
            Arc::new(support::NoAgentLocator),
        );
        let id = TunnelId::generate();
        let store_directory = tempfile::tempdir().unwrap();
        let store_path = store_directory.path().join("tunnels.toml");
        std::fs::write(&store_path, self.store_text(id, free_port().await)).unwrap();
        let (store, _) = Store::open(store_path).unwrap();
        let keyring = Arc::new(match self.keyring_entry {
            Some(passphrase) => FakeKeyring::default().with_entry(&key_path, passphrase),
            None => FakeKeyring::default(),
        });
        let state_store = Arc::new(MemoryStateStore::default());
        let engine = Engine::start(EngineDependencies {
            connector,
            clock: Arc::new(SystemClock),
            store,
            port_scanner: Arc::new(FakePortScanner::default()),
            x11_environment: Arc::new(FakeX11Environment::default()),
            state_store: state_store.clone(),
            keyring: Some(keyring.clone()),
        })
        .unwrap();
        let events = engine.subscribe();
        Harness {
            _workspace: workspace,
            _store_directory: store_directory,
            server,
            engine,
            events,
            prompter,
            keyring,
            state_store,
            key_path,
            id,
        }
    }

    fn store_text(&self, id: TunnelId, listen_port: u16) -> String {
        format!(
            "version = 1\n\n[settings]\nsession_idle_grace_s = 0\nuse_os_keyring = {}\n\n\
[[tunnels]]\nid = \"{id}\"\nhost = \"{}\"\nkind = \"local\"\nlisten_port = {listen_port}\ntarget_port = 9\n",
            self.is_keyring_enabled,
            support::ALIAS
        )
    }
}

impl Harness {
    async fn connect(&mut self) {
        self.engine.start_tunnel(self.id).unwrap();
        wait_for_state(
            &mut self.events,
            self.id,
            |state| matches!(state, TunnelState::Active),
            WAIT,
        )
        .await;
    }

    async fn disconnect(&mut self) {
        self.engine.stop_tunnel(self.id).unwrap();
        wait_for_state(
            &mut self.events,
            self.id,
            |state| matches!(state, TunnelState::Stopped),
            WAIT,
        )
        .await;
        let server = &self.server;
        wait_until(WAIT, "the session to close", || {
            server.open_connection_count() == 0
        })
        .await;
    }

    async fn reconnect(&mut self) {
        self.connect().await;
        self.disconnect().await;
        self.connect().await;
    }

    fn prompt_count(&self) -> usize {
        self.prompter.passphrase_requests().len()
    }

    async fn wait_for_recorded_entry(&self) {
        let store = Arc::clone(&self.state_store);
        let key_path = self.key_path.clone();
        wait_until(WAIT, "the keyring entry to be recorded", move || {
            store.state().keyring_entries.contains(&key_path)
        })
        .await;
    }
}

#[tokio::test]
async fn should_not_ask_again_when_the_passphrase_is_kept_for_the_session() {
    let mut harness = HarnessBuilder::new(Retention::Session).start().await;

    harness.reconnect().await;

    assert_eq!(harness.prompt_count(), 1);
}

#[tokio::test]
async fn should_ask_again_when_the_passphrase_is_not_kept() {
    let mut harness = HarnessBuilder::new(Retention::Never).start().await;

    harness.reconnect().await;

    assert_eq!(harness.prompt_count(), 2);
}

#[tokio::test]
async fn should_not_touch_the_keyring_when_the_passphrase_is_kept_for_the_session() {
    let mut harness = HarnessBuilder::new(Retention::Session)
        .with_keyring_enabled()
        .start()
        .await;

    harness.connect().await;

    assert_eq!(harness.keyring.store_count(), 0);
}

#[tokio::test]
async fn should_use_the_keyring_passphrase_without_asking_when_the_keyring_is_enabled() {
    let mut harness = HarnessBuilder::new(Retention::Never)
        .with_keyring_enabled()
        .with_keyring_entry(PASSPHRASE)
        .start()
        .await;

    harness.connect().await;

    assert_eq!(
        (harness.prompt_count(), harness.keyring.load_count()),
        (0, 1)
    );
}

#[tokio::test]
async fn should_ignore_the_keyring_when_the_setting_is_off() {
    let mut harness = HarnessBuilder::new(Retention::Never)
        .with_keyring_entry(PASSPHRASE)
        .start()
        .await;

    harness.connect().await;

    assert_eq!(
        (harness.prompt_count(), harness.keyring.load_count()),
        (1, 0)
    );
}

#[tokio::test]
async fn should_store_the_passphrase_in_the_keyring_when_the_user_asked_for_it() {
    let mut harness = HarnessBuilder::new(Retention::Keyring)
        .with_keyring_enabled()
        .start()
        .await;

    harness.connect().await;

    assert_eq!(
        harness.keyring.passphrase_of(&harness.key_path),
        Some(PASSPHRASE.to_owned())
    );
}

#[tokio::test]
async fn should_record_the_keyring_entry_when_the_passphrase_is_stored() {
    let mut harness = HarnessBuilder::new(Retention::Keyring)
        .with_keyring_enabled()
        .start()
        .await;

    harness.connect().await;

    harness.wait_for_recorded_entry().await;
}

#[tokio::test]
async fn should_keep_the_passphrase_in_memory_only_when_the_keyring_is_off() {
    let mut harness = HarnessBuilder::new(Retention::Keyring).start().await;

    harness.reconnect().await;

    assert_eq!(
        (harness.prompt_count(), harness.keyring.store_count()),
        (1, 0)
    );
}

#[tokio::test]
async fn should_forget_the_keyring_passphrase_and_ask_when_it_is_refused() {
    let mut harness = HarnessBuilder::new(Retention::Never)
        .with_keyring_enabled()
        .with_keyring_entry(WRONG_PASSPHRASE)
        .start()
        .await;

    harness.connect().await;

    assert_eq!(
        (
            harness.prompt_count(),
            harness.keyring.delete_count(),
            harness.keyring.entry_count()
        ),
        (1, 1, 0)
    );
}

#[tokio::test]
async fn should_delete_the_keyring_entries_and_count_them_when_secrets_are_forgotten() {
    let mut harness = HarnessBuilder::new(Retention::Keyring)
        .with_keyring_enabled()
        .start()
        .await;
    harness.connect().await;
    harness.wait_for_recorded_entry().await;

    let forgotten = harness.engine.forget_secrets().await;

    assert_eq!((forgotten, harness.keyring.entry_count()), (1, 0));
}

#[tokio::test]
async fn should_empty_the_recorded_keyring_entries_when_secrets_are_forgotten() {
    let mut harness = HarnessBuilder::new(Retention::Keyring)
        .with_keyring_enabled()
        .start()
        .await;
    harness.connect().await;
    harness.wait_for_recorded_entry().await;

    harness.engine.forget_secrets().await;

    let store = Arc::clone(&harness.state_store);
    wait_until(WAIT, "the recorded entries to be emptied", move || {
        store.state().keyring_entries.is_empty()
    })
    .await;
}

#[tokio::test]
async fn should_ask_again_when_secrets_were_forgotten() {
    let mut harness = HarnessBuilder::new(Retention::Session).start().await;
    harness.connect().await;
    harness.disconnect().await;

    harness.engine.forget_secrets().await;
    harness.connect().await;

    assert_eq!(harness.prompt_count(), 2);
}

#[tokio::test]
async fn should_forget_nothing_when_no_passphrase_was_remembered() {
    let harness = HarnessBuilder::new(Retention::Never).start().await;

    assert_eq!(harness.engine.forget_secrets().await, 0);
}
