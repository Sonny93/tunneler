#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use engine::{
    ConfigExportFilter, ConfigImportRisk, ConfigImportStatus, Engine, EngineDependencies,
    EngineError, EngineEvent, ImportFileError, ImportRoute, MAX_IMPORT_FILE_BYTES, NewTunnel,
    OpenOptions, OpenScheme, PresetPlatform, RouteConflict, Store, StoreError, SystemClock,
    TunnelDefinition, TunnelId, TunnelKind, TunnelState, ValidationError,
};
use support::{
    FakePortScanner, FakeProgramLocator, FakeX11Environment, FakeX11Servers, MemoryStateStore,
    Workspace,
};
use tokio::sync::broadcast::error::TryRecvError;

const KNOWN_HOSTS: [&str; 2] = ["nova", "atlas"];

struct Harness {
    _workspace: Workspace,
    _store_directory: tempfile::TempDir,
    store_path: PathBuf,
    engine: Engine,
}

fn known_hosts() -> Vec<String> {
    KNOWN_HOSTS.iter().map(|host| (*host).to_owned()).collect()
}

fn start(store_text: &str) -> Harness {
    let workspace = Workspace::new();
    let config_path = workspace.path().join("ssh_config");
    std::fs::write(&config_path, "").unwrap();
    let store_directory = tempfile::tempdir().unwrap();
    let store_path = store_directory.path().join("tunnels.toml");
    std::fs::write(&store_path, store_text).unwrap();
    let (store, _) = Store::open(store_path.clone()).unwrap();
    let connector = workspace.connector_dependencies(
        &config_path,
        support::quiet_prompter(),
        support::rejecting_verifier(),
        Arc::new(support::NoAgentLocator),
    );
    let engine = Engine::start(EngineDependencies {
        connector,
        clock: Arc::new(SystemClock),
        store,
        port_scanner: Arc::new(FakePortScanner::default()),
        x11_environment: Arc::new(FakeX11Environment::default()),
        x11_servers: Arc::new(FakeX11Servers::default()),
        state_store: Arc::new(MemoryStateStore::default()),
        keyring: None,
        program_locator: Arc::new(FakeProgramLocator::default()),
        preset_platform: PresetPlatform::Unix,
    })
    .unwrap();
    Harness {
        _workspace: workspace,
        _store_directory: store_directory,
        store_path,
        engine,
    }
}

fn new_tunnel(host: &str, kind: TunnelKind, listen_port: u16) -> NewTunnel {
    NewTunnel {
        host: host.to_owned(),
        kind,
        listen_port,
        listen_address: None,
        target_host: None,
        target_port: None,
        tags: Vec::new(),
        open: OpenOptions::default(),
        standby: None,
    }
}

fn file_entry(host: &str, kind: &str, port: u16, extra: &str) -> String {
    format!("\n[[tunnels]]\nhost = \"{host}\"\nkind = \"{kind}\"\nlisten_port = {port}\n{extra}")
}

fn export_file(entries: &[String]) -> String {
    format!(
        "format = \"tunneler-export\"\nversion = 1\n{}",
        entries.concat()
    )
}

fn saved_tunnels(harness: &Harness) -> Vec<TunnelDefinition> {
    Store::open(harness.store_path.clone())
        .unwrap()
        .0
        .document()
        .tunnels
}

type Route = (
    String,
    TunnelKind,
    u16,
    Option<String>,
    Option<String>,
    Option<u16>,
    Vec<String>,
    OpenOptions,
    Option<bool>,
);

fn route_of(definition: &TunnelDefinition) -> Route {
    (
        definition.host.clone(),
        definition.kind,
        definition.listen_port,
        definition.listen_address.clone(),
        definition.target_host.clone(),
        definition.target_port,
        definition.tags.clone(),
        definition.open.clone(),
        definition.standby,
    )
}

fn neighbours_store(first_id: TunnelId, second_id: TunnelId) -> String {
    format!(
        "# my hand-written tunnels\nversion = 1\n\n[[tunnels]]\n# the web app of nova\nid = \"{first_id}\"\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3001\ntags = [\"dev\"]\nopen = {{ path = \"/admin\" }}\n\n[[tunnels]]\nid = \"{second_id}\"\nhost = \"atlas\"\nkind = \"local\"\nlisten_port = 3002\ntags = [\"mail\"]\n"
    )
}

