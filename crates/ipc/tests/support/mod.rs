#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::traits::tokio::Stream as _;
use interprocess::local_socket::{GenericFilePath, ToFsName};
use ipc::protocol::{
    AdoptResult, ClientKind, ConfigInvalidEvent, DaemonPathsResult, DeleteTagResult, EventMessage,
    ForgetSecretsResult, Hello, HostOptionDto, ImportCandidateDto, ImportPreviewDto, IpcError,
    LogEntryDto, LogLevelDto, LogsResult, RenameTagResult, Request, RequestEnvelope,
    ResolvedHostDto, ResponsePayload, TunnelKindDto, UiClaimResult,
};
use ipc::{
    ConnectionInfo, Endpoint, IpcClient, IpcServer, RequestHandler, ServerHandle, ServerOptions,
};
use serde_json::Value;
use tempfile::TempDir;
use tokio::io::{AsyncWriteExt, BufReader, ReadHalf, WriteHalf, split};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub const WAIT: Duration = Duration::from_secs(5);
pub const QUIET_PERIOD: Duration = Duration::from_millis(300);
pub const SLOW_TUNNEL_ID: &str = "slow";
pub const MISSING_TUNNEL_ID: &str = "missing";
pub const MISSING_LAUNCHER_NAME: &str = "missing";
pub const UNKILLABLE_PID: u32 = 1;
pub const FORGOTTEN_COUNT: u64 = 3;
pub const RENAMED_COUNT: u64 = 2;
pub const REMOVED_COUNT: u64 = 3;
pub const MISSING_TAG: &str = "missing";
pub const CONFIG_FILE: &str = "/config/tunnels.toml";
pub const SLOW_DELAY: Duration = Duration::from_millis(300);
const EVENT_BUS_CAPACITY: usize = 16;

pub struct FakeHandler;

#[async_trait]
impl RequestHandler for FakeHandler {
    async fn handle(
        &self,
        _connection: &ConnectionInfo,
        request: Request,
    ) -> Result<ResponsePayload, IpcError> {
        match request {
            Request::TunnelLogs(params) => {
                if params.id == SLOW_TUNNEL_ID {
                    tokio::time::sleep(SLOW_DELAY).await;
                }
                Ok(ResponsePayload::Logs(LogsResult {
                    entries: vec![LogEntryDto {
                        at: 1,
                        level: LogLevelDto::Info,
                        message: params.id,
                    }],
                }))
            }
            Request::TunnelDelete(params) if params.id == MISSING_TUNNEL_ID => {
                Err(IpcError::not_found("no such tunnel"))
            }
            Request::HostsResolve(params) => Ok(ResponsePayload::ResolvedHost(ResolvedHostDto {
                host: params.host,
                options: vec![HostOptionDto {
                    key: "port".to_owned(),
                    value: "22".to_owned(),
                }],
                warnings: Vec::new(),
            })),
            Request::ImportSshConfig {} => Ok(ResponsePayload::ImportPreview(ImportPreviewDto {
                candidates: Vec::new(),
                warnings: Vec::new(),
            })),
            Request::ImportScript(params) => Ok(ResponsePayload::ImportPreview(ImportPreviewDto {
                candidates: vec![ImportCandidateDto {
                    host: None,
                    kind: TunnelKindDto::Local,
                    listen_port: 3333,
                    listen_address: None,
                    target_host: None,
                    target_port: None,
                    label: Some(params.text),
                    line: Some(1),
                }],
                warnings: Vec::new(),
            })),
            Request::ExternalAdopt(params) => Ok(ResponsePayload::Adopt(AdoptResult {
                ids: vec![params.id],
            })),
            Request::SecretsForget {} => Ok(ResponsePayload::ForgetSecrets(ForgetSecretsResult {
                forgotten_count: FORGOTTEN_COUNT,
            })),
            Request::UiClaim {} => Ok(ResponsePayload::UiClaim(UiClaimResult { claimed: true })),
            Request::TagsRename(params) if params.from == MISSING_TAG => {
                Err(IpcError::not_found("no such tag"))
            }
            Request::TagsRename(_) => Ok(ResponsePayload::RenameTag(RenameTagResult {
                renamed_count: RENAMED_COUNT,
            })),
            Request::TagsDelete(params) if params.tag == MISSING_TAG => {
                Err(IpcError::not_found("no such tag"))
            }
            Request::TagsDelete(_) => Ok(ResponsePayload::DeleteTag(DeleteTagResult {
                removed_count: REMOVED_COUNT,
            })),
            Request::DaemonPaths {} => Ok(ResponsePayload::DaemonPaths(DaemonPathsResult {
                config_file: CONFIG_FILE.to_owned(),
            })),
            Request::ExternalKill(params) if params.pid == UNKILLABLE_PID => {
                Err(IpcError::kill_failed("process still running"))
            }
            Request::X11Launch(params) if params.name == MISSING_LAUNCHER_NAME => {
                Err(IpcError::not_found("no such launcher"))
            }
            Request::DaemonQuit(params) if !params.force => Err(IpcError::tunnels_running(2)),
            _ => Ok(ResponsePayload::empty()),
        }
    }
}

