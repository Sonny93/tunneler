use std::fmt;
use std::future::{Future, poll_fn};
use std::iter::once;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use russh::Disconnect;
use russh::client::Handle;
use ssh_config::ResolvedConfig;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::watch;
use tokio::time::{sleep, timeout};

use crate::error::SessionError;
use crate::host_key::ClientHandler;
use crate::remote_forwards::{ForwardedSender, RemoteForwardRegistration, RemoteForwardRegistry};

const DEFAULT_ORIGINATOR_ADDRESS: &str = "127.0.0.1";
const DEFAULT_ORIGINATOR_PORT: u16 = 0;
const DISCONNECT_DESCRIPTION: &str = "closed by Tunneler";
const DISCONNECT_LANGUAGE_TAG: &str = "en";
const CLOSE_GRACE: Duration = Duration::from_secs(2);
const CLOSE_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// An authenticated SSH connection to one host alias.
pub struct SshSession {
    alias: String,
    resolved: ResolvedConfig,
    handle: Handle<ClientHandler>,
    closed: watch::Receiver<bool>,
    remote_forwards: Arc<RemoteForwardRegistry>,
    jumps: Vec<SshSession>,
}

type BoxedWait<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

impl fmt::Debug for SshSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SshSession")
            .field("alias", &self.alias)
            .field("hostname", &self.resolved.hostname)
            .field("port", &self.resolved.port)
            .finish_non_exhaustive()
    }
}

impl SshSession {
    pub(crate) fn new(
        alias: String,
        resolved: ResolvedConfig,
        handle: Handle<ClientHandler>,
        closed: watch::Receiver<bool>,
        remote_forwards: Arc<RemoteForwardRegistry>,
        jumps: Vec<SshSession>,
    ) -> Self {
        Self {
            alias,
            resolved,
            handle,
            closed,
            remote_forwards,
            jumps,
        }
    }

    /// The host alias this session was opened for.
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// The configuration the session was opened with.
    pub fn resolved(&self) -> &ResolvedConfig {
        &self.resolved
    }

    /// Opens a `direct-tcpip` channel to `host:port`, as seen from the server.
    ///
    /// `originator` is the address of the client the traffic comes from, as
    /// reported to the server; without it the server is told `127.0.0.1:0`.
    pub async fn open_direct_tcpip(
        &self,
        host: &str,
        port: u16,
        originator: Option<SocketAddr>,
    ) -> Result<impl AsyncRead + AsyncWrite + Unpin + Send + 'static, SessionError> {
        let (originator_address, originator_port) = originator.map_or_else(
            || {
                (
                    DEFAULT_ORIGINATOR_ADDRESS.to_owned(),
                    DEFAULT_ORIGINATOR_PORT,
                )
            },
            |address| (address.ip().to_string(), address.port()),
        );
        let channel = self
            .handle
            .channel_open_direct_tcpip(
                host,
                u32::from(port),
                originator_address,
                u32::from(originator_port),
            )
            .await
            .map_err(|source| SessionError::OpenChannel {
                host: host.to_owned(),
                port,
                source: Box::new(source),
            })?;
        Ok(channel.into_stream())
    }

    /// Asks the server to listen on `address:port` and sends what it accepts
    /// to `sender`, until the returned registration is dropped.
    ///
    /// The routing is in place before the server is asked, so that no
    /// connection can arrive unrouted.
    pub(crate) async fn request_remote_forward(
        &self,
        address: &str,
        port: u16,
        sender: ForwardedSender,
    ) -> Result<RemoteForwardRegistration, SessionError> {
        let registration = self.remote_forwards.register(address, port, sender);
        match self.handle.tcpip_forward(address, u32::from(port)).await {
            Ok(_) => Ok(registration),
            Err(russh::Error::RequestDenied) => Err(SessionError::ForwardRefused {
                address: address.to_owned(),
                port,
            }),
            Err(source) => Err(SessionError::RequestForward {
                address: address.to_owned(),
                port,
                source: Box::new(source),
            }),
        }
    }

    /// Stops the routing of a remote forward, then asks the server to stop
    /// listening.
    pub(crate) async fn cancel_remote_forward(
        &self,
        registration: RemoteForwardRegistration,
    ) -> Result<(), SessionError> {
        let address = registration.address().to_owned();
        let port = registration.port();
        drop(registration);
        self.handle
            .cancel_tcpip_forward(address.as_str(), u32::from(port))
            .await
            .map_err(|source| SessionError::CancelForward {
                address,
                port,
                source: Box::new(source),
            })
    }

    /// Resolves once this session, or any jump host it runs through, has
    /// ended: disconnected by either side, or lost with the network.
    pub async fn closed(&self) {
        self.closed_wait().await;
    }

    fn closed_wait(&self) -> BoxedWait<'_> {
        Box::pin(async move {
            let mut waits: Vec<BoxedWait<'_>> = vec![Box::pin(self.own_closed())];
            waits.extend(self.jumps.iter().map(SshSession::closed_wait));
            poll_fn(|context| {
                let is_any_ready = waits
                    .iter_mut()
                    .any(|wait| wait.as_mut().poll(context).is_ready());
                if is_any_ready {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
        })
    }

    pub(crate) fn is_closed(&self) -> bool {
        *self.closed.borrow()
            || self.handle.is_closed()
            || self.jumps.iter().any(SshSession::is_closed)
    }

    async fn own_closed(&self) {
        let mut receiver = self.closed.clone();
        let _ = receiver.wait_for(|is_closed| *is_closed).await;
    }

    /// The aliases of the jump hosts this session runs through, from the
    /// first hop to the last. Empty when the host is reached directly.
    pub fn jump_chain(&self) -> Vec<&str> {
        self.jumps
            .iter()
            .flat_map(|jump| jump.jump_chain().into_iter().chain(once(jump.alias())))
            .collect()
    }

    /// Closes this session, then the jump hosts it runs through, last hop first.
    ///
    /// Every session is closed even when one fails; the first failure is
    /// returned.
    pub async fn disconnect(&self) -> Result<(), SessionError> {
        let mut first_failure = None;
        for session in self.closing_order() {
            if let Err(error) = session.disconnect_alone().await {
                first_failure.get_or_insert(error);
            }
        }
        first_failure.map_or(Ok(()), Err)
    }

    fn closing_order(&self) -> Vec<&SshSession> {
        once(self)
            .chain(self.jumps.iter().rev().flat_map(SshSession::closing_order))
            .collect()
    }

    async fn disconnect_alone(&self) -> Result<(), SessionError> {
        self.handle
            .disconnect(
                Disconnect::ByApplication,
                DISCONNECT_DESCRIPTION,
                DISCONNECT_LANGUAGE_TAG,
            )
            .await
            .map_err(SessionError::Disconnect)?;
        let _ = timeout(CLOSE_GRACE, self.wait_until_closed()).await;
        Ok(())
    }

    async fn wait_until_closed(&self) {
        while !self.handle.is_closed() {
            sleep(CLOSE_POLL_INTERVAL).await;
        }
    }
}
