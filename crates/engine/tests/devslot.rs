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
use tokio::sync::Semaphore;

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

const INSTANCE: &str = "links-a";
const OTHER_INSTANCE: &str = "links-b";
const DELETE_STALE: &str = "devslot_stale = \"delete\"\n";

struct UpPorts {
    web: u16,
    service: u16,
}

async fn up_ports() -> UpPorts {
    let ports = support::free_ports(2).await;
    UpPorts {
        web: ports[0],
        service: ports[1],
    }
}

fn instance_entry(name: &str, ports: &UpPorts) -> String {
    format!(
        r#"{{"name":"{name}","project":"web-shop","path":"/w/{name}","worktree":false,"slot":1,"app_url":"http://{name}.localhost:{web}","services_running":true,"ports":[{{"var":"PORT","port":{web},"kind":"web","url":"http://{name}.localhost:{web}"}},{{"var":"DB_PORT","port":{service},"kind":"service","service":"postgres","target":5432}}]}}"#,
        web = ports.web,
        service = ports.service
    )
}

fn listing_of(entries: &[String]) -> String {
    format!(r#"{{"version":1,"instances":[{}]}}"#, entries.join(","))
}

async fn start_with_instance(ports: &UpPorts, settings: &str) -> Harness {
    let listing = listing_of(&[instance_entry(INSTANCE, ports)]);
    start(Some(ok_script(&listing)), settings).await
}

fn snapshot_of(harness: &Harness, id: TunnelId) -> engine::TunnelSnapshot {
    harness
        .engine
        .snapshot()
        .tunnels
        .into_iter()
        .find(|tunnel| tunnel.definition.id == id)
        .unwrap()
}

fn tunnel_to(host: &str, listen_port: u16, remote_port: u16, tags: &[&str]) -> NewTunnel {
    NewTunnel {
        host: host.to_owned(),
        kind: TunnelKind::Local,
        listen_port,
        listen_address: Some("127.0.0.1".to_owned()),
        target_host: None,
        target_port: Some(remote_port),
        tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        open: OpenOptions::default(),
        standby: None,
    }
}

#[tokio::test]
async fn should_start_the_web_port_as_a_temporary_tunnel_with_the_origin_when_the_instance_is_listed()
 {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, "").await;

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    let [started] = outcome.started.as_slice() else {
        panic!("expected one started port: {outcome:?}");
    };
    let snapshot = snapshot_of(&harness, started.tunnel_id);
    assert_eq!(
        (
            started.remote_port,
            started.listen_port,
            snapshot.is_temporary,
            snapshot.definition.tags,
            snapshot.devslot,
            snapshot.definition.open.host_name,
            snapshot.definition.listen_address,
            snapshot.definition.target_port
        ),
        (
            ports.web,
            ports.web,
            true,
            Vec::<String>::new(),
            Some(origin_of("ds-web-shop")),
            Some("links-a.localhost".to_owned()),
            Some("127.0.0.1".to_owned()),
            None
        )
    );
}

#[tokio::test]
async fn should_not_tunnel_the_service_port_when_the_instance_is_opened() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, "").await;

    harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    let remote_ports: Vec<u16> = harness
        .engine
        .snapshot()
        .tunnels
        .iter()
        .map(|tunnel| tunnel.definition.effective_target_port())
        .collect();
    assert_eq!(remote_ports, vec![ports.web]);
}

#[tokio::test]
async fn should_wait_until_the_tunnel_is_active_and_name_it_as_the_app_tunnel() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, "").await;

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    let app_tunnel = outcome.app_tunnel.unwrap();
    assert_eq!(
        (
            snapshot_of(&harness, app_tunnel).state,
            outcome.app_url,
            outcome.is_app_port_shared
        ),
        (
            engine::TunnelState::Active,
            Some(format!("http://links-a.localhost:{}", ports.web)),
            false
        )
    );
}

#[tokio::test]
async fn should_report_the_instance_as_not_found_when_the_host_does_not_list_it() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, "").await;

    let error = harness
        .engine
        .devslot_up(support::ALIAS, "links-zzz")
        .await
        .unwrap_err();

    assert!(
        matches!(&error, engine::EngineError::Devslot(DevslotError::InstanceNotFound { host, instance }) if host == support::ALIAS && instance == "links-zzz"),
        "{error:?}"
    );
}