#[tokio::test]
async fn should_keep_every_neighbour_unchanged_when_an_import_adds_one_tunnel() {
    let (first_id, second_id) = (TunnelId::generate(), TunnelId::generate());
    let harness = start(&neighbours_store(first_id, second_id));
    let before = saved_tunnels(&harness);
    let file = export_file(&[
        file_entry("nova", "local", 4000, "tags = [\"dev\"]\n"),
        file_entry(
            "nova",
            "local",
            3001,
            "tags = [\"dev\", \"extra\"]\nopen = { path = \"/admin\" }\n",
        ),
        file_entry("atlas", "local", 3002, "target_port = 80\n"),
    ]);

    let report = harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0, 1, 2])
        .await
        .unwrap();

    let after = saved_tunnels(&harness);
    assert_eq!(report.added.len(), 1);
    assert_eq!(report.added[0].index, 0);
    assert_eq!(
        report
            .skipped
            .iter()
            .map(|skipped| skipped.index)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(after.len(), 3);
    assert_eq!(after[..2], before[..]);
    assert_eq!(after[2].id, report.added[0].id);
    assert_eq!(after[2].listen_port, 4000);
    assert_eq!(after[2].tags, vec!["dev".to_owned()]);
    let text = std::fs::read_to_string(&harness.store_path).unwrap();
    assert!(text.contains("# my hand-written tunnels"));
    assert!(text.contains("# the web app of nova"));
}

#[tokio::test]
async fn should_say_why_each_requested_entry_was_skipped_when_it_is_not_importable() {
    let (first_id, second_id) = (TunnelId::generate(), TunnelId::generate());
    let harness = start(&neighbours_store(first_id, second_id));
    let file = export_file(&[
        file_entry(
            "nova",
            "local",
            3001,
            "tags = [\"dev\", \"extra\"]\nopen = { path = \"/admin\" }\n",
        ),
        file_entry("atlas", "local", 3002, "target_port = 80\n"),
        file_entry("nova", "local", 5000, ""),
        file_entry("nova", "dynamic", 5000, ""),
    ]);

    let report = harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0, 1, 2, 3])
        .await
        .unwrap();

    let statuses: Vec<ConfigImportStatus> = report
        .skipped
        .iter()
        .map(|skipped| skipped.status.clone())
        .collect();
    assert_eq!(
        statuses,
        vec![
            ConfigImportStatus::AlreadyConfigured {
                existing_id: first_id,
                ignored_tags: vec!["extra".to_owned()]
            },
            ConfigImportStatus::ListenConflict {
                holder_id: second_id
            },
            ConfigImportStatus::DuplicateInFile { earlier_index: 2 },
        ]
    );
}

#[tokio::test]
async fn should_skip_the_entry_and_overwrite_nothing_when_the_store_changed_between_preview_and_apply()
 {
    let harness = start("version = 1\n");
    let file = export_file(&[
        file_entry("nova", "local", 4000, ""),
        file_entry("nova", "local", 4001, ""),
    ]);
    let plan = harness
        .engine
        .preview_configuration_import(&file, &BTreeMap::new(), &known_hosts())
        .await
        .unwrap();
    assert!(plan.entries.iter().all(|entry| entry.selected_by_default));
    let rival = harness
        .engine
        .create_tunnel(new_tunnel("atlas", TunnelKind::Local, 4000))
        .await
        .unwrap();

    let report = harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0, 1])
        .await
        .unwrap();

    assert_eq!(
        report
            .added
            .iter()
            .map(|added| added.index)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(
        report.skipped[0].status,
        ConfigImportStatus::ListenConflict { holder_id: rival }
    );
    let saved = saved_tunnels(&harness);
    assert_eq!(saved.len(), 2);
    assert_eq!(saved[0].id, rival);
    assert_eq!(saved[0].host, "atlas");
}

