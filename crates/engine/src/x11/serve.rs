use std::io;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{ChannelOpenHandle, Msg};
use russh::{Channel, ChannelOpenFailure};
use thiserror::Error;
use tokio::io::{AsyncWriteExt, copy_bidirectional};
use tokio::time::timeout;
use tracing::{debug, warn};

use super::local_connection::connect_display;
use super::router::X11Forwarding;
use super::setup::{SetupError, read_setup_prefix, rewrite_setup_prefix};

const SETUP_TIMEOUT: Duration = Duration::from_secs(10);

/// An X11 channel the server opened, waiting to be served.
pub(crate) struct X11Connection {
    pub(crate) channel: Channel<Msg>,
    pub(crate) reply: ChannelOpenHandle,
    pub(crate) originator: String,
}

#[derive(Debug, Error)]
enum ServeError {
    #[error(transparent)]
    Setup(#[from] SetupError),
    #[error("the X client sent no setup packet within {0:?}")]
    SetupTimeout(Duration),
    #[error("cannot write to the local X server: {0}")]
    WriteSetup(io::Error),
}

/// Serves one X11 channel: connects to the local X server, swaps the fake
/// cookie of the X client's setup packet for the real one, then relays both
/// ways. The channel is refused when the X server does not answer and closed
/// when the client does not present the fake cookie.
pub(crate) async fn serve_x11_connection(
    connection: X11Connection,
    forwarding: Arc<X11Forwarding>,
) {
    let X11Connection {
        channel,
        reply,
        originator,
    } = connection;
    let mut local = match connect_display(&forwarding.local.display).await {
        Ok(local) => local,
        Err(error) => {
            reply.reject(ChannelOpenFailure::ConnectFailed).await;
            warn!(%originator, %error, "X11 connection refused");
            return;
        }
    };
    reply.accept().await;
    let mut remote = channel.into_stream();
    if let Err(error) = forward_setup(&mut remote, &mut local, &forwarding).await {
        warn!(%originator, %error, "X11 connection closed");
        return;
    }
    if let Err(error) = copy_bidirectional(&mut remote, &mut local).await {
        debug!(%originator, %error, "X11 connection ended with an error");
    }
}

async fn forward_setup<Remote, Local>(
    remote: &mut Remote,
    local: &mut Local,
    forwarding: &X11Forwarding,
) -> Result<(), ServeError>
where
    Remote: tokio::io::AsyncRead + Unpin,
    Local: tokio::io::AsyncWrite + Unpin,
{
    let prefix = timeout(SETUP_TIMEOUT, read_setup_prefix(remote))
        .await
        .map_err(|_| ServeError::SetupTimeout(SETUP_TIMEOUT))??;
    let rewritten = rewrite_setup_prefix(
        &prefix,
        forwarding.fake_cookie.as_bytes(),
        forwarding.local.real_cookie.as_deref(),
    )?;
    local
        .write_all(&rewritten)
        .await
        .map_err(ServeError::WriteSetup)
}
