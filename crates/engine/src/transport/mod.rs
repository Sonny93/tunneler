mod addresses;
mod proxy_command;
mod tcp;

use ssh_config::ResolvedConfig;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::TransportError;

/// A byte stream carrying an SSH connection: a TCP socket now, a jump host
/// channel or a proxy command later.
pub(crate) trait TransportStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<Stream> TransportStream for Stream where Stream: AsyncRead + AsyncWrite + Unpin + Send {}

pub(crate) type BoxedTransport = Box<dyn TransportStream>;

/// Opens the stream to the host itself: through its `ProxyCommand` when it
/// has one, over TCP otherwise. `ProxyJump` is handled by the connector.
pub(crate) async fn open_transport(
    config: &ResolvedConfig,
) -> Result<BoxedTransport, TransportError> {
    if let Some(command) = &config.proxy_command {
        return Ok(Box::new(proxy_command::spawn_proxy_command(command)?));
    }
    Ok(Box::new(tcp::connect_tcp(config).await?))
}
