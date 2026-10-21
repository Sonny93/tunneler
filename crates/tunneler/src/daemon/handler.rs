use std::sync::Arc;

use async_trait::async_trait;
use engine::{
    Engine, EngineError, HostConfigResolver, StoreError, TunnelId, X11Error, preview_script,
    preview_ssh_config,
};
use ipc::protocol::{
    AdoptResult, CheckTargetResult, DaemonPathsResult, DeleteTagResult, ForgetSecretsResult,
    HostsResult, IpcError, LogsResult, ProcessParams, QuitParams, RenameTagResult, Request,
    ResponsePayload, SettingsDto, TagParams, TagRenameParams, TunnelIdParams, UiClaimResult,
    X11HostParams, X11LaunchParams,
};
use ipc::{ConnectionId, ConnectionInfo, RequestHandler};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use super::mapping::{
    definition_from_dto, health_to_dto, import_preview_to_dto, log_entry_to_dto,
    new_temporary_tunnel_from_spec, new_tunnel_from_spec, resolved_host_to_dto, settings_from_dto,
    snapshot_to_dto, tunnel_to_dto,
};
use super::prompts::PromptBroker;
use super::tray::TrayStatus;
use super::ui::UiCoordinator;

/// Maps each IPC request to calls on the engine, the store, the prompt broker
/// and the UI coordinator.
pub(crate) struct DaemonHandler {
    engine: Engine,
    prompts: Arc<PromptBroker>,
    hosts: Arc<dyn HostConfigResolver>,
    quit_requested: CancellationToken,
    ui: Arc<UiCoordinator>,
    tray_status: watch::Receiver<TrayStatus>,
}

impl DaemonHandler {
    pub(crate) fn new(
        engine: Engine,
        prompts: Arc<PromptBroker>,
        hosts: Arc<dyn HostConfigResolver>,
        quit_requested: CancellationToken,
        ui: Arc<UiCoordinator>,
        tray_status: watch::Receiver<TrayStatus>,
    ) -> Self {
        Self {
            engine,
            prompts,
            hosts,
            quit_requested,
            ui,
            tray_status,
        }
    }

    /// Answers one request. `requester` is the connection that sent it, when
    /// known.
    pub(crate) async fn dispatch(
        &self,
        requester: Option<ConnectionId>,
        request: Request,
    ) -> Result<ResponsePayload, IpcError> {
        match request {
            Request::Hello(_) => Err(IpcError::invalid_request("`hello` was already received")),
            Request::StateSnapshot {} => Ok(self.snapshot()),
            Request::TunnelCreate(spec) => self.create_tunnel(spec).await,
            Request::TunnelUpdate(definition) => self.update_tunnel(definition).await,
            Request::TunnelStartTemporary(spec) => self.start_temporary_tunnel(spec).await,
            Request::TunnelPersist(params) => self.persist_tunnel(params).await,
            Request::TunnelCheckTarget(params) => self.check_target(&params).await,
            Request::TunnelDelete(params) => self.delete_tunnel(params).await,
            Request::TunnelStart(params) => self.start_tunnel(&params),
            Request::TunnelStop(params) => self.stop_tunnel(&params),
            Request::TunnelRestart(params) => self.restart_tunnel(&params),
            Request::TunnelLogs(params) => self.tunnel_logs(&params),
            Request::HostsList {} => self.list_hosts().await,
            Request::HostsResolve(params) => self.resolve_host(params.host).await,
            Request::ImportSshConfig {} => self.import_ssh_config().await,
            Request::ImportScript(params) => self.import_script(params.text).await,
            Request::TagStart(params) => self.start_tag(&params.tag),
            Request::TagStop(params) => self.stop_tag(&params.tag),
            Request::SettingsUpdate(settings) => self.update_settings(settings).await,
            Request::CredentialRespond(params) => self
                .prompts
                .respond_credential(requester, params)
                .map(|()| ResponsePayload::empty()),
            Request::HostKeyRespond(params) => self
                .prompts
                .respond_host_key(requester, &params)
                .map(|()| ResponsePayload::empty()),
            Request::DaemonQuit(params) => self.quit(params),
            Request::UiShow {} => self.show_ui(),
            Request::ExternalAdopt(params) => self.adopt_external(&params).await,
            Request::ExternalKill(params) => self.kill_external(params).await,
            Request::ExternalIgnore(params) => self.ignore_external(params).await,
            Request::X11Start(params) => self.start_x11(&params),
            Request::X11Stop(params) => self.stop_x11(&params),
            Request::X11Launch(params) => self.launch_x11(&params).await,
            Request::SecretsForget {} => Ok(self.forget_secrets().await),
            Request::UiClaim {} => Ok(self.claim_ui(requester)),
            Request::TagsRename(params) => self.rename_tag(&params).await,
            Request::TagsDelete(params) => self.delete_tag(&params).await,
            Request::DaemonPaths {} => self.daemon_paths(),
        }
    }

    fn snapshot(&self) -> ResponsePayload {
        let tray_status = self.tray_status.borrow().clone();
        ResponsePayload::Snapshot(snapshot_to_dto(&self.engine.snapshot(), &tray_status))
    }

    async fn adopt_external(&self, params: &TunnelIdParams) -> Result<ResponsePayload, IpcError> {
        let ids = self
            .engine
            .adopt_external(parse_tunnel_id(params)?)
            .await
            .map_err(engine_error_to_ipc)?;
        Ok(ResponsePayload::Adopt(AdoptResult {
            ids: ids.iter().map(ToString::to_string).collect(),
        }))
    }