#[tokio::test]
async fn should_see_a_tunnel_added_to_the_file_by_hand_when_the_store_changed_on_disk() {
    let harness = start("version = 1\n");
    let file = export_file(&[file_entry("nova", "local", 4000, "")]);
    std::fs::write(
        &harness.store_path,
        "version = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 4000\n",
    )
    .unwrap();

    let report = harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0])
        .await
        .unwrap();

    assert_eq!(report.added, Vec::new());
    assert!(matches!(
        report.skipped[0].status,
        ConfigImportStatus::AlreadyConfigured { .. }
    ));
    assert_eq!(saved_tunnels(&harness).len(), 1);
}

#[tokio::test]
async fn should_refuse_and_leave_the_file_alone_when_the_store_on_disk_is_invalid() {
    let harness = start("version = 1\n");
    let broken = "version = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 0\n";
    std::fs::write(&harness.store_path, broken).unwrap();
    let file = export_file(&[file_entry("nova", "local", 4000, "")]);

    let outcome = harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0])
        .await;

    assert!(matches!(outcome, Err(EngineError::StoreInvalid(_))));
    assert_eq!(
        std::fs::read_to_string(&harness.store_path).unwrap(),
        broken
    );
}

#[tokio::test]
async fn should_refuse_again_when_the_invalid_store_was_already_seen_by_a_previous_apply() {
    let harness = start("version = 1\n");
    std::fs::write(&harness.store_path, "this is = not [valid\n").unwrap();
    let file = export_file(&[file_entry("nova", "local", 4000, "")]);

    for _ in 0..2 {
        let outcome = harness
            .engine
            .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0])
            .await;
        assert!(matches!(outcome, Err(EngineError::StoreInvalid(_))));
    }
}

#[tokio::test]
async fn should_start_the_imported_tunnels_stopped_and_publish_a_reload() {
    let harness = start("version = 1\n");
    let mut events = harness.engine.subscribe();
    let file = export_file(&[file_entry("nova", "local", 4000, "")]);

    let report = harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0])
        .await
        .unwrap();

    let snapshot = harness.engine.snapshot();
    assert_eq!(snapshot.tunnels.len(), 1);
    assert_eq!(snapshot.tunnels[0].definition.id, report.added[0].id);
    assert_eq!(snapshot.tunnels[0].state, TunnelState::Stopped);
    assert!(matches!(events.try_recv(), Ok(EngineEvent::ConfigReloaded)));
}

#[tokio::test]
async fn should_write_nothing_and_publish_nothing_when_no_requested_entry_is_importable() {
    let harness = start("version = 1\n");
    harness
        .engine
        .create_tunnel(new_tunnel("nova", TunnelKind::Local, 4000))
        .await
        .unwrap();
    let before = std::fs::read_to_string(&harness.store_path).unwrap();
    let mut events = harness.engine.subscribe();
    let file = export_file(&[file_entry("nova", "local", 4000, "")]);

    harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0])
        .await
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(&harness.store_path).unwrap(),
        before
    );
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn should_add_a_risky_entry_only_when_its_index_is_requested() {
    let harness = start("version = 1\n");
    let file = export_file(&[
        file_entry("nova", "local", 4000, "listen_address = \"0.0.0.0\"\n"),
        file_entry("nova", "local", 4001, ""),
    ]);
    let plan = harness
        .engine
        .preview_configuration_import(&file, &BTreeMap::new(), &known_hosts())
        .await
        .unwrap();
    assert_eq!(
        plan.entries[0].risks,
        vec![ConfigImportRisk::NonLoopbackListen {
            address: "0.0.0.0".to_owned()
        }]
    );
    let selected: Vec<usize> = plan
        .entries
        .iter()
        .filter(|entry| entry.selected_by_default)
        .map(|entry| entry.index)
        .collect();

    let default_report = harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &selected)
        .await
        .unwrap();
    let explicit_report = harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0])
        .await
        .unwrap();

    assert_eq!(default_report.added.len(), 1);
    assert_eq!(explicit_report.added.len(), 1);
    assert_eq!(saved_tunnels(&harness).len(), 2);
}

