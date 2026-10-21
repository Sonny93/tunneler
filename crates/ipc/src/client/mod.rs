//! The client side, shared by the CLI and the UI bridge.

mod connection;
mod pending;

use std::io;
use std::sync::Arc;

use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::traits::tokio::Stream as _;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncWriteExt, BufReader, WriteHalf, split};
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;

use crate::endpoint::Endpoint;
use crate::error::ClientError;
use crate::protocol::{
    AdoptResult, CheckTargetResult, CredentialAnswer, CredentialResponseParams, DaemonPathsResult,
    DeleteTagResult, EventMessage, ForgetSecretsResult, Hello, HostKeyDecisionDto,
    HostKeyResponseParams, HostParams, HostsResult, ImportPreviewDto, ImportScriptParams,
    LogEntryDto, LogsResult, OpenTunnelResult, ProcessParams, QuitParams, RememberDto,
    RenameTagResult, Request, RequestEnvelope, ResolvedHostDto, SettingsDto, SnapshotDto,
    TagParams, TagRenameParams, TargetHealthDto, TemporaryTunnelSpecDto, TunnelDefinitionDto,
    TunnelDto, TunnelIdParams, TunnelSpecDto, UiClaimResult, X11HostParams, X11LaunchParams,
    encode_line,
};
use connection::{read_handshake_answer, run_reader};
use pending::PendingCalls;

/// The events a daemon pushed to this client, in order.
///
/// The queue is unbounded: a consumer that never reads makes it grow.
#[derive(Debug)]
pub struct EventReceiver {
    receiver: mpsc::UnboundedReceiver<EventMessage>,
}

impl EventReceiver {
    /// The next event, or `None` once the connection is closed and the queue
    /// drained.
    pub async fn recv(&mut self) -> Option<EventMessage> {
        self.receiver.recv().await
    }
}

/// A connection to the daemon, past its handshake.
///
/// Calls take `&self` and may overlap: each response is matched to its request
/// by identifier. Dropping the client closes the connection.
pub struct IpcClient {
    writer: Mutex<WriteHalf<Stream>>,
    pending: Arc<PendingCalls>,
    reader: JoinHandle<()>,
    events: Option<EventReceiver>,
    daemon_version: String,
}

impl Drop for IpcClient {
    fn drop(&mut self) {
        self.reader.abort();
    }
}

impl IpcClient {
    /// Connects to the daemon and performs the handshake.
    ///
    /// # Errors
    ///
    /// [`ClientError::DaemonNotRunning`] when nothing listens,
    /// [`ClientError::ProtocolMismatch`] when the versions differ, and the
    /// other variants when the connection or the handshake fails.
    pub async fn connect(endpoint: &Endpoint, hello: Hello) -> Result<Self, ClientError> {
        let name = endpoint.local_socket_name()?;
        let stream = Stream::connect(name)
            .await
            .map_err(describe_connect_error)?;
        let (read_half, mut write_half) = split(stream);
        let mut reader = BufReader::new(read_half);
        let client_version = hello.protocol_version;
        let handshake_id = 0;
        let line = encode_line(&RequestEnvelope {
            id: handshake_id,
            request: Request::Hello(hello),
        })
        .map_err(ClientError::from)?;
        write_half.write_all(line.as_bytes()).await?;
        write_half.flush().await?;
        let result = read_handshake_answer(&mut reader, client_version).await?;

        let pending = Arc::new(PendingCalls::new());
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let reader = tokio::spawn(run_reader(reader, Arc::clone(&pending), event_sender));
        Ok(Self {
            writer: Mutex::new(write_half),
            pending,
            reader,
            events: Some(EventReceiver {
                receiver: event_receiver,
            }),
            daemon_version: result.daemon_version,
        })
    }

    /// The version of the daemon program.
    pub fn daemon_version(&self) -> &str {
        &self.daemon_version
    }

    /// Takes the stream of events; `None` when it was already taken.
    ///
    /// Events only flow when the handshake asked for them.
    pub fn take_events(&mut self) -> Option<EventReceiver> {
        self.events.take()
    }