#[tokio::test]
async fn should_remove_nothing_when_the_instance_is_not_found_even_with_the_delete_policy() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, DELETE_STALE).await;
    let listed = create_saved(&harness, support::ALIAS, &["devslot", "ds-web-shop"]).await;
    let mut gone = new_tunnel(support::ALIAS, &["devslot", "ds-web-shop"]).await;
    gone.target_port = Some(GONE_PORT);
    let gone_id = harness.engine.create_tunnel(gone).await.unwrap();
    let temporary = start_temporary(&harness, support::ALIAS, Some(origin_of("ds-web-shop"))).await;
    let before = tunnel_ids(&harness);

    harness
        .engine
        .devslot_up(support::ALIAS, "links-zzz")
        .await
        .unwrap_err();

    assert_eq!(
        (tunnel_ids(&harness), before.contains(&gone_id)),
        (before, true)
    );
    assert!(tunnel_ids(&harness).contains(&listed) && tunnel_ids(&harness).contains(&temporary));
}

#[tokio::test]
async fn should_create_nothing_and_report_the_port_as_existing_when_the_instance_is_opened_twice() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, "").await;
    let first = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    let second = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    assert_eq!(
        (
            second.started.len(),
            second.existing.clone(),
            tunnel_ids(&harness).len(),
            second.app_tunnel
        ),
        (0, first.started.clone(), 1, first.app_tunnel)
    );
}

#[tokio::test]
async fn should_start_a_stopped_tunnel_again_when_the_instance_is_opened_again() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, "").await;
    let first = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();
    let saved = first.app_tunnel.unwrap();
    harness.engine.persist_tunnel(saved).await.unwrap();
    harness.engine.stop_tunnel(saved).unwrap();

    let second = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    assert_eq!(
        (
            second.existing.len(),
            snapshot_of(&harness, saved).state,
            second.failed
        ),
        (1, engine::TunnelState::Active, Vec::new())
    );
}

#[tokio::test]
async fn should_create_each_tunnel_once_when_two_calls_run_at_the_same_time() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, "").await;

    let (first, second) = tokio::join!(
        harness.engine.devslot_up(support::ALIAS, INSTANCE),
        harness.engine.devslot_up(support::ALIAS, INSTANCE)
    );
    let (first, second) = (first.unwrap(), second.unwrap());

    assert_eq!(
        (
            first.started.len() + second.started.len(),
            first.existing.len() + second.existing.len(),
            tunnel_ids(&harness).len()
        ),
        (1, 1, 1)
    );
}

#[tokio::test]
async fn should_remove_a_stale_temporary_route_and_report_it_when_the_instance_is_opened() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, "").await;
    let stale = harness
        .engine
        .start_temporary_tunnel(
            tunnel_to(support::ALIAS, free_port().await, GONE_PORT, &[]),
            Some(origin_of("ds-web-shop")),
        )
        .await
        .unwrap();

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    assert_eq!(
        (
            outcome.removed.len(),
            outcome.removed[0].remote_port,
            outcome.removed[0].is_temporary,
            tunnel_ids(&harness).contains(&stale)
        ),
        (1, GONE_PORT, true, false)
    );
}

#[tokio::test]
async fn should_remove_a_saved_route_whose_port_is_gone_when_the_policy_is_delete() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, DELETE_STALE).await;
    let kept = harness
        .engine
        .create_tunnel(tunnel_to(
            support::ALIAS,
            free_port().await,
            ports.service,
            &["devslot", "ds-web-shop"],
        ))
        .await
        .unwrap();
    let gone = harness
        .engine
        .create_tunnel(tunnel_to(
            support::ALIAS,
            free_port().await,
            GONE_PORT,
            &["devslot", "ds-web-shop"],
        ))
        .await
        .unwrap();

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    let ids = tunnel_ids(&harness);
    assert_eq!(
        (
            outcome.removed.len(),
            ids.contains(&gone),
            ids.contains(&kept)
        ),
        (1, false, true)
    );
}

#[tokio::test]
async fn should_keep_a_saved_route_that_needs_a_question_and_skip_its_port_when_the_instance_is_opened()
 {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, DELETE_STALE).await;
    let reassigned = harness
        .engine
        .create_tunnel(tunnel_to(
            support::ALIAS,
            free_port().await,
            ports.web,
            &["devslot", "ds-other-project"],
        ))
        .await
        .unwrap();

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    assert_eq!(
        (
            tunnel_ids(&harness),
            outcome.needs_review,
            outcome.skipped,
            outcome.started.len() + outcome.existing.len(),
            outcome.app_tunnel
        ),
        (
            vec![reassigned],
            1,
            vec![engine::DevslotSkippedPort {
                remote_port: ports.web,
                reason: engine::DevslotSkipReason::NeedsReview
            }],
            0,
            None
        )
    );
}