#[tokio::test]
async fn should_recreate_the_same_routes_when_an_export_is_imported_into_an_empty_store() {
    let source = start("version = 1\n");
    let rich = NewTunnel {
        listen_address: Some("127.0.0.2".to_owned()),
        target_host: Some("db.internal".to_owned()),
        target_port: Some(5432),
        tags: vec!["dev".to_owned(), "db".to_owned()],
        open: OpenOptions {
            scheme: Some(OpenScheme::Https),
            path: Some("/admin".to_owned()),
            ..OpenOptions::default()
        },
        standby: Some(true),
        ..new_tunnel("nova", TunnelKind::Local, 3333)
    };
    for tunnel in [
        rich,
        new_tunnel("atlas", TunnelKind::Remote, 8080),
        new_tunnel("nova", TunnelKind::Dynamic, 1080),
    ] {
        source.engine.create_tunnel(tunnel).await.unwrap();
    }
    let export = source
        .engine
        .export_configuration(&ConfigExportFilter::default());
    let target = start("version = 1\n");

    let report = target
        .engine
        .apply_configuration_import(
            &export.contents,
            &BTreeMap::new(),
            &known_hosts(),
            &[0, 1, 2],
        )
        .await
        .unwrap();

    let original: Vec<Route> = saved_tunnels(&source).iter().map(route_of).collect();
    let imported: Vec<Route> = saved_tunnels(&target).iter().map(route_of).collect();
    assert_eq!(export.tunnel_count, 3);
    assert_eq!(report.added.len(), 3);
    assert_eq!(imported, original);
    let source_ids: Vec<TunnelId> = saved_tunnels(&source)
        .iter()
        .map(|tunnel| tunnel.id)
        .collect();
    assert!(
        saved_tunnels(&target)
            .iter()
            .all(|tunnel| !source_ids.contains(&tunnel.id))
    );
}

#[tokio::test]
async fn should_export_only_saved_tunnels_and_nothing_but_tunnels() {
    let harness = start(
        "version = 1\n\n[settings]\nstandby = true\ndefault_bind_address = \"0.0.0.0\"\n\n[hosts.nova]\nx11_display = \":7\"\n\n[[openers]]\nid = \"viewer\"\nname = \"Viewer\"\ncommand = \"viewer\"\nargs = []\n\n[[x11_launchers]]\nhost = \"nova\"\nname = \"term\"\ncommand = \"xterm\"\n",
    );
    harness
        .engine
        .create_tunnel(new_tunnel("nova", TunnelKind::Local, 3333))
        .await
        .unwrap();
    harness
        .engine
        .start_temporary_tunnel(new_tunnel("atlas", TunnelKind::Local, 3334), None)
        .await
        .unwrap();

    let export = harness
        .engine
        .export_configuration(&ConfigExportFilter::default());

    assert_eq!(export.tunnel_count, 1);
    assert!(
        export
            .contents
            .starts_with("# Tunneler export (format version 1)\n")
    );
    assert!(!export.contents.contains("3334"));
    assert!(!export.contents.contains("id ="));
    for forbidden in [
        "settings",
        "hosts",
        "openers",
        "x11_launchers",
        "viewer",
        "xterm",
        "0.0.0.0",
        ":7",
    ] {
        assert!(!export.contents.contains(forbidden), "{forbidden}");
    }
}

#[tokio::test]
async fn should_export_the_union_of_the_host_and_tag_filters() {
    let harness = start("version = 1\n");
    for (host, port, tag) in [
        ("nova", 3001, "dev"),
        ("nova", 3002, "mail"),
        ("atlas", 3003, "dev"),
        ("atlas", 3004, "ops"),
    ] {
        let tunnel = NewTunnel {
            tags: vec![tag.to_owned()],
            ..new_tunnel(host, TunnelKind::Local, port)
        };
        harness.engine.create_tunnel(tunnel).await.unwrap();
    }

    let by_host = harness.engine.export_configuration(&ConfigExportFilter {
        hosts: vec!["nova".to_owned()],
        tags: Vec::new(),
    });
    let by_tag = harness.engine.export_configuration(&ConfigExportFilter {
        hosts: Vec::new(),
        tags: vec!["ops".to_owned()],
    });
    let union = harness.engine.export_configuration(&ConfigExportFilter {
        hosts: vec!["nova".to_owned()],
        tags: vec!["ops".to_owned()],
    });
    let none = harness
        .engine
        .export_configuration(&ConfigExportFilter::default());

    assert_eq!(
        (
            by_host.tunnel_count,
            by_tag.tunnel_count,
            union.tunnel_count,
            none.tunnel_count
        ),
        (2, 1, 3, 4)
    );
}

