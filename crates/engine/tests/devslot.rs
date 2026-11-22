#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::sync::Arc;
use std::time::Duration;

use engine::{
    DevslotError, DevslotOrigin, DevslotPortStatus, DevslotResyncTunnel, DevslotStaleAction,
    DevslotStaleReason, Engine, EngineDependencies, EngineOptions, NewTunnel, OpenOptions,
    PresetPlatform, Store, SystemClock, TunnelId, TunnelKind,
};
use russh::keys::Algorithm;
use support::{
    CommandEnding, FakePortScanner, FakeProgramLocator, FakeX11Environment, FakeX11Servers,
    MemoryStateStore, ScriptedCommand, TestServer, TestUser, Workspace, free_port, generate_key,
    write_private_key,
};

const REMOTE_PORT: u16 = 20050;
const GONE_PORT: u16 = 20999;
const OTHER_HOST: &str = "elsewhere";
const SHORT_TIMEOUT: Duration = Duration::from_millis(400);
const ONE_MEBIBYTE: usize = 1024 * 1024;

const LISTING: &str = r#"{"version":1,"instances":[{"name":"links-a","project":"web-shop","path":"/w/a","worktree":false,"slot":1,"app_url":"http://links-a.localhost:20050","services_running":true,"ports":[{"var":"PORT","port":20050,"kind":"web","url":"http://links-a.localhost:20050"},{"var":"DB_PORT","port":20052,"kind":"service","service":"postgres","target":5432}]}]}"#;

struct Harness {
    _workspace: Workspace,
    _store_directory: tempfile::TempDir,
    store_path: std::path::PathBuf,
    server: TestServer,
    state_store: Arc<MemoryStateStore>,
    engine: Engine,
}

fn ok_script(stdout: &str) -> ScriptedCommand {
    ScriptedCommand {
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
        ending: CommandEnding::ExitStatus(0),
    }
}

async fn start(script: Option<ScriptedCommand>, host_overrides: &str) -> Harness {
    start_with(TestServer::builder(), script, host_overrides).await
}

async fn start_with(
    builder: support::TestServerBuilder,
    script: Option<ScriptedCommand>,
    host_overrides: &str,
) -> Harness {
    let client_key = generate_key(Algorithm::Ed25519);
    let user = TestUser {
        authorized_keys: vec![client_key.public_key().clone()],
        ..TestUser::default()
    };
    let mut builder = builder
        .host_key(generate_key(Algorithm::Ed25519))
        .user(support::REMOTE_USER, user);
    if let Some(script) = script {
        builder = builder.answering_commands(script);
    }
    let server = builder.start().await;
    let workspace = Workspace::new();
    let key_path = write_private_key(workspace.path(), "id_test", &client_key, None);
    let identity_line = format!("IdentityFile {}", key_path.display());
    let config_path = workspace.write_config(
        server.port,
        &[
            identity_line.as_str(),
            "IdentitiesOnly yes",
            "PreferredAuthentications publickey",
        ],
    );
    let store_directory = tempfile::tempdir().unwrap();
    let store_path = store_directory.path().join("tunnels.toml");
    std::fs::write(
        &store_path,
        format!(
            "version = 1\n\n[settings]\nreconnect_initial_delay_ms = 20\nreconnect_max_delay_ms = 80\nsession_idle_grace_s = 0\n{host_overrides}"
        ),
    )
    .unwrap();
    let (store, _) = Store::open(store_path.clone()).unwrap();
    let connector = workspace.connector_dependencies(
        &config_path,
        support::quiet_prompter(),
        support::rejecting_verifier(),
        Arc::new(support::NoAgentLocator),
    );
    let state_store = Arc::new(MemoryStateStore::default());
    let engine = Engine::start_with_options(
        EngineDependencies {
            connector,
            clock: Arc::new(SystemClock),
            store,
            port_scanner: Arc::new(FakePortScanner::default()),
            x11_environment: Arc::new(FakeX11Environment::default()),
            x11_servers: Arc::new(FakeX11Servers::default()),
            state_store: state_store.clone(),
            keyring: None,
            program_locator: Arc::new(FakeProgramLocator::default()),
            preset_platform: PresetPlatform::Unix,
        },
        EngineOptions {
            remote_command_timeout: SHORT_TIMEOUT,
            ..EngineOptions::default()
        },
    )
    .unwrap();
    Harness {
        _workspace: workspace,
        _store_directory: store_directory,
        store_path,
        server,
        state_store,
        engine,
    }
}

