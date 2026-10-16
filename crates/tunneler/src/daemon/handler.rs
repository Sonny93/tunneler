use std::sync::Arc;

use async_trait::async_trait;
use engine::{Engine, EngineError, HostConfigResolver, StoreError, TunnelId};
use ipc::protocol::{
    AdoptResult, HostsResult, IpcError, LogsResult, ProcessParams, QuitParams, Request,
    ResponsePayload, SettingsDto, TunnelIdParams,
};
use ipc::{ConnectionId, ConnectionInfo, RequestHandler};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use super::mapping::{
    definition_from_dto, log_entry_to_dto, new_tunnel_from_spec, settings_from_dto,
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
            Request::TunnelDelete(params) => self.delete_tunnel(params).await,
            Request::TunnelStart(params) => self.start_tunnel(&params),
            Request::TunnelStop(params) => self.stop_tunnel(&params),
            Request::TunnelRestart(params) => self.restart_tunnel(&params),
            Request::TunnelLogs(params) => self.tunnel_logs(&params),
            Request::HostsList {} => self.list_hosts().await,
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
        let listing = tokio::task::spawn_blocking(move || hosts.list_hosts())
            .await
            .map_err(|error| IpcError::internal(error.to_string()))?;
        listing
            .map(|hosts| ResponsePayload::Hosts(HostsResult { hosts }))
            .map_err(|error| IpcError::internal(error.to_string()))
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
        let active_tunnels = self.engine.active_tunnel_count();
        if !params.force && active_tunnels > 0 {
            return Err(IpcError::tunnels_running(active_tunnels));
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

fn engine_error_to_ipc(error: EngineError) -> IpcError {
    match error {
        EngineError::UnknownTunnel(id) => IpcError::not_found(format!("no tunnel has the id {id}")),
        EngineError::UnknownTag(tag) => {
            IpcError::not_found(format!("no tunnel carries the tag \"{tag}\""))
        }
        EngineError::Store(error) => store_error_to_ipc(error),
        EngineError::ShutDown => IpcError::internal("the daemon is shutting down"),
        EngineError::NotExternal(_) => IpcError::invalid_state(error.to_string()),
        EngineError::UnknownProcess(_) => IpcError::not_found(error.to_string()),
        EngineError::Terminate(_) | EngineError::ProcessStillRunning(_) => {
            IpcError::kill_failed(error.to_string())
        }
        EngineError::PortScan(_) => IpcError::internal(error.to_string()),
    }
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
    use std::path::PathBuf;
    use std::time::Duration;

    use engine::{
        ConnectorDependencies, EngineDependencies, SshConfigResolver, Store, SystemAgentLocator,
        SystemClock,
    };
    use ipc::protocol::{ErrorCode, TunnelDto, TunnelKindDto, TunnelSpecDto};
    use ssh_config::{ConfigSources, SystemEnvironment};
    use tempfile::TempDir;

    use super::*;
    use crate::daemon::prompts::NoResponderSource;
    use crate::daemon::testing::{EmptyPortScanner, TestServer};
    use crate::daemon::tray::TrayStatusCell;
    use crate::daemon::ui::testing::FakeLauncher;

    const WAIT: Duration = Duration::from_secs(10);
    const POLL_INTERVAL: Duration = Duration::from_millis(25);
    const UNREACHABLE_SSH_PORT: u16 = 1;

    struct Fixture {
        handler: DaemonHandler,
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
        let directory = tempfile::tempdir().unwrap();
        let ssh_config = directory.path().join("ssh_config");
        std::fs::write(
            &ssh_config,
            format!(
                "Host dead\n  HostName 127.0.0.1\n  Port {UNREACHABLE_SSH_PORT}\nHost *.internal\n  User me\n"
            ),
        )
        .unwrap();
        let store_path = directory.path().join("tunnels.toml");
        let (store, _) = Store::open(store_path.clone()).unwrap();
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
            engine,
            quit_requested,
            launcher,
            tray_status,
            store_path,
            _directory: directory,
            _server: server,
        }
    }

    fn spec(name: &str, listen_port: u16) -> TunnelSpecDto {
        TunnelSpecDto {
            name: name.to_owned(),
            host: "dead".to_owned(),
            kind: TunnelKindDto::Local,
            listen_port,
            listen_address: None,
            target_host: None,
            target_port: None,
            tags: vec!["dev".to_owned()],
        }
    }

    async fn created_tunnel(fixture: &Fixture, name: &str, listen_port: u16) -> TunnelDto {
        let response = fixture
            .handler
            .dispatch(None, Request::TunnelCreate(spec(name, listen_port)))
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
    async fn should_list_the_created_tunnel_when_taking_a_snapshot() {
        let fixture = fixture();
        created_tunnel(&fixture, "Web", free_port().await).await;

        let response = fixture
            .handler
            .dispatch(None, Request::StateSnapshot {})
            .await
            .unwrap();

        let ResponsePayload::Snapshot(snapshot) = response else {
            panic!("expected a snapshot");
        };
        assert_eq!(snapshot.tunnels[0].definition.spec.name, "Web");
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
    async fn should_answer_validation_failed_with_the_messages_when_the_spec_is_invalid() {
        let fixture = fixture();
        let mut invalid = spec("Web", 8080);
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

    #[tokio::test]
    async fn should_answer_invalid_state_when_adopting_a_tunnel_that_is_not_external() {
        let fixture = fixture();
        let tunnel = created_tunnel(&fixture, "Web", free_port().await).await;

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
        let tunnel = created_tunnel(&fixture, "Web", free_port().await).await;
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
        let tunnel = created_tunnel(&fixture, "Web", free_port().await).await;
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

    fn settings_to_dto_of(fixture: &Fixture) -> SettingsDto {
        crate::daemon::mapping::settings_to_dto(&fixture.engine.snapshot().settings)
    }
}
