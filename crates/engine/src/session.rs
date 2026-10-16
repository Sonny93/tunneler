use std::fmt;

use russh::Disconnect;
use russh::client::Handle;
use ssh_config::ResolvedConfig;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::SessionError;
use crate::host_key::ClientHandler;

const ORIGINATOR_ADDRESS: &str = "127.0.0.1";
const ORIGINATOR_PORT: u32 = 0;
const DISCONNECT_DESCRIPTION: &str = "closed by Tunneler";
const DISCONNECT_LANGUAGE_TAG: &str = "en";

/// An authenticated SSH connection to one host alias.
pub struct SshSession {
    alias: String,
    resolved: ResolvedConfig,
    handle: Handle<ClientHandler>,
}

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
    ) -> Self {
        Self {
            alias,
            resolved,
            handle,
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
    pub async fn open_direct_tcpip(
        &self,
        host: &str,
        port: u16,
    ) -> Result<impl AsyncRead + AsyncWrite + Unpin + Send + 'static, SessionError> {
        let channel = self
            .handle
            .channel_open_direct_tcpip(host, u32::from(port), ORIGINATOR_ADDRESS, ORIGINATOR_PORT)
            .await
            .map_err(|source| SessionError::OpenChannel {
                host: host.to_owned(),
                port,
                source,
            })?;
        Ok(channel.into_stream())
    }

    /// Tells the server the session is over.
    pub async fn disconnect(&self) -> Result<(), SessionError> {
        self.handle
            .disconnect(
                Disconnect::ByApplication,
                DISCONNECT_DESCRIPTION,
                DISCONNECT_LANGUAGE_TAG,
            )
            .await
            .map_err(SessionError::Disconnect)
    }
}