async fn new_tunnel(host: &str, tags: &[&str]) -> NewTunnel {
    NewTunnel {
        host: host.to_owned(),
        kind: TunnelKind::Local,
        listen_port: free_port().await,
        listen_address: Some("127.0.0.1".to_owned()),
        target_host: None,
        target_port: Some(REMOTE_PORT),
        tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        open: OpenOptions::default(),
        standby: None,
    }
}

async fn create_saved(harness: &Harness, host: &str, tags: &[&str]) -> TunnelId {
    harness
        .engine
        .create_tunnel(new_tunnel(host, tags).await)
        .await
        .unwrap()
}

async fn start_temporary(harness: &Harness, host: &str, origin: Option<DevslotOrigin>) -> TunnelId {
    harness
        .engine
        .start_temporary_tunnel(new_tunnel(host, &[]).await, origin)
        .await
        .unwrap()
}

fn origin_of(project_tag: &str) -> DevslotOrigin {
    DevslotOrigin {
        project_tag: Some(project_tag.to_owned()),
    }
}

fn request_for(harness: &Harness, id: TunnelId) -> DevslotResyncTunnel {
    let snapshot = harness
        .engine
        .snapshot()
        .tunnels
        .into_iter()
        .find(|tunnel| tunnel.definition.id == id)
        .unwrap();
    DevslotResyncTunnel {
        definition: snapshot.definition,
        is_temporary: snapshot.is_temporary,
    }
}

fn tunnel_ids(harness: &Harness) -> Vec<TunnelId> {
    harness
        .engine
        .snapshot()
        .tunnels
        .iter()
        .map(|tunnel| tunnel.definition.id)
        .collect()
}

#[tokio::test]
async fn should_run_ds_ls_json_when_the_host_has_no_devslot_path() {
    let harness = start(Some(ok_script(LISTING)), "").await;

    harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap();

    assert_eq!(harness.server.executed_commands(), ["'ds' ls --json"]);
}

#[tokio::test]
async fn should_run_the_configured_program_when_the_host_has_a_devslot_path_with_a_home_prefix() {
    let overrides = format!(
        "\n[hosts.{}]\ndevslot_path = \"~/.cargo/bin/ds\"\n",
        support::ALIAS
    );
    let harness = start(Some(ok_script(LISTING)), &overrides).await;

    harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap();

    assert_eq!(
        harness.server.executed_commands(),
        ["~/'.cargo/bin/ds' ls --json"]
    );
}

#[tokio::test]
async fn should_list_the_instances_when_the_command_succeeds() {
    let harness = start(Some(ok_script(LISTING)), "").await;

    let discovery = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap();

    assert_eq!(
        (
            discovery.instances.len(),
            discovery.instances[0].project_tag.clone(),
            discovery.instances[0].ports[0].host_name.clone(),
            discovery.instances[0].ports[0].status,
            discovery.persist_default,
            discovery.stale.len()
        ),
        (
            1,
            Some("ds-web-shop".to_owned()),
            Some("links-a.localhost".to_owned()),
            DevslotPortStatus::New,
            false,
            0
        )
    );
}

#[tokio::test]
async fn should_use_the_host_override_as_the_persist_default_when_it_is_set() {
    let overrides = format!("\n[hosts.{}]\ndevslot_persist = true\n", support::ALIAS);
    let harness = start(Some(ok_script(LISTING)), &overrides).await;

    let discovery = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap();

    assert!(discovery.persist_default);
}

#[tokio::test]
async fn should_report_the_tunnel_and_the_stale_ones_when_tunnels_exist() {
    let harness = start(Some(ok_script(LISTING)), "").await;
    let listed = create_saved(&harness, support::ALIAS, &[]).await;
    let mut gone = new_tunnel(support::ALIAS, &["devslot", "ds-web-shop"]).await;
    gone.target_port = Some(GONE_PORT);
    let gone_id = harness.engine.create_tunnel(gone).await.unwrap();
    create_saved(&harness, support::ALIAS, &["devslot", "ds-web-shop"]).await;

    let discovery = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap();

    assert_eq!(
        (
            discovery.instances[0].ports[0].status,
            discovery
                .stale
                .iter()
                .map(|stale| (stale.definition.id, stale.reason, stale.action))
                .collect::<Vec<_>>(),
        ),
        (
            DevslotPortStatus::AlreadyTunneled { tunnel_id: listed },
            vec![(
                gone_id,
                DevslotStaleReason::PortGone,
                DevslotStaleAction::Ask
            )],
        )
    );
}

