mod addresses;
mod tcp;

use ssh_config::ResolvedConfig;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::TransportError;

/// A byte stream carrying an SSH connection: a TCP socket now, a jump host
/// channel or a proxy command later.
pub(crate) trait TransportStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<Stream> TransportStream for Stream where Stream: AsyncRead + AsyncWrite + Unpin + Send {}

pub(crate) type BoxedTransport = Box<dyn TransportStream>;

pub(crate) async fn open_transport(
    config: &ResolvedConfig,
) -> Result<BoxedTransport, TransportError> {
    refuse_unsupported_proxy(config)?;
    let stream = tcp::connect_tcp(config).await?;
    Ok(Box::new(stream))
}

fn refuse_unsupported_proxy(config: &ResolvedConfig) -> Result<(), TransportError> {
    if !config.proxy_jump.is_empty() {
        return Err(TransportError::UnsupportedProxy {
            directive: "ProxyJump",
        });
    }
    if config.proxy_command.is_some() {
        return Err(TransportError::UnsupportedProxy {
            directive: "ProxyCommand",
        });
    }
    Ok(())
}
