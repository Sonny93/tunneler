//! A real IPC server for the unit tests of the daemon, with real clients.

use std::sync::Arc;

use async_trait::async_trait;
use engine::{PortScanner, ProcessIdentity, ScanError, SystemScan, TerminateError};
use ipc::protocol::{ClientKind, Hello, IpcError, Request, ResponsePayload};
use ipc::{ConnectionInfo, Endpoint, IpcClient, IpcServer, RequestHandler, ServerHandle};
use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const EVENT_BUS_CAPACITY: usize = 16;

struct AcceptingHandler;

#[async_trait]
impl RequestHandler for AcceptingHandler {
    async fn handle(
        &self,
        _connection: &ConnectionInfo,
        _request: Request,
    ) -> Result<ResponsePayload, IpcError> {
        Ok(ResponsePayload::empty())
    }
}

/// A port scanner that sees nothing and ends nothing.
pub(crate) struct EmptyPortScanner;

impl PortScanner for EmptyPortScanner {
    fn scan(&self) -> Result<SystemScan, ScanError> {
        Ok(SystemScan::default())
    }

    fn terminate(&self, process: ProcessIdentity) -> Result<(), TerminateError> {
        Err(TerminateError::NotFound(process.pid))
    }
}

pub(crate) struct TestServer {
    _directory: TempDir,
    pub(crate) endpoint: Endpoint,
    pub(crate) handle: ServerHandle,
    shutdown: CancellationToken,
    _task: JoinHandle<()>,
}

impl TestServer {
    pub(crate) fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let endpoint = Endpoint::from_path(directory.path().join("tunneler.sock"));
        let server = IpcServer::bind(&endpoint).unwrap();
        let handle = server.handle();
        let (_events, receiver) = broadcast::channel(EVENT_BUS_CAPACITY);
        let shutdown = CancellationToken::new();
        let task = server.serve(Arc::new(AcceptingHandler), receiver, shutdown.clone());
        Self {
            _directory: directory,
            endpoint,
            handle,
            shutdown,
            _task: task,
        }
    }

    pub(crate) async fn connect(
        &self,
        kind: ClientKind,
        subscribe_events: bool,
        can_prompt: bool,
    ) -> IpcClient {
        let hello = Hello::new(kind, "test", subscribe_events, can_prompt);
        IpcClient::connect(&self.endpoint, hello).await.unwrap()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}
