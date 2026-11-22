//! The engine side of the configuration export and import.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::blocking::run_blocking;
use crate::openers::available_openers;
use crate::runtime::engine::{Engine, EngineError};
use crate::runtime::events::EngineEvent;
use crate::store::{LoadOutcome, StoreError, TunnelDefinition, TunnelId};
use crate::transfer::{
    ConfigExport, ConfigExportFilter, ConfigImportAdded, ConfigImportPlan, ConfigImportReport,
    ImportFileError, ImportRoute, ParsedImportFile, PlanContext, RouteConflict, classify_routes,
    export_tunnels, parse_import_file, plan_parsed, select_importable,
};

impl Engine {
    /// Renders the saved tunnels the filter selects as an export file.
    /// Temporary tunnels are never exported.
    pub fn export_configuration(&self, filter: &ConfigExportFilter) -> ConfigExport {
        let tunnels = self.inner.catalog().persistent_definitions();
        export_tunnels(&tunnels, filter)
    }

    /// Classifies candidate routes against the saved tunnels and the
    /// settings as they are now, and against the earlier candidates, with
    /// the normalization of the validation of the store. The result has one
    /// entry per candidate, in order.
    pub fn classify_import_routes(&self, candidates: &[ImportRoute]) -> Vec<Option<RouteConflict>> {
        let existing = self.inner.catalog().persistent_definitions();
        let settings = self.inner.settings.borrow().clone();
        classify_routes(&existing, &settings, candidates)
    }

    /// Plans the import of an export file or of a `tunnels.toml`, without
    /// touching anything: what each tunnel of the file would become, what
    /// stops it, what is risky about it.
    ///
    /// `host_map` renames hosts of the file (`file host` to `local alias`)
    /// before the entries are checked; keys no entry uses are ignored.
    /// `known_hosts` are the aliases the `ssh_config` names, as the host
    /// listing gives them: nothing is resolved here, so no `Match exec` or
    /// `ProxyCommand` of the `ssh_config` runs. When the engine has just
    /// started, this waits, for a second at most, for the detection of the
    /// openers that is already running.
    pub async fn preview_configuration_import(
        &self,
        contents: &str,
        host_map: &BTreeMap<String, String>,
        known_hosts: &[String],
    ) -> Result<ConfigImportPlan, EngineError> {
        self.wait_for_running_detection().await;
        let parsed = parse_in_background(contents).await?;
        Ok(self.plan_configuration_import(parsed, host_map, known_hosts))
    }

    /// Adds the tunnels at `indexes` of the file, after planning it again
    /// against the saved tunnels as they are at that moment: an entry that
    /// stopped being importable since the preview is skipped, with its
    /// current status, and nothing existing is ever modified or removed. An
    /// entry the plan marks risky is added when its index is requested. The
    /// new tunnels start stopped, and the store is written once.
    ///
    /// Fails with [`EngineError::StoreInvalid`] when the tunnels file on disk
    /// is invalid right now, so that it is not written over.
    pub async fn apply_configuration_import(
        &self,
        contents: &str,
        host_map: &BTreeMap<String, String>,
        known_hosts: &[String],
        indexes: &[usize],
    ) -> Result<ConfigImportReport, EngineError> {
        self.wait_for_running_detection().await;
        let parsed = parse_in_background(contents).await?;
        let _serialized = self.inner.mutation.lock().await;
        self.ensure_running()?;
        self.refresh_store_from_disk().await?;
        let plan = self.plan_configuration_import(parsed, host_map, known_hosts);
        let selection = select_importable(plan, indexes);
        let added = self.add_imported_tunnels(selection.to_add).await?;
        Ok(ConfigImportReport {
            added,
            skipped: selection.skipped,
        })
    }

    fn plan_configuration_import(
        &self,
        parsed: ParsedImportFile,
        host_map: &BTreeMap<String, String>,
        known_hosts: &[String],
    ) -> ConfigImportPlan {
        let existing = self.inner.catalog().persistent_definitions();
        let settings = self.inner.settings.borrow().clone();
        let opener_ids = self.available_opener_ids();
        let context = PlanContext {
            existing: &existing,
            settings: &settings,
            known_hosts,
            opener_ids: &opener_ids,
        };
        plan_parsed(parsed, host_map, &context)
    }

    fn available_opener_ids(&self) -> Vec<String> {
        let user_openers = self.inner.store.document().openers;
        let detected = self.inner.detected_openers().clone();
        available_openers(&detected, &user_openers)
            .into_iter()
            .map(|available| available.opener.id)
            .collect()
    }

    async fn refresh_store_from_disk(&self) -> Result<(), EngineError> {
        let store = Arc::clone(&self.inner.store);
        run_blocking(move || store.refresh_from_disk())
            .await
            .map_err(unusable_store_error)?;
        self.apply_store_document_if_stale().await;
        Ok(())
    }