#[tokio::test]
async fn should_keep_the_routes_of_other_hosts_and_plain_tunnels_when_it_removes_stale_routes() {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, DELETE_STALE).await;
    let other_host_temporary = harness
        .engine
        .start_temporary_tunnel(
            tunnel_to(OTHER_HOST, free_port().await, GONE_PORT, &[]),
            Some(origin_of("ds-web-shop")),
        )
        .await
        .unwrap();
    let other_host_saved = harness
        .engine
        .create_tunnel(tunnel_to(
            OTHER_HOST,
            free_port().await,
            GONE_PORT,
            &["devslot", "ds-web-shop"],
        ))
        .await
        .unwrap();
    let plain = harness
        .engine
        .create_tunnel(tunnel_to(support::ALIAS, free_port().await, GONE_PORT, &[]))
        .await
        .unwrap();
    harness
        .engine
        .start_temporary_tunnel(
            tunnel_to(support::ALIAS, free_port().await, GONE_PORT, &[]),
            Some(origin_of("ds-web-shop")),
        )
        .await
        .unwrap();

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    let ids = tunnel_ids(&harness);
    assert_eq!(
        (
            outcome.removed.len(),
            ids.contains(&other_host_temporary),
            ids.contains(&other_host_saved),
            ids.contains(&plain)
        ),
        (1, true, true, true)
    );
}

#[tokio::test]
async fn should_save_the_tunnel_with_the_devslot_and_project_tags_when_the_host_persists() {
    let ports = up_ports().await;
    let overrides = format!("\n[hosts.{}]\ndevslot_persist = true\n", support::ALIAS);
    let harness = start_with_instance(&ports, &overrides).await;

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    let snapshot = snapshot_of(&harness, outcome.started[0].tunnel_id);
    let text = std::fs::read_to_string(&harness.store_path).unwrap();
    assert_eq!(
        (
            snapshot.is_temporary,
            snapshot.devslot,
            snapshot.definition.tags,
            text.contains("tags = [\"devslot\", \"ds-web-shop\"]"),
            harness
                .state_store
                .state()
                .running
                .contains(&outcome.started[0].tunnel_id)
        ),
        (
            false,
            None,
            vec!["devslot".to_owned(), "ds-web-shop".to_owned()],
            true,
            true
        )
    );
}

#[tokio::test]
async fn should_listen_on_another_port_when_a_stopped_tunnel_of_another_host_listens_on_the_web_port()
 {
    let ports = up_ports().await;
    let harness = start_with_instance(&ports, "").await;
    harness
        .engine
        .create_tunnel(tunnel_to(OTHER_HOST, ports.web, GONE_PORT, &[]))
        .await
        .unwrap();

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    let [started] = outcome.started.as_slice() else {
        panic!("expected one started port: {outcome:?}");
    };
    assert_eq!(
        (
            started.remote_port,
            started.listen_port,
            snapshot_of(&harness, started.tunnel_id)
                .definition
                .target_port
        ),
        (ports.web, ports.web + 10_000, Some(ports.web))
    );
}

#[tokio::test]
async fn should_skip_a_port_below_1024_and_start_nothing_when_the_web_port_is_privileged() {
    let listing = r#"{"version":1,"instances":[{"name":"links-a","project":"web-shop","path":"/w/a","worktree":false,"slot":1,"app_url":"http://links-a.localhost:80","ports":[{"var":"PORT","port":80,"kind":"web","url":"http://links-a.localhost:80"}]}]}"#;
    let harness = start(Some(ok_script(listing)), "").await;

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    assert_eq!(
        (outcome.skipped, tunnel_ids(&harness), outcome.app_tunnel),
        (
            vec![engine::DevslotSkippedPort {
                remote_port: 80,
                reason: engine::DevslotSkipReason::PrivilegedPort
            }],
            Vec::new(),
            None
        )
    );
}

#[tokio::test]
async fn should_open_only_the_asked_instance_when_the_host_lists_several() {
    let first_ports = up_ports().await;
    let second_ports = up_ports().await;
    let listing = listing_of(&[
        instance_entry(INSTANCE, &first_ports),
        instance_entry(OTHER_INSTANCE, &second_ports),
    ]);
    let harness = start(Some(ok_script(&listing)), "").await;

    harness
        .engine
        .devslot_up(support::ALIAS, OTHER_INSTANCE)
        .await
        .unwrap();

    let remote_ports: Vec<u16> = harness
        .engine
        .snapshot()
        .tunnels
        .iter()
        .map(|tunnel| tunnel.definition.effective_target_port())
        .collect();
    assert_eq!(remote_ports, vec![second_ports.web]);
}

#[tokio::test]
async fn should_flag_the_app_port_as_shared_when_another_instance_lists_it() {
    let ports = up_ports().await;
    let listing = listing_of(&[
        instance_entry(INSTANCE, &ports),
        instance_entry(OTHER_INSTANCE, &ports),
    ]);
    let harness = start(Some(ok_script(&listing)), "").await;

    let outcome = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    assert!(outcome.is_app_port_shared);
}