#[tokio::test]
async fn should_drop_the_opener_only_when_it_exists_neither_in_the_store_nor_on_this_machine() {
    let harness = start(
        "version = 1\n\n[[openers]]\nid = \"viewer\"\nname = \"Viewer\"\ncommand = \"viewer\"\nargs = []\n",
    );
    let file = export_file(&[
        file_entry("nova", "local", 4000, "open = { with = \"viewer\" }\n"),
        file_entry("nova", "local", 4001, "open = { with = \"browser\" }\n"),
        file_entry("nova", "local", 4002, "open = { with = \"ghost\" }\n"),
    ]);

    let plan = harness
        .engine
        .preview_configuration_import(&file, &BTreeMap::new(), &known_hosts())
        .await
        .unwrap();

    let kept: Vec<Option<&str>> = plan
        .entries
        .iter()
        .map(|entry| entry.tunnel.open.with.as_deref())
        .collect();
    assert_eq!(kept, vec![Some("viewer"), Some("browser"), None]);
}

#[tokio::test]
async fn should_wait_for_the_background_detection_of_the_openers_before_dropping_one() {
    let workspace = Workspace::new();
    let config_path = workspace.path().join("ssh_config");
    std::fs::write(&config_path, "").unwrap();
    let store_directory = tempfile::tempdir().unwrap();
    let store_path = store_directory.path().join("tunnels.toml");
    let (store, _) = Store::open(store_path).unwrap();
    let locator = Arc::new(FakeProgramLocator::default());
    locator.install("remmina");
    let engine = Engine::start(EngineDependencies {
        connector: workspace.connector_dependencies(
            &config_path,
            support::quiet_prompter(),
            support::rejecting_verifier(),
            Arc::new(support::NoAgentLocator),
        ),
        clock: Arc::new(SystemClock),
        store,
        port_scanner: Arc::new(FakePortScanner::default()),
        x11_environment: Arc::new(FakeX11Environment::default()),
        x11_servers: Arc::new(FakeX11Servers::default()),
        state_store: Arc::new(MemoryStateStore::default()),
        keyring: None,
        program_locator: locator.clone(),
        preset_platform: PresetPlatform::Unix,
    })
    .unwrap();
    tokio::task::yield_now().await;
    let file = export_file(&[file_entry(
        "nova",
        "local",
        4000,
        "open = { with = \"remmina-vnc\" }\n",
    )]);

    let plan = engine
        .preview_configuration_import(&file, &BTreeMap::new(), &known_hosts())
        .await
        .unwrap();

    assert_eq!(
        plan.entries[0].tunnel.open.with.as_deref(),
        Some("remmina-vnc")
    );
    assert_eq!(plan.entries[0].warnings, Vec::new());
}

#[tokio::test]
async fn should_not_resolve_any_host_when_previewing() {
    let workspace = Workspace::new();
    let config_path = workspace.path().join("ssh_config");
    let marker = workspace.path().join("match_exec_ran");
    std::fs::write(
        &config_path,
        format!("Match exec \"touch {}\"\n  Port 2200\n", marker.display()),
    )
    .unwrap();
    let store_directory = tempfile::tempdir().unwrap();
    let (store, _) = Store::open(store_directory.path().join("tunnels.toml")).unwrap();
    let engine = Engine::start(EngineDependencies {
        connector: workspace.connector_dependencies(
            &config_path,
            support::quiet_prompter(),
            support::rejecting_verifier(),
            Arc::new(support::NoAgentLocator),
        ),
        clock: Arc::new(SystemClock),
        store,
        port_scanner: Arc::new(FakePortScanner::default()),
        x11_environment: Arc::new(FakeX11Environment::default()),
        x11_servers: Arc::new(FakeX11Servers::default()),
        state_store: Arc::new(MemoryStateStore::default()),
        keyring: None,
        program_locator: Arc::new(FakeProgramLocator::default()),
        preset_platform: PresetPlatform::Unix,
    })
    .unwrap();
    let file = export_file(&[file_entry("oldbox", "local", 4000, "")]);

    let plan = engine
        .preview_configuration_import(&file, &BTreeMap::new(), &known_hosts())
        .await
        .unwrap();

    assert_eq!(plan.unknown_hosts, vec!["oldbox".to_owned()]);
    assert!(!marker.exists());
}