    /// The watcher may have loaded an edit into the store without the
    /// engine having applied it yet, because its reload waits for the lock
    /// this call holds. The plan and the write must start from the store.
    async fn apply_store_document_if_stale(&self) {
        let document = self.inner.store.document();
        let is_current = self.inner.catalog().persistent_definitions() == document.tunnels
            && *self.inner.settings.borrow() == document.settings;
        if is_current {
            return;
        }
        let outcome = LoadOutcome {
            document,
            was_normalized: self.inner.store.has_pending_normalization(),
        };
        self.inner.apply_reload_locked(outcome).await;
    }

    async fn add_imported_tunnels(
        &self,
        to_add: Vec<(usize, crate::runtime::NewTunnel)>,
    ) -> Result<Vec<ConfigImportAdded>, EngineError> {
        if to_add.is_empty() {
            return Ok(Vec::new());
        }
        let created: Vec<(usize, TunnelDefinition)> = to_add
            .into_iter()
            .map(|(index, tunnel)| (index, tunnel.into_definition(TunnelId::generate())))
            .collect();
        let mut tunnels = self.inner.catalog().persistent_definitions();
        tunnels.extend(created.iter().map(|(_, definition)| definition.clone()));
        self.inner.persist_tunnels(tunnels).await?;
        let mut added = Vec::with_capacity(created.len());
        for (index, definition) in created {
            added.push(ConfigImportAdded {
                index,
                id: definition.id,
            });
            self.inner.catalog().add(definition);
        }
        self.inner.bus.publish(EngineEvent::ConfigReloaded);
        Ok(added)
    }
}

/// Parses off the async runtime: the parser runs on a thread of its own with
/// a large stack, and this waits for it on the blocking pool.
async fn parse_in_background(contents: &str) -> Result<ParsedImportFile, ImportFileError> {
    let owned = contents.to_owned();
    run_blocking(move || parse_import_file(&owned)).await
}