    /// Reads every tunnel with its live state, and the settings.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn snapshot(&self) -> Result<SnapshotDto, ClientError> {
        self.call(Request::StateSnapshot {}).await
    }

    /// Adds a tunnel.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn create_tunnel(&self, spec: TunnelSpecDto) -> Result<TunnelDto, ClientError> {
        self.call(Request::TunnelCreate(spec)).await
    }

    /// Starts a temporary tunnel: it is never saved and stopping it removes
    /// it.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn start_temporary_tunnel(
        &self,
        spec: TemporaryTunnelSpecDto,
    ) -> Result<TunnelDto, ClientError> {
        self.call(Request::TunnelStartTemporary(spec)).await
    }

    /// Saves a temporary tunnel; it keeps running.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn persist_tunnel(&self, id: &str) -> Result<(), ClientError> {
        self.call_without_result(Request::TunnelPersist(tunnel_id(id)))
            .await
    }

    /// Checks once whether the target of a running tunnel answers.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn check_target(&self, id: &str) -> Result<TargetHealthDto, ClientError> {
        let result: CheckTargetResult =
            self.call(Request::TunnelCheckTarget(tunnel_id(id))).await?;
        Ok(result.health)
    }

    /// Opens a tunnel in the browser, starting it first when it is stopped,
    /// and returns what was opened.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn open_tunnel(&self, id: &str) -> Result<String, ClientError> {
        let result: OpenTunnelResult = self.call(Request::TunnelOpen(tunnel_id(id))).await?;
        Ok(result.target)
    }

    /// Replaces the definition of a tunnel.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn update_tunnel(
        &self,
        definition: TunnelDefinitionDto,
    ) -> Result<TunnelDto, ClientError> {
        self.call(Request::TunnelUpdate(definition)).await
    }

    /// Removes a tunnel.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn delete_tunnel(&self, id: &str) -> Result<(), ClientError> {
        self.call_without_result(Request::TunnelDelete(tunnel_id(id)))
            .await
    }

    /// Starts a tunnel.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn start_tunnel(&self, id: &str) -> Result<(), ClientError> {
        self.call_without_result(Request::TunnelStart(tunnel_id(id)))
            .await
    }

    /// Stops a tunnel.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn stop_tunnel(&self, id: &str) -> Result<(), ClientError> {
        self.call_without_result(Request::TunnelStop(tunnel_id(id)))
            .await
    }

    /// Stops then starts a tunnel.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn restart_tunnel(&self, id: &str) -> Result<(), ClientError> {
        self.call_without_result(Request::TunnelRestart(tunnel_id(id)))
            .await
    }

    /// Reads the recent log of a tunnel, oldest first.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn tunnel_logs(&self, id: &str) -> Result<Vec<LogEntryDto>, ClientError> {
        let logs: LogsResult = self.call(Request::TunnelLogs(tunnel_id(id))).await?;
        Ok(logs.entries)
    }

    /// Lists the concrete `Host` aliases of the daemon user's `ssh_config`.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`]. A daemon older than
    /// this method answers `unknown_method`.
    pub async fn list_hosts(&self) -> Result<Vec<String>, ClientError> {
        let hosts: HostsResult = self.call(Request::HostsList {}).await?;
        Ok(hosts.hosts)
    }

    /// Reads the effective configuration of a host alias, as `ssh -G` would.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`]. A daemon older than
    /// this method answers `unknown_method`.
    pub async fn resolve_host(&self, host: &str) -> Result<ResolvedHostDto, ClientError> {
        self.call(Request::HostsResolve(HostParams {
            host: host.to_owned(),
        }))
        .await
    }

    /// Lists the forwards declared in the daemon user's `ssh_config`.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`]. A daemon older than
    /// this method answers `unknown_method`.
    pub async fn import_ssh_config(&self) -> Result<ImportPreviewDto, ClientError> {
        self.call(Request::ImportSshConfig {}).await
    }

    /// Reads the forwards of a pasted script.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`]. The daemon answers
    /// `invalid_request` when the text exceeds
    /// [`MAX_IMPORT_SCRIPT_BYTES`](crate::protocol::MAX_IMPORT_SCRIPT_BYTES).
    pub async fn import_script(&self, text: &str) -> Result<ImportPreviewDto, ClientError> {
        self.call(Request::ImportScript(ImportScriptParams {
            text: text.to_owned(),
        }))
        .await
    }

    /// Starts every tunnel carrying a tag.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn start_tag(&self, tag: &str) -> Result<(), ClientError> {
        self.call_without_result(Request::TagStart(tag_params(tag)))
            .await
    }

    /// Stops every tunnel carrying a tag.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn stop_tag(&self, tag: &str) -> Result<(), ClientError> {
        self.call_without_result(Request::TagStop(tag_params(tag)))
            .await
    }

    /// Replaces the settings.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn update_settings(&self, settings: SettingsDto) -> Result<(), ClientError> {
        self.call_without_result(Request::SettingsUpdate(settings))
            .await
    }

    /// Answers a credential prompt; `None` means the user cancelled. `remember`
    /// asks the daemon to keep a passphrase that unlocks its key; it is ignored
    /// for the other prompts.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn respond_credential(
        &self,
        prompt_id: &str,
        answer: Option<CredentialAnswer>,
        remember: Option<RememberDto>,
    ) -> Result<(), ClientError> {
        let params = CredentialResponseParams {
            prompt_id: prompt_id.to_owned(),
            answer,
            remember,
        };
        self.call_without_result(Request::CredentialRespond(params))
            .await
    }

    /// Answers a host key prompt.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn respond_host_key(
        &self,
        prompt_id: &str,
        decision: HostKeyDecisionDto,
    ) -> Result<(), ClientError> {
        let params = HostKeyResponseParams {
            prompt_id: prompt_id.to_owned(),
            decision,
        };
        self.call_without_result(Request::HostKeyRespond(params))
            .await
    }

    /// Asks the daemon to exit; refused with
    /// [`ErrorCode::TunnelsRunning`](crate::protocol::ErrorCode::TunnelsRunning) while tunnels run unless `force`.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn quit(&self, force: bool) -> Result<(), ClientError> {
        self.call_without_result(Request::DaemonQuit(QuitParams { force }))
            .await
    }

    /// Asks the daemon to show the UI.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn show_ui(&self) -> Result<(), ClientError> {
        self.call_without_result(Request::UiShow {}).await
    }

    /// Terminates the external ssh process providing a tunnel and starts every
    /// configured tunnel it provided; returns the identifiers of those tunnels.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn adopt_external(&self, id: &str) -> Result<Vec<String>, ClientError> {
        let adopted: AdoptResult = self.call(Request::ExternalAdopt(tunnel_id(id))).await?;
        Ok(adopted.ids)
    }

    /// Terminates a process the daemon reported.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn kill_external(&self, pid: u32) -> Result<(), ClientError> {
        self.call_without_result(Request::ExternalKill(ProcessParams { pid }))
            .await
    }

    /// Stops the daemon from reporting a process.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn ignore_external(&self, pid: u32) -> Result<(), ClientError> {
        self.call_without_result(Request::ExternalIgnore(ProcessParams { pid }))
            .await
    }

    /// Opens the X11 anchor of a host; does nothing when it is already open.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn start_x11(&self, host: &str) -> Result<(), ClientError> {
        self.call_without_result(Request::X11Start(x11_host(host)))
            .await
    }

    /// Closes the X11 anchor of a host.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn stop_x11(&self, host: &str) -> Result<(), ClientError> {
        self.call_without_result(Request::X11Stop(x11_host(host)))
            .await
    }

    /// Runs a saved launcher in an X11-enabled session on a host.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`]. An unknown launcher
    /// answers [`ErrorCode::NotFound`](crate::protocol::ErrorCode::NotFound).
    pub async fn launch_x11(&self, host: &str, name: &str) -> Result<(), ClientError> {
        let params = X11LaunchParams {
            host: host.to_owned(),
            name: name.to_owned(),
        };
        self.call_without_result(Request::X11Launch(params)).await
    }

    /// Forgets every remembered passphrase, in the daemon's memory and in the
    /// OS keyring.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`].
    pub async fn forget_secrets(&self) -> Result<ForgetSecretsResult, ClientError> {
        self.call(Request::SecretsForget {}).await
    }

    /// Asks whether this UI may stay open: `false` means an older UI is
    /// connected, was asked to show itself, and this one should exit.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`]. A daemon older than
    /// this method answers `unknown_method`.
    pub async fn claim_ui(&self) -> Result<bool, ClientError> {
        let claim: UiClaimResult = self.call(Request::UiClaim {}).await?;
        Ok(claim.claimed)
    }

    /// Renames a tag in every tunnel and in the launch tags; returns how many
    /// tunnels carried it.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`]. The daemon answers
    /// [`ErrorCode::NotFound`](crate::protocol::ErrorCode::NotFound) when no
    /// tunnel carries `from` and
    /// [`ErrorCode::ValidationFailed`](crate::protocol::ErrorCode::ValidationFailed)
    /// when `to` is not a valid tag.
    pub async fn rename_tag(&self, from: &str, to: &str) -> Result<u64, ClientError> {
        let params = TagRenameParams {
            from: from.to_owned(),
            to: to.to_owned(),
        };
        let renamed: RenameTagResult = self.call(Request::TagsRename(params)).await?;
        Ok(renamed.renamed_count)
    }

    /// Removes a tag from every tunnel and from the launch tags; returns how
    /// many tunnels carried it.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`]. The daemon answers
    /// [`ErrorCode::NotFound`](crate::protocol::ErrorCode::NotFound) when no
    /// tunnel carries the tag.
    pub async fn delete_tag(&self, tag: &str) -> Result<u64, ClientError> {
        let params = TagParams {
            tag: tag.to_owned(),
        };
        let deleted: DeleteTagResult = self.call(Request::TagsDelete(params)).await?;
        Ok(deleted.removed_count)
    }

    /// Reads where the daemon keeps its files.
    ///
    /// # Errors
    ///
    /// Fails when the call fails; see [`ClientError`]. A daemon older than
    /// this method answers `unknown_method`.
    pub async fn daemon_paths(&self) -> Result<DaemonPathsResult, ClientError> {
        self.call(Request::DaemonPaths {}).await
    }

    async fn call_without_result(&self, request: Request) -> Result<(), ClientError> {
        self.call::<serde_json::Value>(request).await.map(drop)
    }

    async fn call<T: DeserializeOwned>(&self, request: Request) -> Result<T, ClientError> {
        let (id, answer) = self.pending.register()?;
        let line = encode_line(&RequestEnvelope { id, request })?;
        if let Err(error) = self.write(&line).await {
            self.pending.forget(id);
            return Err(describe_write_error(error));
        }
        let result = answer.await.map_err(|_| ClientError::ConnectionClosed)??;
        Ok(serde_json::from_value(result)?)
    }

    async fn write(&self, line: &str) -> io::Result<()> {
        let mut writer = self.writer.lock().await;
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await
    }
}

fn tunnel_id(id: &str) -> TunnelIdParams {
    TunnelIdParams { id: id.to_owned() }
}

fn tag_params(tag: &str) -> TagParams {
    TagParams {
        tag: tag.to_owned(),
    }
}

fn x11_host(host: &str) -> X11HostParams {
    X11HostParams {
        host: host.to_owned(),
    }
}

fn describe_write_error(error: io::Error) -> ClientError {
    match error.kind() {
        io::ErrorKind::BrokenPipe
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::ConnectionAborted => ClientError::ConnectionClosed,
        _ => ClientError::Io(error),
    }
}

fn describe_connect_error(error: io::Error) -> ClientError {
    match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => ClientError::DaemonNotRunning,
        _ => ClientError::Io(error),
    }
}