/// A temporary directory closed to group and others, as a socket directory must be on macOS.
pub fn private_tempdir() -> TempDir {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        const OWNER_ONLY: u32 = 0o700;
        tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(OWNER_ONLY))
            .tempdir()
            .unwrap()
    }
    #[cfg(not(unix))]
    {
        tempfile::tempdir().unwrap()
    }
}

pub struct TestDaemon {
    pub directory: TempDir,
    pub endpoint: Endpoint,
    pub handle: ServerHandle,
    pub events: broadcast::Sender<EventMessage>,
    pub shutdown: CancellationToken,
    pub task: JoinHandle<()>,
}

impl TestDaemon {
    pub fn start() -> Self {
        Self::start_with(ServerOptions::default())
    }

    pub fn start_with(options: ServerOptions) -> Self {
        let directory = private_tempdir();
        let endpoint = Endpoint::from_path(directory.path().join("tunneler.sock"));
        let server = IpcServer::bind(&endpoint).unwrap().with_options(options);
        let handle = server.handle();
        let (events, receiver) = broadcast::channel(EVENT_BUS_CAPACITY);
        let shutdown = CancellationToken::new();
        let task = server.serve(Arc::new(FakeHandler), receiver, shutdown.clone());
        Self {
            directory,
            endpoint,
            handle,
            events,
            shutdown,
            task,
        }
    }

    pub async fn connect(&self, kind: ClientKind, subscribe_events: bool) -> IpcClient {
        let hello = Hello::new(kind, "test", subscribe_events, false);
        IpcClient::connect(&self.endpoint, hello).await.unwrap()
    }

    pub async fn connect_raw(&self) -> RawClient {
        RawClient::connect(&self.endpoint).await
    }

    pub async fn connect_raw_after_hello(&self, subscribe_events: bool) -> RawClient {
        let mut client = self.connect_raw().await;
        let hello = Hello::new(ClientKind::Ui, "test", subscribe_events, false);
        client.send_request(1, Request::Hello(hello)).await;
        client.read_value().await.unwrap();
        client
    }
}

pub struct RawClient {
    reader: BufReader<ReadHalf<Stream>>,
    writer: WriteHalf<Stream>,
}

impl RawClient {
    pub async fn connect(endpoint: &Endpoint) -> Self {
        let path = endpoint.socket_path().unwrap().to_owned();
        let name = path.as_os_str().to_fs_name::<GenericFilePath>().unwrap();
        let stream = Stream::connect(name).await.unwrap();
        let (reader, writer) = split(stream);
        Self {
            reader: BufReader::new(reader),
            writer,
        }
    }

    pub async fn send_line(&mut self, line: &str) {
        self.writer.write_all(line.as_bytes()).await.unwrap();
        self.writer.write_all(b"\n").await.unwrap();
        self.writer.flush().await.unwrap();
    }

    pub async fn send_bytes(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes).await;
        let _ = self.writer.flush().await;
    }

    pub async fn send_request(&mut self, id: u64, request: Request) {
        let line = serde_json::to_string(&RequestEnvelope { id, request }).unwrap();
        self.send_line(&line).await;
    }

    pub async fn read_value(&mut self) -> Option<Value> {
        let line = tokio::time::timeout(WAIT, self.read_raw_line())
            .await
            .expect("timed out waiting for a line")?;
        Some(serde_json::from_str(&line).unwrap())
    }

    pub async fn read_value_within(&mut self, period: Duration) -> Option<Option<Value>> {
        let line = tokio::time::timeout(period, self.read_raw_line())
            .await
            .ok()?;
        Some(line.map(|line| serde_json::from_str(&line).unwrap()))
    }

    async fn read_raw_line(&mut self) -> Option<String> {
        use tokio::io::AsyncBufReadExt;
        let mut line = String::new();
        match self.reader.read_line(&mut line).await {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line),
        }
    }
}

pub fn large_event(kilobytes: usize) -> EventMessage {
    EventMessage::ConfigInvalid(ConfigInvalidEvent {
        messages: vec!["x".repeat(kilobytes * 1024)],
    })
}