#[tokio::test]
async fn should_import_under_the_mapped_host_when_the_host_map_renames_it() {
    let harness = start("version = 1\n");
    let file = export_file(&[file_entry("oldbox", "local", 4000, "")]);
    let map = BTreeMap::from([("oldbox".to_owned(), "nova".to_owned())]);

    let report = harness
        .engine
        .apply_configuration_import(&file, &map, &known_hosts(), &[0])
        .await
        .unwrap();

    assert_eq!(report.added.len(), 1);
    assert_eq!(saved_tunnels(&harness)[0].host, "nova");
}

#[tokio::test]
async fn should_read_only_the_tunnels_when_the_file_is_a_raw_tunnels_file() {
    let harness = start("version = 1\n");
    let raw = format!(
        "version = 1\n\n[settings]\nstandby = true\n\n[[openers]]\nid = \"x\"\nname = \"x\"\ncommand = \"x\"\nargs = []\n\n[[tunnels]]\nid = \"{}\"\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 4000\n",
        TunnelId::generate()
    );

    let report = harness
        .engine
        .apply_configuration_import(&raw, &BTreeMap::new(), &known_hosts(), &[0])
        .await
        .unwrap();

    let document = Store::open(harness.store_path.clone())
        .unwrap()
        .0
        .document();
    assert_eq!(report.added.len(), 1);
    assert_eq!(document.tunnels.len(), 1);
    assert!(!document.settings.standby);
    assert_eq!(document.openers, Vec::new());
}

#[tokio::test]
async fn should_report_each_reason_when_the_file_is_refused() {
    let harness = start("version = 1\n");
    let preview = |contents: String| {
        let engine = harness.engine.clone();
        async move {
            engine
                .preview_configuration_import(&contents, &BTreeMap::new(), &known_hosts())
                .await
        }
    };

    let syntax = preview("format = ".to_owned()).await;
    let foreign = preview("format = \"other\"\n".to_owned()).await;
    let newer = preview("format = \"tunneler-export\"\nversion = 2\n".to_owned()).await;
    let unsafe_text = preview(export_file(&[file_entry(
        "nova",
        "local",
        1,
        "target_host = \"a\\u0007\"\n",
    )]))
    .await;
    let too_large = preview(format!("# {}", "a".repeat(MAX_IMPORT_FILE_BYTES))).await;
    let too_many = preview(export_file(&vec![file_entry("nova", "local", 1, ""); 1001])).await;

    assert!(matches!(
        syntax,
        Err(EngineError::InvalidImportFile(
            ImportFileError::Syntax { .. }
        ))
    ));
    assert!(matches!(
        foreign,
        Err(EngineError::InvalidImportFile(
            ImportFileError::NotTunnelerFile
        ))
    ));
    assert!(matches!(
        newer,
        Err(EngineError::InvalidImportFile(
            ImportFileError::NewerVersion {
                found: 2,
                supported: 1
            }
        ))
    ));
    assert!(matches!(
        unsafe_text,
        Err(EngineError::InvalidImportFile(
            ImportFileError::UnsafeCharacters {
                entry: 0,
                field: "target_host"
            }
        ))
    ));
    assert!(matches!(
        too_large,
        Err(EngineError::InvalidImportFile(
            ImportFileError::TooLarge { .. }
        ))
    ));
    assert!(matches!(
        too_many,
        Err(EngineError::InvalidImportFile(
            ImportFileError::TooManyEntries {
                found: 1001,
                limit: 1000
            }
        ))
    ));
}

