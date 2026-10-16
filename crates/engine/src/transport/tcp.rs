use std::net::SocketAddr;
use std::time::Duration;

use ssh_config::ResolvedConfig;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use tracing::debug;

use super::addresses::resolve_addresses;
use crate::error::TransportError;

const PAUSE_BETWEEN_ATTEMPT_ROUNDS: Duration = Duration::from_secs(1);

pub(crate) async fn connect_tcp(config: &ResolvedConfig) -> Result<TcpStream, TransportError> {
    let addresses = resolve_addresses(&config.hostname, config.port, config.address_family).await?;
    let destination = Destination {
        host: &config.hostname,
        port: config.port,
    };
    let total_rounds = config.connection_attempts.max(1);
    let mut round = 1;
    loop {
        let outcome = connect_first_reachable(&destination, &addresses, config.connect_timeout);
        match outcome.await {
            Ok(stream) => return Ok(stream),
            Err(error) if round >= total_rounds => return Err(error),
            Err(error) => {
                debug!(%error, round, total_rounds, "connection attempt round failed");
                sleep(PAUSE_BETWEEN_ATTEMPT_ROUNDS).await;
            }
        }
        round += 1;
    }
}

struct Destination<'a> {
    host: &'a str,
    port: u16,
}

async fn connect_first_reachable(
    destination: &Destination<'_>,
    addresses: &[SocketAddr],
    connect_timeout: Option<Duration>,
) -> Result<TcpStream, TransportError> {
    let mut last_error = TransportError::NoAddress {
        host: destination.host.to_owned(),
        port: destination.port,
    };
    for address in addresses {
        match connect_address(*address, connect_timeout).await {
            Ok(stream) => return Ok(stream),
            Err(error) => {
                debug!(%error, "cannot connect to an address");
                last_error = error;
            }
        }
    }
    Err(last_error)
}

async fn connect_address(
    address: SocketAddr,
    connect_timeout: Option<Duration>,
) -> Result<TcpStream, TransportError> {
    let connecting = TcpStream::connect(address);
    let outcome = match connect_timeout {
        Some(limit) => timeout(limit, connecting)
            .await
            .map_err(|_| TransportError::Timeout {
                address,
                timeout: limit,
            })?,
        None => connecting.await,
    };
    outcome.map_err(|source| TransportError::Connect { address, source })
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;

    use super::*;

    const DESTINATION: Destination<'static> = Destination {
        host: "127.0.0.1",
        port: 22,
    };

    async fn closed_port_address() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap()
    }

    #[tokio::test]
    async fn should_connect_when_an_address_is_listening() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        assert!(
            connect_first_reachable(&DESTINATION, &[address], None)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn should_fall_through_to_the_next_address_when_the_first_refuses() {
        let closed = closed_port_address().await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let open = listener.local_addr().unwrap();
        assert!(
            connect_first_reachable(&DESTINATION, &[closed, open], None)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn should_report_the_address_when_every_address_refuses() {
        let closed = closed_port_address().await;
        let outcome = connect_first_reachable(&DESTINATION, &[closed], None).await;
        assert!(matches!(
            outcome,
            Err(TransportError::Connect { address, .. }) if address == closed
        ));
    }
}