fn unusable_store_error(error: StoreError) -> EngineError {
    match error {
        StoreError::Parse { .. }
        | StoreError::UnsupportedVersion { .. }
        | StoreError::Validation(_) => EngineError::StoreInvalid(error.to_string()),
        other => EngineError::Store(other),
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use async_trait::async_trait;
    use ssh_config::{AgentSocket, ConfigSources, EnvironmentError, LocalEnvironment};
    use zeroize::Zeroizing;

    use super::*;
    use crate::agent::{AgentEndpoint, AgentLocator};
    use crate::clock::SystemClock;
    use crate::connector::ConnectorDependencies;
    use crate::external::SystemPortScanner;
    use crate::host_config::SshConfigResolver;
    use crate::host_key::{HostKeyDecision, HostKeyVerifier, RejectedHostKey, UnknownHostKey};
    use crate::openers::{PresetPlatform, SystemProgramLocator};
    use crate::prompter::{
        CredentialPrompter, KeyboardInteractiveRequest, PassphraseAnswer, PassphraseRequest,
        PasswordRequest, PromptError,
    };
    use crate::runtime::engine::EngineDependencies;
    use crate::store::{FileStateStore, Store, StoreEvent};
    use crate::x11::{LocalX11Servers, SystemX11Environment, X11Server};

    struct NoPrompts;

    #[async_trait]
    impl CredentialPrompter for NoPrompts {
        async fn passphrase(
            &self,
            _request: PassphraseRequest,
        ) -> Result<Option<PassphraseAnswer>, PromptError> {
            Ok(None)
        }

        async fn password(
            &self,
            _request: PasswordRequest,
        ) -> Result<Option<Zeroizing<String>>, PromptError> {
            Ok(None)
        }

        async fn keyboard_interactive(
            &self,
            _request: KeyboardInteractiveRequest,
        ) -> Result<Option<Vec<Zeroizing<String>>>, PromptError> {
            Ok(None)
        }
    }

    struct RejectingVerifier;

    #[async_trait]
    impl HostKeyVerifier for RejectingVerifier {
        async fn decide_unknown(&self, _host_key: UnknownHostKey) -> HostKeyDecision {
            HostKeyDecision::Reject
        }

        async fn report_rejected(&self, _rejection: RejectedHostKey) {}
    }

    struct NoAgent;

    impl AgentLocator for NoAgent {
        fn locate(&self, _socket: Option<&AgentSocket>) -> Vec<AgentEndpoint> {
            Vec::new()
        }
    }

    struct NoX11Servers;

    impl LocalX11Servers for NoX11Servers {
        fn find(&self) -> Option<X11Server> {
            None
        }

        fn start(&self, _server: &X11Server, _display_number: u16) -> io::Result<()> {
            Ok(())
        }
    }

    struct EmptyEnvironment;

    impl LocalEnvironment for EmptyEnvironment {
        fn variable(&self, _name: &str) -> Option<String> {
            None
        }

        fn user_name(&self) -> String {
            "tester".to_owned()
        }

        fn home_directory(&self) -> PathBuf {
            PathBuf::from("/nonexistent")
        }

        fn host_name(&self) -> String {
            "testbed".to_owned()
        }

        fn user_id(&self) -> u32 {
            1000
        }

        fn system_config_directory(&self) -> PathBuf {
            PathBuf::from("/nonexistent")
        }

        fn command_succeeds(&self, _command: &str) -> Result<bool, EnvironmentError> {
            Ok(false)
        }
    }

    const KNOWN_HOSTS: [&str; 1] = ["nova"];
    const EDITED_STORE: &str = "version = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3001\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3002\ntags = [\"hand-edit\"]\n";
    const IMPORT_FILE: &str = "format = \"tunneler-export\"\nversion = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 4000\n";

    fn start_engine(directory: &tempfile::TempDir, store_text: &str) -> Engine {
        let config_path = directory.path().join("ssh_config");
        std::fs::write(&config_path, "").unwrap();
        let store_path = directory.path().join("tunnels.toml");
        std::fs::write(&store_path, store_text).unwrap();
        let (store, _) = Store::open(store_path).unwrap();
        let resolver =
            SshConfigResolver::new(ConfigSources::user_only(config_path), EmptyEnvironment);
        Engine::start(EngineDependencies {
            connector: ConnectorDependencies {
                resolver: Arc::new(resolver),
                prompter: Arc::new(NoPrompts),
                verifier: Arc::new(RejectingVerifier),
                agent_locator: Arc::new(NoAgent),
            },
            clock: Arc::new(SystemClock),
            store,
            port_scanner: Arc::new(SystemPortScanner),
            x11_environment: Arc::new(SystemX11Environment),
            x11_servers: Arc::new(NoX11Servers),
            state_store: Arc::new(FileStateStore::new(directory.path().join("state.toml"))),
            keyring: None,
            program_locator: Arc::new(SystemProgramLocator),
            preset_platform: PresetPlatform::Unix,
        })
        .unwrap()
    }

    fn ports(tunnels: &[TunnelDefinition]) -> Vec<u16> {
        tunnels.iter().map(|tunnel| tunnel.listen_port).collect()
    }

    #[tokio::test]
    async fn should_keep_an_edit_the_watcher_loaded_but_the_engine_did_not_apply_when_importing() {
        let directory = tempfile::tempdir().unwrap();
        let engine = start_engine(&directory, "version = 1\n");
        let store_path = directory.path().join("tunnels.toml");
        std::fs::write(&store_path, EDITED_STORE).unwrap();
        let queued_event = engine.inner.store.shared().reload_if_changed();
        assert!(queued_event.is_some());
        let known_hosts: Vec<String> = KNOWN_HOSTS.iter().map(|host| (*host).to_owned()).collect();

        let report = engine
            .apply_configuration_import(IMPORT_FILE, &BTreeMap::new(), &known_hosts, &[0])
            .await
            .unwrap();
        if let Some(event) = queued_event {
            engine.inner.handle_store_event(event).await;
        }

        let on_disk = Store::open(store_path).unwrap().0.document().tunnels;
        let in_catalog = engine.inner.catalog().persistent_definitions();
        assert_eq!(report.added.len(), 1);
        assert_eq!(ports(&on_disk), vec![3001, 3002, 4000]);
        assert_eq!(ports(&in_catalog), vec![3001, 3002, 4000]);
        assert_eq!(on_disk[1].tags, vec!["hand-edit".to_owned()]);
    }

    #[tokio::test]
    async fn should_refuse_and_leave_the_file_alone_when_the_watcher_already_saw_the_invalid_file()
    {
        let directory = tempfile::tempdir().unwrap();
        let engine = start_engine(&directory, "version = 1\n");
        let store_path = directory.path().join("tunnels.toml");
        let broken =
            "version = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 0\n";
        std::fs::write(&store_path, broken).unwrap();
        let seen = engine.inner.store.shared().reload_if_changed();
        assert!(matches!(seen, Some(StoreEvent::Invalid(_))));
        let known_hosts: Vec<String> = KNOWN_HOSTS.iter().map(|host| (*host).to_owned()).collect();

        let outcome = engine
            .apply_configuration_import(IMPORT_FILE, &BTreeMap::new(), &known_hosts, &[0])
            .await;

        assert!(matches!(outcome, Err(EngineError::StoreInvalid(_))));
        assert_eq!(std::fs::read_to_string(store_path).unwrap(), broken);
    }
}