#[tokio::test]
async fn should_report_not_found_when_the_exit_status_is_127() {
    let script = ScriptedCommand {
        stdout: Vec::new(),
        stderr: b"sh: ds: not found\n".to_vec(),
        ending: CommandEnding::ExitStatus(127),
    };
    let harness = start(Some(script), "").await;

    let error = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap_err();

    assert!(
        matches!(&error, engine::EngineError::Devslot(DevslotError::NotFound { path, .. }) if path == "ds"),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_report_the_status_and_the_standard_error_when_the_command_fails() {
    let script = ScriptedCommand {
        stdout: Vec::new(),
        stderr: b"registry locked\n".to_vec(),
        ending: CommandEnding::ExitStatus(2),
    };
    let harness = start(Some(script), "").await;

    let error = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap_err();

    assert!(
        matches!(&error, engine::EngineError::Devslot(DevslotError::Failed { status: Some(2), stderr, .. }) if stderr == "registry locked"),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_report_a_failure_without_status_when_the_command_ends_without_one() {
    let script = ScriptedCommand {
        stdout: LISTING.as_bytes().to_vec(),
        stderr: Vec::new(),
        ending: CommandEnding::NoStatus,
    };
    let harness = start(Some(script), "").await;

    let error = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap_err();

    assert!(
        matches!(
            &error,
            engine::EngineError::Devslot(DevslotError::Failed { status: None, .. })
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_report_a_timeout_when_the_command_never_ends() {
    let script = ScriptedCommand {
        stdout: Vec::new(),
        stderr: Vec::new(),
        ending: CommandEnding::Hang,
    };
    let harness = start(Some(script), "").await;

    let error = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap_err();

    assert!(
        matches!(
            &error,
            engine::EngineError::Devslot(DevslotError::Timeout { seconds: 1, .. })
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_report_an_invalid_output_when_it_exceeds_one_mebibyte() {
    let script = ScriptedCommand {
        stdout: vec![b' '; ONE_MEBIBYTE + 1],
        stderr: Vec::new(),
        ending: CommandEnding::ExitStatus(0),
    };
    let harness = start(Some(script), "").await;

    let error = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap_err();

    assert!(
        matches!(
            &error,
            engine::EngineError::Devslot(DevslotError::InvalidOutput { .. })
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_report_an_invalid_output_when_the_listing_is_not_json() {
    let harness = start(Some(ok_script("welcome to vega")), "").await;

    let error = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap_err();

    assert!(
        matches!(
            &error,
            engine::EngineError::Devslot(DevslotError::InvalidOutput { .. })
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_remove_the_listed_stale_tunnels_when_nothing_changed() {
    let harness = start(None, "").await;
    let saved = create_saved(&harness, support::ALIAS, &["devslot"]).await;
    let temporary = start_temporary(&harness, support::ALIAS, Some(DevslotOrigin::default())).await;
    let requested = vec![
        request_for(&harness, saved),
        request_for(&harness, temporary),
    ];

    let outcome = harness
        .engine
        .resync_devslot(support::ALIAS, requested)
        .await
        .unwrap();

    assert_eq!(
        (outcome.removed, outcome.skipped, tunnel_ids(&harness)),
        (vec![saved, temporary], vec![], vec![])
    );
}

#[tokio::test]
async fn should_write_the_store_without_the_deleted_tunnels_when_saved_ones_are_removed() {
    let harness = start(None, "").await;
    let saved = create_saved(&harness, support::ALIAS, &["devslot"]).await;
    let neighbour = create_saved(&harness, support::ALIAS, &[]).await;
    let requested = vec![request_for(&harness, saved)];

    harness
        .engine
        .resync_devslot(support::ALIAS, requested)
        .await
        .unwrap();

    let text = std::fs::read_to_string(&harness.store_path).unwrap();
    assert_eq!(
        (
            text.contains(&saved.to_string()),
            text.contains(&neighbour.to_string())
        ),
        (false, true)
    );
}

#[tokio::test]
async fn should_keep_the_neighbours_when_a_resync_removes_stale_tunnels() {
    let harness = start(None, "").await;
    let stale = create_saved(&harness, support::ALIAS, &["devslot"]).await;
    let other_host = create_saved(&harness, OTHER_HOST, &["devslot"]).await;
    let untagged = create_saved(&harness, support::ALIAS, &["mine"]).await;
    let plain_temporary = start_temporary(&harness, support::ALIAS, None).await;
    let requested = vec![request_for(&harness, stale)];

    harness
        .engine
        .resync_devslot(support::ALIAS, requested)
        .await
        .unwrap();

    assert_eq!(
        tunnel_ids(&harness),
        vec![other_host, untagged, plain_temporary]
    );
}

#[tokio::test]
async fn should_not_remove_a_tunnel_of_another_host_when_it_is_asked_for_this_host() {
    let harness = start(None, "").await;
    let other_host = create_saved(&harness, OTHER_HOST, &["devslot"]).await;
    let requested = vec![request_for(&harness, other_host)];

    let outcome = harness
        .engine
        .resync_devslot(support::ALIAS, requested)
        .await
        .unwrap();

    assert_eq!(
        (outcome.skipped, tunnel_ids(&harness)),
        (vec![other_host], vec![other_host])
    );
}

#[tokio::test]
async fn should_not_remove_a_saved_tunnel_without_the_devslot_tag_when_it_is_asked() {
    let harness = start(None, "").await;
    let untagged = create_saved(&harness, support::ALIAS, &["mine"]).await;
    let requested = vec![request_for(&harness, untagged)];

    let outcome = harness
        .engine
        .resync_devslot(support::ALIAS, requested)
        .await
        .unwrap();

    assert_eq!(
        (outcome.skipped, tunnel_ids(&harness)),
        (vec![untagged], vec![untagged])
    );
}

#[tokio::test]
async fn should_not_remove_a_temporary_tunnel_without_origin_when_it_is_asked() {
    let harness = start(None, "").await;
    let plain = start_temporary(&harness, support::ALIAS, None).await;
    let requested = vec![request_for(&harness, plain)];

    let outcome = harness
        .engine
        .resync_devslot(support::ALIAS, requested)
        .await
        .unwrap();

    assert_eq!(
        (outcome.skipped, tunnel_ids(&harness)),
        (vec![plain], vec![plain])
    );
}

#[tokio::test]
async fn should_skip_a_tunnel_when_its_definition_changed_since_the_analysis() {
    let harness = start(None, "").await;
    let saved = create_saved(&harness, support::ALIAS, &["devslot"]).await;
    let requested = vec![request_for(&harness, saved)];
    let mut changed = requested[0].definition.clone();
    changed.tags.push("edited".to_owned());
    harness.engine.update_tunnel(changed).await.unwrap();

    let outcome = harness
        .engine
        .resync_devslot(support::ALIAS, requested)
        .await
        .unwrap();

    assert_eq!(
        (outcome.skipped, tunnel_ids(&harness)),
        (vec![saved], vec![saved])
    );
}

#[tokio::test]
async fn should_skip_a_temporary_tunnel_when_it_was_saved_between_the_discovery_and_the_resync() {
    let harness = start(None, "").await;
    let temporary = start_temporary(&harness, support::ALIAS, Some(origin_of("ds-web-shop"))).await;
    let requested = vec![request_for(&harness, temporary)];
    harness.engine.persist_tunnel(temporary).await.unwrap();

    let outcome = harness
        .engine
        .resync_devslot(support::ALIAS, requested)
        .await
        .unwrap();

    assert_eq!(
        (outcome.skipped, tunnel_ids(&harness)),
        (vec![temporary], vec![temporary])
    );
}

#[tokio::test]
async fn should_count_a_vanished_tunnel_as_removed_when_it_is_resynced_twice() {
    let harness = start(None, "").await;
    let saved = create_saved(&harness, support::ALIAS, &["devslot"]).await;
    let requested = vec![request_for(&harness, saved)];
    harness
        .engine
        .resync_devslot(support::ALIAS, requested.clone())
        .await
        .unwrap();

    let outcome = harness
        .engine
        .resync_devslot(support::ALIAS, requested)
        .await
        .unwrap();

    assert_eq!((outcome.removed, outcome.skipped), (vec![saved], vec![]));
}

#[tokio::test]
async fn should_never_remember_a_devslot_temporary_in_the_state_or_the_store() {
    let harness = start(None, "").await;
    let temporary = start_temporary(&harness, support::ALIAS, Some(origin_of("ds-web-shop"))).await;
    harness.engine.stop_tunnel(temporary).unwrap();
    let again = start_temporary(&harness, support::ALIAS, Some(DevslotOrigin::default())).await;

    harness.engine.shutdown().await;

    let text = std::fs::read_to_string(&harness.store_path).unwrap();
    let state = harness.state_store.state();
    assert_eq!(
        (
            state.running.contains(&temporary) || state.running.contains(&again),
            text.contains("devslot"),
            text.contains(&again.to_string())
        ),
        (false, false, false)
    );
}

#[tokio::test]
async fn should_expose_the_devslot_origin_in_the_snapshot_of_a_temporary_tunnel() {
    let harness = start(None, "").await;
    let id = start_temporary(&harness, support::ALIAS, Some(origin_of("ds-web-shop"))).await;

    let snapshot = harness
        .engine
        .snapshot()
        .tunnels
        .into_iter()
        .find(|tunnel| tunnel.definition.id == id)
        .unwrap();

    assert_eq!(
        (snapshot.devslot, snapshot.definition.tags),
        (
            Some(DevslotOrigin {
                project_tag: Some("ds-web-shop".to_owned())
            }),
            Vec::<String>::new()
        )
    );
}

#[tokio::test]
async fn should_refuse_a_devslot_origin_when_its_project_tag_lacks_the_prefix() {
    let harness = start(None, "").await;

    let outcome = harness
        .engine
        .start_temporary_tunnel(
            new_tunnel(support::ALIAS, &[]).await,
            Some(DevslotOrigin {
                project_tag: Some("web-shop".to_owned()),
            }),
        )
        .await;

    assert!(matches!(
        outcome,
        Err(engine::EngineError::InvalidDevslotProjectTag(_))
    ));
}

#[tokio::test]
async fn should_save_a_devslot_temporary_with_the_devslot_and_project_tags_when_it_is_persisted() {
    let harness = start(None, "").await;
    let id = start_temporary(&harness, support::ALIAS, Some(origin_of("ds-web-shop"))).await;

    harness.engine.persist_tunnel(id).await.unwrap();

    let snapshot = harness
        .engine
        .snapshot()
        .tunnels
        .into_iter()
        .find(|tunnel| tunnel.definition.id == id)
        .unwrap();
    let text = std::fs::read_to_string(&harness.store_path).unwrap();
    assert_eq!(
        (
            snapshot.definition.tags,
            snapshot.is_temporary,
            snapshot.devslot,
            text.contains("tags = [\"devslot\", \"ds-web-shop\"]")
        ),
        (
            vec!["devslot".to_owned(), "ds-web-shop".to_owned()],
            false,
            None,
            true
        )
    );
}

#[tokio::test]
async fn should_save_a_plain_temporary_without_tags_when_it_is_persisted() {
    let harness = start(None, "").await;
    let id = start_temporary(&harness, support::ALIAS, None).await;

    harness.engine.persist_tunnel(id).await.unwrap();

    let snapshot = harness
        .engine
        .snapshot()
        .tunnels
        .into_iter()
        .find(|tunnel| tunnel.definition.id == id)
        .unwrap();
    assert_eq!(snapshot.definition.tags, Vec::<String>::new());
}

#[tokio::test]
async fn should_report_a_timeout_when_the_session_channel_never_opens() {
    let harness = start_with(TestServer::builder().stalling_session_channels(), None, "").await;

    let outcome = tokio::time::timeout(
        Duration::from_secs(5),
        harness.engine.discover_devslot(support::ALIAS),
    )
    .await
    .unwrap();

    assert!(
        matches!(
            &outcome,
            Err(engine::EngineError::Devslot(DevslotError::Timeout { .. }))
        ),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn should_refuse_a_listing_when_it_holds_more_instances_than_the_cap() {
    let instances: Vec<String> = (0..=engine::MAX_DEVSLOT_INSTANCES)
        .map(|index| format!(r#"{{"name":"i{index}","path":"/a","slot":1,"ports":[]}}"#))
        .collect();
    let listing = format!(r#"{{"version":1,"instances":[{}]}}"#, instances.join(","));
    let harness = start(Some(ok_script(&listing)), "").await;

    let error = harness
        .engine
        .discover_devslot(support::ALIAS)
        .await
        .unwrap_err();

    assert!(
        matches!(&error, engine::EngineError::Devslot(DevslotError::InvalidOutput { reason, .. }) if reason.contains("instances")),
        "{error:?}"
    );
}