#[tokio::test]
async fn should_classify_routes_against_the_saved_tunnels_and_the_earlier_candidates() {
    let harness = start("version = 1\n");
    let existing = harness
        .engine
        .create_tunnel(new_tunnel("nova", TunnelKind::Local, 3333))
        .await
        .unwrap();
    let candidate = |host: &str, kind: TunnelKind, port: u16| ImportRoute {
        host: host.to_owned(),
        kind,
        listen_port: port,
        listen_address: None,
        target_host: None,
        target_port: None,
    };

    let conflicts = harness.engine.classify_import_routes(&[
        candidate("nova", TunnelKind::Local, 3333),
        candidate("atlas", TunnelKind::Dynamic, 3333),
        candidate("nova", TunnelKind::Local, 4000),
        candidate("nova", TunnelKind::Local, 4000),
    ]);

    assert_eq!(
        conflicts,
        vec![
            Some(RouteConflict::AlreadyConfigured {
                existing_id: existing
            }),
            Some(RouteConflict::ListenConflict {
                holder_id: existing
            }),
            None,
            Some(RouteConflict::DuplicateInImport { earlier_index: 2 }),
        ]
    );
}

const BAD_HOSTS: [&str; 7] = [
    "",
    "-oProxyCommand=touch",
    "a b",
    "a\u{1b}b",
    "a\u{202e}b",
    "a;b",
    "a$(b)",
];

fn is_validation_failure(outcome: &Result<impl std::fmt::Debug, EngineError>) -> bool {
    matches!(
        outcome,
        Err(EngineError::Store(StoreError::Validation(errors)))
            if errors.iter().any(|error| matches!(error, ValidationError::InvalidSshHost { .. } | ValidationError::EmptyHost { .. }))
    )
}

#[tokio::test]
async fn should_refuse_a_tunnel_creation_when_the_host_breaks_the_host_rule() {
    let harness = start("version = 1\n");

    for host in BAD_HOSTS {
        let outcome = harness
            .engine
            .create_tunnel(new_tunnel(host, TunnelKind::Local, 3333))
            .await;
        assert!(is_validation_failure(&outcome), "{host:?}");
    }
    assert_eq!(saved_tunnels(&harness), Vec::new());
}

#[tokio::test]
async fn should_refuse_a_temporary_tunnel_when_the_host_breaks_the_host_rule() {
    let harness = start("version = 1\n");

    for host in BAD_HOSTS {
        let outcome = harness
            .engine
            .start_temporary_tunnel(new_tunnel(host, TunnelKind::Local, 3333), None)
            .await;
        assert!(is_validation_failure(&outcome), "{host:?}");
    }
    assert_eq!(harness.engine.snapshot().tunnels, Vec::new());
}

#[tokio::test]
async fn should_refuse_a_host_change_to_a_bad_alias_and_allow_an_unchanged_odd_host() {
    let odd_id = TunnelId::generate();
    let harness = start(&format!(
        "version = 1\n\n[[tunnels]]\nid = \"{odd_id}\"\nhost = \"odd;host\"\nkind = \"local\"\nlisten_port = 4000\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 4001\n"
    ));
    let tunnels = harness.engine.snapshot().tunnels;
    let odd = tunnels[0].definition.clone();
    let nova = tunnels[1].definition.clone();

    let unchanged = harness
        .engine
        .update_tunnel(TunnelDefinition {
            tags: vec!["still-editable".to_owned()],
            ..odd.clone()
        })
        .await;
    let changed_to_bad = harness
        .engine
        .update_tunnel(TunnelDefinition {
            host: "a b".to_owned(),
            ..nova.clone()
        })
        .await;
    let changed_to_good = harness
        .engine
        .update_tunnel(TunnelDefinition {
            host: "atlas".to_owned(),
            ..nova
        })
        .await;

    assert!(unchanged.is_ok());
    assert!(is_validation_failure(&changed_to_bad));
    assert!(changed_to_good.is_ok());
}

#[tokio::test]
async fn should_refuse_a_tunnel_of_the_file_whose_host_breaks_the_host_rule_and_still_import_the_others()
 {
    let harness = start("version = 1\n");
    let file = export_file(&[
        file_entry("-oProxyCommand=touch", "local", 4000, ""),
        file_entry("nova", "local", 4001, ""),
    ]);

    let report = harness
        .engine
        .apply_configuration_import(&file, &BTreeMap::new(), &known_hosts(), &[0, 1])
        .await
        .unwrap();

    assert_eq!(report.added.len(), 1);
    assert!(matches!(
        report.skipped[0].status,
        ConfigImportStatus::Invalid { .. }
    ));
    assert_eq!(saved_tunnels(&harness).len(), 1);
}