    async fn kill_external(&self, params: ProcessParams) -> Result<ResponsePayload, IpcError> {
        self.engine
            .kill_external(params.pid)
            .await
            .map(|()| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    async fn ignore_external(&self, params: ProcessParams) -> Result<ResponsePayload, IpcError> {
        self.engine
            .ignore_external(params.pid)
            .await
            .map(|()| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    fn start_x11(&self, params: &X11HostParams) -> Result<ResponsePayload, IpcError> {
        self.engine
            .start_x11(&params.host)
            .map(|()| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    fn stop_x11(&self, params: &X11HostParams) -> Result<ResponsePayload, IpcError> {
        self.engine
            .stop_x11(&params.host)
            .map(|()| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    async fn launch_x11(&self, params: &X11LaunchParams) -> Result<ResponsePayload, IpcError> {
        self.engine
            .launch_x11(&params.host, &params.name)
            .await
            .map(|()| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    async fn forget_secrets(&self) -> ResponsePayload {
        let forgotten = self.engine.forget_secrets().await;
        ResponsePayload::ForgetSecrets(ForgetSecretsResult {
            forgotten_count: u64::try_from(forgotten).unwrap_or(u64::MAX),
        })
    }

    fn claim_ui(&self, requester: Option<ConnectionId>) -> ResponsePayload {
        ResponsePayload::UiClaim(UiClaimResult {
            claimed: self.ui.claim(requester),
        })
    }

    async fn rename_tag(&self, params: &TagRenameParams) -> Result<ResponsePayload, IpcError> {
        let renamed_count = self
            .engine
            .rename_tag(&params.from, &params.to)
            .await
            .map_err(engine_error_to_ipc)?;
        Ok(ResponsePayload::RenameTag(RenameTagResult {
            renamed_count: u64::try_from(renamed_count).unwrap_or(u64::MAX),
        }))
    }

    async fn delete_tag(&self, params: &TagParams) -> Result<ResponsePayload, IpcError> {
        let removed_count = self
            .engine
            .delete_tag(&params.tag)
            .await
            .map_err(engine_error_to_ipc)?;
        Ok(ResponsePayload::DeleteTag(DeleteTagResult {
            removed_count: u64::try_from(removed_count).unwrap_or(u64::MAX),
        }))
    }

    fn daemon_paths(&self) -> Result<ResponsePayload, IpcError> {
        let config_file = std::path::absolute(self.engine.config_path())
            .map_err(|error| IpcError::internal(error.to_string()))?;
        Ok(ResponsePayload::DaemonPaths(DaemonPathsResult {
            config_file: config_file.to_string_lossy().into_owned(),
        }))
    }

    fn show_ui(&self) -> Result<ResponsePayload, IpcError> {
        self.ui
            .show()
            .map(|()| ResponsePayload::empty())
            .map_err(|error| IpcError::ui_unavailable(error.to_string()))
    }

    async fn create_tunnel(
        &self,
        spec: ipc::protocol::TunnelSpecDto,
    ) -> Result<ResponsePayload, IpcError> {
        let id = self
            .engine
            .create_tunnel(new_tunnel_from_spec(spec))
            .await
            .map_err(engine_error_to_ipc)?;
        self.tunnel_response(id)
    }

    async fn start_temporary_tunnel(
        &self,
        spec: ipc::protocol::TemporaryTunnelSpecDto,
    ) -> Result<ResponsePayload, IpcError> {
        let id = self
            .engine
            .start_temporary_tunnel(new_temporary_tunnel_from_spec(spec))
            .await
            .map_err(engine_error_to_ipc)?;
        self.tunnel_response(id)
    }

    async fn persist_tunnel(&self, params: TunnelIdParams) -> Result<ResponsePayload, IpcError> {
        self.engine
            .persist_tunnel(parse_tunnel_id(&params)?)
            .await
            .map(|()| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    async fn update_tunnel(
        &self,
        dto: ipc::protocol::TunnelDefinitionDto,
    ) -> Result<ResponsePayload, IpcError> {
        let definition = definition_from_dto(dto)
            .map_err(|error| IpcError::invalid_request(error.to_string()))?;
        let id = definition.id;
        self.engine
            .update_tunnel(definition)
            .await
            .map_err(engine_error_to_ipc)?;
        self.tunnel_response(id)
    }

    async fn delete_tunnel(&self, params: TunnelIdParams) -> Result<ResponsePayload, IpcError> {
        let id = parse_tunnel_id(&params)?;
        self.engine
            .delete_tunnel(id)
            .await
            .map_err(engine_error_to_ipc)?;
        Ok(ResponsePayload::empty())
    }

    fn start_tunnel(&self, params: &TunnelIdParams) -> Result<ResponsePayload, IpcError> {
        self.engine
            .start_tunnel(parse_tunnel_id(params)?)
            .map(|()| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    fn stop_tunnel(&self, params: &TunnelIdParams) -> Result<ResponsePayload, IpcError> {
        self.engine
            .stop_tunnel(parse_tunnel_id(params)?)
            .map(|()| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    fn restart_tunnel(&self, params: &TunnelIdParams) -> Result<ResponsePayload, IpcError> {
        self.engine
            .restart_tunnel(parse_tunnel_id(params)?)
            .map(|()| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    async fn check_target(&self, params: &TunnelIdParams) -> Result<ResponsePayload, IpcError> {
        let health = self
            .engine
            .check_target(parse_tunnel_id(params)?)
            .await
            .map_err(engine_error_to_ipc)?;
        Ok(ResponsePayload::CheckTarget(CheckTargetResult {
            health: health_to_dto(&health),
        }))
    }

    fn tunnel_logs(&self, params: &TunnelIdParams) -> Result<ResponsePayload, IpcError> {
        let entries = self
            .engine
            .logs(parse_tunnel_id(params)?)
            .map_err(engine_error_to_ipc)?;
        Ok(ResponsePayload::Logs(LogsResult {
            entries: entries.iter().map(log_entry_to_dto).collect(),
        }))
    }

    async fn list_hosts(&self) -> Result<ResponsePayload, IpcError> {
        let hosts = Arc::clone(&self.hosts);
        run_blocking(move || hosts.list_hosts())
            .await?
            .map(|hosts| ResponsePayload::Hosts(HostsResult { hosts }))
            .map_err(|error| IpcError::internal(error.to_string()))
    }

    async fn resolve_host(&self, alias: String) -> Result<ResponsePayload, IpcError> {
        let hosts = Arc::clone(&self.hosts);
        run_blocking(move || {
            hosts
                .resolve_with_warnings(&alias)
                .map(|resolution| resolved_host_to_dto(&alias, &resolution))
        })
        .await?
        .map(ResponsePayload::ResolvedHost)
        .map_err(|error| IpcError::internal(error.to_string()))
    }

    async fn import_ssh_config(&self) -> Result<ResponsePayload, IpcError> {
        let hosts = Arc::clone(&self.hosts);
        run_blocking(move || preview_ssh_config(hosts.as_ref()))
            .await?
            .map(|preview| ResponsePayload::ImportPreview(import_preview_to_dto(&preview)))
            .map_err(|error| IpcError::internal(error.to_string()))
    }

    async fn import_script(&self, text: String) -> Result<ResponsePayload, IpcError> {
        let hosts = Arc::clone(&self.hosts);
        let preview = run_blocking(move || preview_script(&text, hosts.as_ref())).await?;
        Ok(ResponsePayload::ImportPreview(import_preview_to_dto(
            &preview,
        )))
    }

    fn start_tag(&self, tag: &str) -> Result<ResponsePayload, IpcError> {
        self.engine
            .start_tag(tag)
            .map(|_| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    fn stop_tag(&self, tag: &str) -> Result<ResponsePayload, IpcError> {
        self.engine
            .stop_tag(tag)
            .map(|_| ResponsePayload::empty())
            .map_err(engine_error_to_ipc)
    }

    async fn update_settings(&self, dto: SettingsDto) -> Result<ResponsePayload, IpcError> {
        self.engine
            .update_settings(settings_from_dto(dto))
            .await
            .map_err(engine_error_to_ipc)?;
        Ok(ResponsePayload::empty())
    }

    fn quit(&self, params: QuitParams) -> Result<ResponsePayload, IpcError> {
        let active_sessions = self.engine.active_tunnel_count() + self.engine.active_x11_count();
        if !params.force && active_sessions > 0 {
            return Err(IpcError::tunnels_running(active_sessions));
        }
        self.quit_requested.cancel();
        Ok(ResponsePayload::empty())
    }

    fn tunnel_response(&self, id: TunnelId) -> Result<ResponsePayload, IpcError> {
        self.engine
            .snapshot()
            .tunnels
            .iter()
            .find(|snapshot| snapshot.definition.id == id)
            .map(|snapshot| ResponsePayload::Tunnel(tunnel_to_dto(snapshot)))
            .ok_or_else(|| {
                IpcError::internal(format!("the tunnel {id} vanished after being saved"))
            })
    }
}

#[async_trait]
impl RequestHandler for DaemonHandler {
    async fn handle(
        &self,
        connection: &ConnectionInfo,
        request: Request,
    ) -> Result<ResponsePayload, IpcError> {
        self.dispatch(Some(connection.id), request).await
    }
}

fn parse_tunnel_id(params: &TunnelIdParams) -> Result<TunnelId, IpcError> {
    params
        .id
        .parse()
        .map_err(|error: engine::InvalidTunnelId| IpcError::invalid_request(error.to_string()))
}

async fn run_blocking<Output, Work>(work: Work) -> Result<Output, IpcError>
where
    Output: Send + 'static,
    Work: FnOnce() -> Output + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| IpcError::internal(error.to_string()))
}

fn engine_error_to_ipc(error: EngineError) -> IpcError {
    match error {
        EngineError::UnknownTunnel(id) => IpcError::not_found(format!("no tunnel has the id {id}")),
        EngineError::UnknownTag(tag) => {
            IpcError::not_found(format!("no tunnel carries the tag \"{tag}\""))
        }
        EngineError::InvalidTag { .. } => IpcError::validation_failed(&[error.to_string()]),
        EngineError::Store(error) => store_error_to_ipc(error),
        EngineError::ShutDown => IpcError::internal("the daemon is shutting down"),
        EngineError::NotExternal(_)
        | EngineError::TunnelIsTemporary(_)
        | EngineError::TunnelIsNotTemporary(_)
        | EngineError::TunnelNotActive(_)
        | EngineError::TunnelHasNoTarget(_) => IpcError::invalid_state(error.to_string()),
        EngineError::UnknownProcess(_) | EngineError::UnknownLauncher { .. } => {
            IpcError::not_found(error.to_string())
        }
        EngineError::Terminate(_) | EngineError::ProcessStillRunning(_) => {
            IpcError::kill_failed(error.to_string())
        }
        EngineError::X11(ref failure) if is_x11_precondition(failure) => {
            IpcError::invalid_state(error.to_string())
        }
        EngineError::PortScan(_) | EngineError::X11(_) | EngineError::TargetCheckFailed(_) => {
            IpcError::internal(error.to_string())
        }
    }
}

fn is_x11_precondition(failure: &X11Error) -> bool {
    matches!(
        failure,
        X11Error::NoLocalDisplay
            | X11Error::InvalidDisplay { .. }
            | X11Error::Refused
            | X11Error::CommandRefused
            | X11Error::EmptyRemoteDisplay
    )
}

fn store_error_to_ipc(error: StoreError) -> IpcError {
    match error {
        StoreError::Validation(failures) => {
            let messages: Vec<String> = failures.iter().map(ToString::to_string).collect();
            IpcError::validation_failed(&messages)
        }
        other => {
            tracing::error!(error = %other, "the store failed");
            IpcError::internal(other.to_string())
        }
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use engine::{
        AppState, ConnectorDependencies, EngineDependencies, FileStateStore, KeyringError,
        SecretKeyring, SshConfigResolver, StateStore, Store, SystemAgentLocator, SystemClock,
        SystemX11Environment,
    };
    use ipc::protocol::{ErrorCode, TunnelDto, TunnelKindDto, TunnelSpecDto};
    use ssh_config::{ConfigSources, SystemEnvironment};
    use tempfile::TempDir;
    use zeroize::Zeroizing;

    use super::*;
    use crate::daemon::prompts::NoResponderSource;
    use crate::daemon::testing::{EmptyPortScanner, TestServer};
    use crate::daemon::tray::TrayStatusCell;
    use crate::daemon::ui::testing::FakeLauncher;

    const WAIT: Duration = Duration::from_secs(10);
    const POLL_INTERVAL: Duration = Duration::from_millis(25);
    const UNREACHABLE_SSH_PORT: u16 = 1;

    #[derive(Default)]
    struct RecordingKeyring {
        deleted: std::sync::Mutex<Vec<PathBuf>>,
    }

    impl SecretKeyring for RecordingKeyring {
        fn load(&self, _key_path: &Path) -> Result<Option<Zeroizing<String>>, KeyringError> {
            Ok(None)
        }

        fn store(&self, _key_path: &Path, _passphrase: &str) -> Result<(), KeyringError> {
            Ok(())
        }

        fn delete(&self, key_path: &Path) -> Result<(), KeyringError> {
            self.deleted.lock().unwrap().push(key_path.to_owned());
            Ok(())
        }
    }

    struct Fixture {
        handler: DaemonHandler,
        keyring: Arc<RecordingKeyring>,
        engine: Engine,
        quit_requested: CancellationToken,
        launcher: Arc<FakeLauncher>,
        tray_status: Arc<TrayStatusCell>,
        store_path: PathBuf,
        _directory: TempDir,
        _server: TestServer,
    }

    fn fixture() -> Fixture {
        fixture_with_launcher(FakeLauncher::succeeding())
    }

    fn fixture_with_launcher(launcher: Arc<FakeLauncher>) -> Fixture {
        fixture_with(launcher, &AppState::default())
    }

    fn fixture_with(launcher: Arc<FakeLauncher>, saved_state: &AppState) -> Fixture {
        let directory = tempfile::tempdir().unwrap();
        let ssh_config = directory.path().join("ssh_config");
        std::fs::write(
            &ssh_config,
            format!(
                "Host dead\n  HostName 127.0.0.1\n  Port {UNREACHABLE_SSH_PORT}\n  LocalForward 8080 localhost:80\nHost *.internal\n  User me\n"
            ),
        )
        .unwrap();
        let store_path = directory.path().join("tunnels.toml");
        let (store, _) = Store::open(store_path.clone()).unwrap();
        let state_store = FileStateStore::new(directory.path().join("state.toml"));
        state_store.save(saved_state).unwrap();
        let keyring = Arc::new(RecordingKeyring::default());
        let server = TestServer::start();
        let prompts = Arc::new(PromptBroker::new(
            server.handle.clone(),
            tokio::sync::broadcast::channel(8).0,
            Arc::new(NoResponderSource),
        ));
        let hosts: Arc<dyn HostConfigResolver> = Arc::new(SshConfigResolver::new(
            ConfigSources::user_only(&ssh_config),
            SystemEnvironment::new().unwrap(),
        ));
        let engine = Engine::start(EngineDependencies {
            connector: ConnectorDependencies {
                resolver: hosts.clone(),
                prompter: prompts.clone(),
                verifier: prompts.clone(),
                agent_locator: Arc::new(SystemAgentLocator),
            },
            clock: Arc::new(SystemClock),
            store,
            port_scanner: Arc::new(EmptyPortScanner),
            x11_environment: Arc::new(SystemX11Environment),
            state_store: Arc::new(state_store),
            keyring: Some(keyring.clone()),
        })
        .unwrap();
        let quit_requested = CancellationToken::new();
        let ui = Arc::new(UiCoordinator::new(server.handle.clone(), launcher.clone()));
        let tray_status = Arc::new(TrayStatusCell::new(TrayStatus::Disabled));
        let handler = DaemonHandler::new(
            engine.clone(),
            prompts,
            hosts,
            quit_requested.clone(),
            ui,
            tray_status.subscribe(),
        );
        Fixture {
            handler,
            keyring,
            engine,
            quit_requested,
            launcher,
            tray_status,
            store_path,
            _directory: directory,
            _server: server,
        }
    }

    fn spec(listen_port: u16) -> TunnelSpecDto {
        TunnelSpecDto {
            host: "dead".to_owned(),
            kind: TunnelKindDto::Local,
            listen_port,
            listen_address: None,
            target_host: None,
            target_port: None,
            tags: vec!["dev".to_owned()],
        }
    }

    async fn created_tunnel(fixture: &Fixture, listen_port: u16) -> TunnelDto {
        let response = fixture
            .handler
            .dispatch(None, Request::TunnelCreate(spec(listen_port)))
            .await
            .unwrap();
        match response {
            ResponsePayload::Tunnel(tunnel) => tunnel,
            other => panic!("expected a tunnel, got {other:?}"),
        }
    }

    async fn free_port() -> u16 {
        test_support::free_port().await
    }

    #[tokio::test]
    async fn should_report_nothing_forgotten_when_no_passphrase_was_remembered() {
        let fixture = fixture();

        let response = fixture
            .handler
            .dispatch(None, Request::SecretsForget {})
            .await
            .unwrap();

        assert_eq!(
            response,
            ResponsePayload::ForgetSecrets(ForgetSecretsResult { forgotten_count: 0 })
        );
    }

    #[tokio::test]
    async fn should_delete_the_recorded_keyring_entries_when_secrets_are_forgotten() {
        let key_path = PathBuf::from("/home/me/.ssh/id_ed25519");
        let saved_state = AppState {
            keyring_entries: [key_path.clone()].into(),
            ..AppState::default()
        };
        let fixture = fixture_with(FakeLauncher::succeeding(), &saved_state);

        let response = fixture
            .handler
            .dispatch(None, Request::SecretsForget {})
            .await
            .unwrap();

        assert_eq!(
            (response, fixture.keyring.deleted.lock().unwrap().clone()),
            (
                ResponsePayload::ForgetSecrets(ForgetSecretsResult { forgotten_count: 1 }),
                vec![key_path]
            )
        );
    }

    #[tokio::test]
    async fn should_list_the_created_tunnel_when_taking_a_snapshot() {
        let fixture = fixture();
        let port = free_port().await;
        created_tunnel(&fixture, port).await;

        let response = fixture
            .handler
            .dispatch(None, Request::StateSnapshot {})
            .await
            .unwrap();

        let ResponsePayload::Snapshot(snapshot) = response else {
            panic!("expected a snapshot");
        };
        assert_eq!(snapshot.tunnels[0].definition.spec.listen_port, port);
    }

    #[tokio::test]
    async fn should_list_only_the_concrete_aliases_when_listing_hosts() {
        let fixture = fixture();

        let response = fixture
            .handler
            .dispatch(None, Request::HostsList {})
            .await
            .unwrap();

        let ResponsePayload::Hosts(result) = response else {
            panic!("expected hosts");
        };
        assert_eq!(result.hosts, vec!["dead"]);
    }

    #[tokio::test]
    async fn should_resolve_the_host_in_ssh_g_spelling_when_resolving_a_host() {
        let fixture = fixture();

        let response = fixture
            .handler
            .dispatch(
                None,
                Request::HostsResolve(ipc::protocol::HostParams {
                    host: "dead".to_owned(),
                }),
            )
            .await
            .unwrap();

        let ResponsePayload::ResolvedHost(resolved) = response else {
            panic!("expected a resolved host");
        };
        let port = resolved.options.iter().find(|option| option.key == "port");
        assert_eq!(port.map(|option| option.value.as_str()), Some("1"));
    }

    #[tokio::test]
    async fn should_list_the_forwards_of_the_aliases_when_importing_the_ssh_config() {
        let fixture = fixture();

        let response = fixture
            .handler
            .dispatch(None, Request::ImportSshConfig {})
            .await
            .unwrap();

        let ResponsePayload::ImportPreview(preview) = response else {
            panic!("expected a preview");
        };
        let candidate = &preview.candidates[0];
        assert_eq!(
            (
                candidate.host.as_deref(),
                candidate.listen_port,
                candidate.target_port
            ),
            (Some("dead"), 8080, Some(80))
        );
    }

    #[tokio::test]
    async fn should_attribute_the_forwards_to_the_alias_when_importing_a_script() {
        let fixture = fixture();

        let response = fixture
            .handler
            .dispatch(
                None,
                Request::ImportScript(ipc::protocol::ImportScriptParams {
                    text: "ssh -N -L 3333:localhost:3333 dead".to_owned(),
                }),
            )
            .await
            .unwrap();

        let ResponsePayload::ImportPreview(preview) = response else {
            panic!("expected a preview");
        };
        assert_eq!(
            (
                preview.candidates[0].host.as_deref(),
                preview.candidates[0].line
            ),
            (Some("dead"), Some(1))
        );
    }

    #[tokio::test]
    async fn should_answer_validation_failed_with_the_messages_when_the_spec_is_invalid() {
        let fixture = fixture();
        let mut invalid = spec(8080);
        invalid.host = String::new();

        let error = fixture
            .handler
            .dispatch(None, Request::TunnelCreate(invalid))
            .await
            .unwrap_err();

        assert_eq!(
            (error.code, error.data.is_some()),
            (ErrorCode::ValidationFailed, true)
        );
    }

    #[tokio::test]
    async fn should_answer_not_found_when_starting_an_unknown_tunnel() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(
                None,
                Request::TunnelStart(TunnelIdParams {
                    id: TunnelId::generate().to_string(),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn should_answer_invalid_request_when_the_tunnel_id_is_malformed() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(
                None,
                Request::TunnelStop(TunnelIdParams {
                    id: "nope".to_owned(),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::InvalidRequest);
    }

    #[tokio::test]
    async fn should_answer_not_found_when_starting_an_unknown_tag() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(
                None,
                Request::TagStart(ipc::protocol::TagParams {
                    tag: "ghost".to_owned(),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::NotFound);
    }

    fn delete_request(tag: &str) -> Request {
        Request::TagsDelete(TagParams {
            tag: tag.to_owned(),
        })
    }

    fn rename_request(from: &str, to: &str) -> Request {
        Request::TagsRename(TagRenameParams {
            from: from.to_owned(),
            to: to.to_owned(),
        })
    }

    #[tokio::test]
    async fn should_answer_the_renamed_count_when_renaming_a_tag() {
        let fixture = fixture();
        created_tunnel(&fixture, free_port().await).await;

        let response = fixture
            .handler
            .dispatch(None, rename_request("dev", "staging"))
            .await
            .unwrap();

        assert_eq!(
            response,
            ResponsePayload::RenameTag(RenameTagResult { renamed_count: 1 })
        );
    }

    #[tokio::test]
    async fn should_answer_the_removed_count_when_deleting_a_tag() {
        let fixture = fixture();
        created_tunnel(&fixture, free_port().await).await;

        let response = fixture
            .handler
            .dispatch(None, delete_request("dev"))
            .await
            .unwrap();

        assert_eq!(
            response,
            ResponsePayload::DeleteTag(DeleteTagResult { removed_count: 1 })
        );
    }

    #[tokio::test]
    async fn should_answer_not_found_when_deleting_an_unknown_tag() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(None, delete_request("missing"))
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn should_answer_not_found_when_renaming_an_unknown_tag() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(None, rename_request("missing", "staging"))
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn should_answer_validation_failed_when_the_new_tag_name_is_invalid() {
        let fixture = fixture();
        created_tunnel(&fixture, free_port().await).await;

        let error = fixture
            .handler
            .dispatch(None, rename_request("dev", "two words"))
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ValidationFailed);
    }

    #[tokio::test]
    async fn should_answer_the_absolute_config_path_when_asked_for_the_paths() {
        let fixture = fixture();

        let response = fixture
            .handler
            .dispatch(None, Request::DaemonPaths {})
            .await
            .unwrap();

        assert_eq!(
            response,
            ResponsePayload::DaemonPaths(DaemonPathsResult {
                config_file: fixture.store_path.to_string_lossy().into_owned()
            })
        );
    }

    #[tokio::test]
    async fn should_grant_the_claim_when_no_other_ui_is_connected() {
        let fixture = fixture();

        let response = fixture
            .handler
            .dispatch(None, Request::UiClaim {})
            .await
            .unwrap();

        assert_eq!(
            response,
            ResponsePayload::UiClaim(UiClaimResult { claimed: true })
        );
    }

    #[tokio::test]
    async fn should_answer_invalid_state_when_adopting_a_tunnel_that_is_not_external() {
        let fixture = fixture();
        let tunnel = created_tunnel(&fixture, free_port().await).await;

        let error = fixture
            .handler
            .dispatch(
                None,
                Request::ExternalAdopt(TunnelIdParams {
                    id: tunnel.definition.id,
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::InvalidState);
    }

    #[tokio::test]
    async fn should_answer_not_found_when_killing_an_unreported_process() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(None, Request::ExternalKill(ProcessParams { pid: 4242 }))
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn should_answer_not_found_when_ignoring_an_unreported_process() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(None, Request::ExternalIgnore(ProcessParams { pid: 4242 }))
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[test]
    fn should_answer_kill_failed_when_the_process_is_still_running() {
        assert_eq!(
            engine_error_to_ipc(EngineError::ProcessStillRunning(1)).code,
            ErrorCode::KillFailed
        );
    }

    #[test]
    fn should_answer_kill_failed_when_the_termination_is_refused() {
        let error = EngineError::Terminate(engine::TerminateError::Refused { pid: 1 });

        assert_eq!(engine_error_to_ipc(error).code, ErrorCode::KillFailed);
    }

    #[test]
    fn should_answer_internal_when_the_port_scan_fails() {
        let error = EngineError::PortScan(engine::ScanError::ListeningSockets {
            message: "denied".to_owned(),
        });

        assert_eq!(engine_error_to_ipc(error).code, ErrorCode::Internal);
    }

    #[test]
    fn should_answer_not_found_when_the_launcher_is_unknown() {
        let error = EngineError::UnknownLauncher {
            host: "nova".to_owned(),
            name: "Editor".to_owned(),
        };

        assert_eq!(engine_error_to_ipc(error).code, ErrorCode::NotFound);
    }

    #[test]
    fn should_answer_invalid_state_when_the_server_refuses_x11_forwarding() {
        let error = EngineError::X11(X11Error::Refused);

        assert_eq!(engine_error_to_ipc(error).code, ErrorCode::InvalidState);
    }

    #[test]
    fn should_answer_invalid_state_when_no_local_display_is_known() {
        let error = EngineError::X11(X11Error::NoLocalDisplay);

        assert_eq!(engine_error_to_ipc(error).code, ErrorCode::InvalidState);
    }

    #[test]
    fn should_answer_internal_when_the_x11_channel_closes() {
        let error = EngineError::X11(X11Error::ChannelClosed);

        assert_eq!(engine_error_to_ipc(error).code, ErrorCode::Internal);
    }

    #[tokio::test]
    async fn should_answer_not_found_when_launching_an_unknown_x11_launcher() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(
                None,
                Request::X11Launch(X11LaunchParams {
                    host: "dead".to_owned(),
                    name: "Missing".to_owned(),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::NotFound);
    }

    async fn start_x11_and_wait_until_running(fixture: &Fixture) {
        let response = fixture
            .handler
            .dispatch(
                None,
                Request::X11Start(X11HostParams {
                    host: "dead".to_owned(),
                }),
            )
            .await;
        assert!(response.is_ok());
        let deadline = tokio::time::Instant::now() + WAIT;
        while fixture.engine.active_x11_count() == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the anchor never started"
            );
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    #[tokio::test]
    async fn should_list_the_anchor_when_x11_is_started() {
        let fixture = fixture();
        start_x11_and_wait_until_running(&fixture).await;

        let response = fixture
            .handler
            .dispatch(None, Request::StateSnapshot {})
            .await
            .unwrap();

        match response {
            ResponsePayload::Snapshot(snapshot) => assert_eq!(snapshot.x11_anchors.len(), 1),
            other => panic!("expected a snapshot, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_stop_the_anchor_when_x11_is_stopped() {
        let fixture = fixture();
        start_x11_and_wait_until_running(&fixture).await;

        let response = fixture
            .handler
            .dispatch(
                None,
                Request::X11Stop(X11HostParams {
                    host: "dead".to_owned(),
                }),
            )
            .await;

        assert!(response.is_ok());
        let deadline = tokio::time::Instant::now() + WAIT;
        while fixture.engine.active_x11_count() > 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the anchor never stopped"
            );
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    #[tokio::test]
    async fn should_refuse_to_quit_with_the_count_when_an_x11_anchor_runs_and_force_is_off() {
        let fixture = fixture();
        start_x11_and_wait_until_running(&fixture).await;

        let error = fixture
            .handler
            .dispatch(None, Request::DaemonQuit(QuitParams { force: false }))
            .await
            .unwrap_err();

        assert_eq!(
            (error.code, error.data),
            (ErrorCode::TunnelsRunning, Some(serde_json::json!(1)))
        );
    }

    #[tokio::test]
    async fn should_launch_the_ui_when_asked_to_show_it_and_none_is_connected() {
        let fixture = fixture();

        fixture
            .handler
            .dispatch(None, Request::UiShow {})
            .await
            .unwrap();

        assert_eq!(fixture.launcher.launch_count(), 1);
    }

    #[tokio::test]
    async fn should_answer_ui_unavailable_when_the_ui_cannot_be_launched() {
        let fixture = fixture_with_launcher(FakeLauncher::failing());

        let error = fixture
            .handler
            .dispatch(None, Request::UiShow {})
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::UiUnavailable);
    }

    #[tokio::test]
    async fn should_report_the_tray_status_when_taking_a_snapshot() {
        let fixture = fixture();
        fixture.tray_status.set(TrayStatus::MissingHost);

        let response = fixture
            .handler
            .dispatch(None, Request::StateSnapshot {})
            .await
            .unwrap();

        let ResponsePayload::Snapshot(snapshot) = response else {
            panic!("expected a snapshot");
        };
        assert_eq!(snapshot.tray, ipc::protocol::TrayStatusDto::MissingHost);
    }

    #[tokio::test]
    async fn should_answer_prompt_not_pending_when_responding_to_an_unknown_prompt() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(
                None,
                Request::HostKeyRespond(ipc::protocol::HostKeyResponseParams {
                    prompt_id: "ghost".to_owned(),
                    decision: ipc::protocol::HostKeyDecisionDto::Reject,
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::PromptNotPending);
    }

    #[tokio::test]
    async fn should_request_the_shutdown_when_quitting_without_active_tunnels() {
        let fixture = fixture();

        let response = fixture
            .handler
            .dispatch(None, Request::DaemonQuit(QuitParams { force: false }))
            .await;

        assert!(response.is_ok() && fixture.quit_requested.is_cancelled());
    }

    async fn start_and_wait_until_running(fixture: &Fixture, tunnel: &TunnelDto) {
        fixture
            .handler
            .dispatch(
                None,
                Request::TunnelStart(TunnelIdParams {
                    id: tunnel.definition.id.clone(),
                }),
            )
            .await
            .unwrap();
        let deadline = tokio::time::Instant::now() + WAIT;
        while fixture.engine.active_tunnel_count() == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the tunnel never started"
            );
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    #[tokio::test]
    async fn should_refuse_to_quit_with_the_count_when_tunnels_run_and_force_is_off() {
        let fixture = fixture();
        let tunnel = created_tunnel(&fixture, free_port().await).await;
        start_and_wait_until_running(&fixture, &tunnel).await;

        let error = fixture
            .handler
            .dispatch(None, Request::DaemonQuit(QuitParams { force: false }))
            .await
            .unwrap_err();

        assert_eq!(
            (
                error.code,
                error.data,
                fixture.quit_requested.is_cancelled()
            ),
            (ErrorCode::TunnelsRunning, Some(serde_json::json!(1)), false)
        );
    }

    #[tokio::test]
    async fn should_request_the_shutdown_when_tunnels_run_and_force_is_on() {
        let fixture = fixture();
        let tunnel = created_tunnel(&fixture, free_port().await).await;
        start_and_wait_until_running(&fixture, &tunnel).await;

        let response = fixture
            .handler
            .dispatch(None, Request::DaemonQuit(QuitParams { force: true }))
            .await;

        assert!(response.is_ok() && fixture.quit_requested.is_cancelled());
    }

    #[tokio::test]
    async fn should_apply_the_settings_when_they_are_updated_through_the_store() {
        let fixture = fixture();
        let mut settings = settings_to_dto_of(&fixture);
        settings.keepalive_max = 9;

        fixture
            .handler
            .dispatch(None, Request::SettingsUpdate(settings))
            .await
            .unwrap();

        let deadline = tokio::time::Instant::now() + WAIT;
        while fixture.engine.snapshot().settings.keepalive_max != 9 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the engine never applied the settings"
            );
            tokio::time::sleep(POLL_INTERVAL).await;
        }
        assert!(
            std::fs::read_to_string(&fixture.store_path)
                .unwrap()
                .contains("keepalive_max = 9")
        );
    }

    fn temporary_spec(listen_port: u16) -> ipc::protocol::TemporaryTunnelSpecDto {
        ipc::protocol::TemporaryTunnelSpecDto {
            host: "dead".to_owned(),
            kind: TunnelKindDto::Local,
            listen_port,
            listen_address: None,
            target_host: None,
            target_port: None,
        }
    }

    async fn started_temporary_tunnel(fixture: &Fixture, listen_port: u16) -> TunnelDto {
        let response = fixture
            .handler
            .dispatch(
                None,
                Request::TunnelStartTemporary(temporary_spec(listen_port)),
            )
            .await
            .unwrap();
        match response {
            ResponsePayload::Tunnel(tunnel) => tunnel,
            other => panic!("expected a tunnel, got {other:?}"),
        }
    }

    fn tunnel_id_params(tunnel: &TunnelDto) -> TunnelIdParams {
        TunnelIdParams {
            id: tunnel.definition.id.clone(),
        }
    }

    #[tokio::test]
    async fn should_answer_a_temporary_tunnel_when_one_is_started() {
        let fixture = fixture();

        let tunnel = started_temporary_tunnel(&fixture, free_port().await).await;

        assert!(tunnel.is_temporary);
    }

    #[tokio::test]
    async fn should_not_save_the_tunnel_when_a_temporary_tunnel_is_started() {
        let fixture = fixture();

        started_temporary_tunnel(&fixture, free_port().await).await;

        assert!(
            !std::fs::read_to_string(&fixture.store_path)
                .unwrap_or_default()
                .contains("[[tunnels]]")
        );
    }

    #[tokio::test]
    async fn should_answer_validation_failed_when_a_temporary_tunnel_is_invalid() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(None, Request::TunnelStartTemporary(temporary_spec(0)))
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ValidationFailed);
    }

    #[tokio::test]
    async fn should_remove_the_tunnel_when_a_temporary_tunnel_is_stopped() {
        let fixture = fixture();
        let tunnel = started_temporary_tunnel(&fixture, free_port().await).await;

        fixture
            .handler
            .dispatch(None, Request::TunnelStop(tunnel_id_params(&tunnel)))
            .await
            .unwrap();

        assert_eq!(fixture.engine.snapshot().tunnels, Vec::new());
    }

    #[tokio::test]
    async fn should_save_the_tunnel_when_a_temporary_tunnel_is_persisted() {
        let fixture = fixture();
        let port = free_port().await;
        let tunnel = started_temporary_tunnel(&fixture, port).await;

        fixture
            .handler
            .dispatch(None, Request::TunnelPersist(tunnel_id_params(&tunnel)))
            .await
            .unwrap();

        assert!(
            std::fs::read_to_string(&fixture.store_path)
                .unwrap()
                .contains(&format!("listen_port = {port}"))
        );
        assert!(!fixture.engine.snapshot().tunnels[0].is_temporary);
    }

    #[tokio::test]
    async fn should_answer_invalid_state_when_a_saved_tunnel_is_persisted() {
        let fixture = fixture();
        let tunnel = created_tunnel(&fixture, free_port().await).await;

        let error = fixture
            .handler
            .dispatch(None, Request::TunnelPersist(tunnel_id_params(&tunnel)))
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::InvalidState);
    }

    #[tokio::test]
    async fn should_answer_invalid_state_when_the_target_of_a_stopped_tunnel_is_checked() {
        let fixture = fixture();
        let tunnel = created_tunnel(&fixture, free_port().await).await;

        let error = fixture
            .handler
            .dispatch(None, Request::TunnelCheckTarget(tunnel_id_params(&tunnel)))
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::InvalidState);
    }

    #[tokio::test]
    async fn should_answer_not_found_when_the_target_of_an_unknown_tunnel_is_checked() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(
                None,
                Request::TunnelCheckTarget(TunnelIdParams {
                    id: TunnelId::generate().to_string(),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn should_answer_not_found_when_an_unknown_tunnel_is_persisted() {
        let fixture = fixture();

        let error = fixture
            .handler
            .dispatch(
                None,
                Request::TunnelPersist(TunnelIdParams {
                    id: TunnelId::generate().to_string(),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn should_flag_temporary_tunnels_when_taking_a_snapshot() {
        let fixture = fixture();
        started_temporary_tunnel(&fixture, free_port().await).await;
        created_tunnel(&fixture, free_port().await).await;

        let response = fixture
            .handler
            .dispatch(None, Request::StateSnapshot {})
            .await
            .unwrap();

        let ResponsePayload::Snapshot(snapshot) = response else {
            panic!("expected a snapshot");
        };
        let flags: Vec<bool> = snapshot
            .tunnels
            .iter()
            .map(|tunnel| tunnel.is_temporary)
            .collect();
        assert_eq!(flags, vec![false, true]);
    }

    fn settings_to_dto_of(fixture: &Fixture) -> SettingsDto {
        crate::daemon::mapping::settings_to_dto(&fixture.engine.snapshot().settings)
    }
}