#[tokio::test]
async fn should_report_the_listing_failure_without_creating_anything_when_the_host_fails() {
    let script = ScriptedCommand {
        stdout: Vec::new(),
        stderr: b"registry locked\n".to_vec(),
        ending: CommandEnding::ExitStatus(2),
    };
    let harness = start(Some(script), "").await;

    let error = harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap_err();

    assert!(
        matches!(
            &error,
            engine::EngineError::Devslot(DevslotError::Failed {
                status: Some(2),
                ..
            })
        ),
        "{error:?}"
    );
    assert_eq!(tunnel_ids(&harness), Vec::new());
}

const CALLERS: usize = 3;
const WAIT_LIMIT: Duration = Duration::from_secs(5);

type UpOutcome = Result<engine::DevslotUp, engine::EngineError>;

async fn start_gated(script: ScriptedCommand, permits: usize) -> (Harness, Arc<Semaphore>) {
    let gate = Arc::new(Semaphore::new(permits));
    let builder = TestServer::builder().holding_command_answers(Arc::clone(&gate));
    (start_with(builder, Some(script), "").await, gate)
}

async fn wait_for_commands(harness: &Harness, count: usize) {
    support::wait_until(
        WAIT_LIMIT,
        "the listing command to reach the server",
        || harness.server.executed_commands().len() >= count,
    )
    .await;
}

async fn release_once_received(harness: &Harness, gate: &Semaphore) {
    wait_for_commands(harness, 1).await;
    gate.add_permits(1);
}

async fn open_concurrently(harness: &Harness, gate: &Semaphore) -> [UpOutcome; CALLERS] {
    let (first, second, third, ()) = tokio::join!(
        harness.engine.devslot_up(support::ALIAS, INSTANCE),
        harness.engine.devslot_up(support::ALIAS, INSTANCE),
        harness.engine.devslot_up(support::ALIAS, INSTANCE),
        release_once_received(harness, gate)
    );
    [first, second, third]
}

#[tokio::test]
async fn should_run_one_listing_when_the_same_host_is_opened_concurrently() {
    let ports = up_ports().await;
    let listing = listing_of(&[instance_entry(INSTANCE, &ports)]);
    let (harness, gate) = start_gated(ok_script(&listing), 0).await;

    let outcomes = open_concurrently(&harness, &gate).await;
    let started: usize = outcomes
        .into_iter()
        .map(|outcome| outcome.unwrap().started.len())
        .sum();

    assert_eq!(
        (
            harness.server.executed_commands().len(),
            started,
            tunnel_ids(&harness).len()
        ),
        (1, 1, 1)
    );
}

#[tokio::test]
async fn should_share_the_listing_error_with_every_concurrent_caller() {
    let script = ScriptedCommand {
        stdout: Vec::new(),
        stderr: b"registry locked".to_vec(),
        ending: CommandEnding::ExitStatus(2),
    };
    let (harness, gate) = start_gated(script, 0).await;

    let outcomes = open_concurrently(&harness, &gate).await;
    let shared_errors = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome,
                Err(engine::EngineError::Devslot(DevslotError::Failed { status: Some(2), stderr, .. })) if stderr == "registry locked"
            )
        })
        .count();

    assert_eq!(
        (harness.server.executed_commands().len(), shared_errors),
        (1, CALLERS)
    );
}

#[tokio::test]
async fn should_run_a_new_listing_when_the_previous_one_has_ended() {
    let ports = up_ports().await;
    let listing = listing_of(&[instance_entry(INSTANCE, &ports)]);
    let (harness, _gate) = start_gated(ok_script(&listing), 2).await;

    harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();
    harness
        .engine
        .devslot_up(support::ALIAS, INSTANCE)
        .await
        .unwrap();

    assert_eq!(harness.server.executed_commands().len(), 2);
}

#[tokio::test]
async fn should_finish_the_listing_for_the_others_when_the_first_caller_goes_away() {
    let ports = up_ports().await;
    let listing = listing_of(&[instance_entry(INSTANCE, &ports)]);
    let (harness, gate) = start_gated(ok_script(&listing), 0).await;
    let first = tokio::spawn({
        let engine = harness.engine.clone();
        async move { engine.devslot_up(support::ALIAS, INSTANCE).await }
    });
    wait_for_commands(&harness, 1).await;
    first.abort();
    let _ = first.await;

    let (second, ()) = tokio::join!(
        harness.engine.devslot_up(support::ALIAS, INSTANCE),
        release_once_received(&harness, &gate)
    );

    assert_eq!(
        (
            second.unwrap().started.len(),
            harness.server.executed_commands().len()
        ),
        (1, 1)
    );
}
