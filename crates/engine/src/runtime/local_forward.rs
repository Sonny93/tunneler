use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::debug;

use crate::error::SessionError;
use crate::runtime::counting_stream::CountingStream;
use crate::runtime::failure::is_target_unreachable;
use crate::runtime::log_buffer::LogLevel;
use crate::runtime::shared::TunnelShared;
use crate::runtime::state::FailureReason;
use crate::session::SshSession;

/// What a connection task tells its tunnel about the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetReport {
    /// A channel to the target opened.
    Reachable,
    /// The server could not reach the target.
    Unreachable(String),
}

/// Where the traffic of a forward goes: as seen from the server for a local
/// forward, from this machine for a remote one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForwardTarget {
    pub(crate) host: String,
    pub(crate) port: u16,
}

/// Everything a connection task needs.
pub(crate) struct ConnectionJob {
    pub(crate) client: TcpStream,
    pub(crate) peer: SocketAddr,
    pub(crate) session: Arc<SshSession>,
    pub(crate) target: ForwardTarget,
    pub(crate) shared: Arc<TunnelShared>,
    pub(crate) reports: mpsc::UnboundedSender<TargetReport>,
}

/// Binds the listening socket, before any SSH traffic, so that a taken port
/// fails fast.
pub(crate) async fn bind_listener(address: &str, port: u16) -> Result<TcpListener, FailureReason> {
    TcpListener::bind((address, port))
        .await
        .map_err(|error| describe_bind_failure(address, port, &error))
}

fn describe_bind_failure(address: &str, port: u16, error: &io::Error) -> FailureReason {
    let address = format_socket_address(address, port);
    if error.kind() == io::ErrorKind::AddrInUse {
        return FailureReason::PortInUse { address };
    }
    FailureReason::ListenFailed {
        address,
        message: error.to_string(),
    }
}

pub(crate) fn format_socket_address(address: &str, port: u16) -> String {
    if address.contains(':') {
        return format!("[{address}]:{port}");
    }
    format!("{address}:{port}")
}

/// Forwards one accepted client connection through the session.
pub(crate) async fn forward_connection(job: ConnectionJob) {
    let ConnectionJob {
        client,
        peer,
        session,
        target,
        shared,
        reports,
    } = job;
    let _active = shared.counters().begin_connection();
    let channel = match session
        .open_direct_tcpip(&target.host, target.port, Some(peer))
        .await
    {
        Ok(channel) => channel,
        Err(error) => return report_open_failure(&shared, &reports, peer, &target, &error),
    };
    let _ = reports.send(TargetReport::Reachable);
    let counted_client = CountingStream::on_client_side(client, Arc::clone(shared.counters()));
    relay(counted_client, channel).await;
}

fn report_open_failure(
    shared: &TunnelShared,
    reports: &mpsc::UnboundedSender<TargetReport>,
    peer: SocketAddr,
    target: &ForwardTarget,
    error: &SessionError,
) {
    shared.log(
        LogLevel::Warn,
        format!(
            "connection from {peer} closed: cannot reach {}:{}: {error}",
            target.host, target.port
        ),
    );
    if is_target_unreachable(error) {
        let _ = reports.send(TargetReport::Unreachable(error.to_string()));
    }
}

/// Copies bytes both ways between a local stream and an SSH channel until
/// either side ends.
pub(crate) async fn relay<Local, Channel>(mut local: Local, mut channel: Channel)
where
    Local: AsyncRead + AsyncWrite + Unpin,
    Channel: AsyncRead + AsyncWrite + Unpin,
{
    if let Err(error) = copy_bidirectional(&mut local, &mut channel).await {
        debug!(%error, "forwarded connection ended with an error");
    }
}

/// Opens a channel to the target and closes it right away.
pub(crate) async fn probe_target(
    session: Arc<SshSession>,
    target: ForwardTarget,
    reports: mpsc::UnboundedSender<TargetReport>,
) {
    match session
        .open_direct_tcpip(&target.host, target.port, None)
        .await
    {
        Ok(mut channel) => {
            let _ = channel.shutdown().await;
            let _ = reports.send(TargetReport::Reachable);
        }
        Err(error) if is_target_unreachable(&error) => {
            let _ = reports.send(TargetReport::Unreachable(error.to_string()));
        }
        Err(error) => debug!(%error, "health probe could not open a channel"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_fail_with_port_in_use_when_the_port_is_taken() {
        let taken = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = taken.local_addr().unwrap().port();

        let failure = bind_listener("127.0.0.1", port).await.unwrap_err();

        assert_eq!(
            failure,
            FailureReason::PortInUse {
                address: format!("127.0.0.1:{port}")
            }
        );
    }

    #[tokio::test]
    async fn should_listen_on_the_address_when_the_port_is_free() {
        let listener = bind_listener("127.0.0.1", 0).await.unwrap();

        assert!(listener.local_addr().unwrap().ip().is_loopback());
    }

    #[tokio::test]
    async fn should_fail_with_listen_failed_when_the_address_is_not_local() {
        let failure = bind_listener("203.0.113.1", 0).await.unwrap_err();

        assert!(
            matches!(failure, FailureReason::ListenFailed { .. }),
            "{failure:?}"
        );
    }

    #[test]
    fn should_bracket_the_address_when_it_is_ipv6() {
        assert_eq!(format_socket_address("::1", 80), "[::1]:80");
    }

    #[test]
    fn should_join_host_and_port_when_the_address_is_ipv4() {
        assert_eq!(format_socket_address("127.0.0.1", 80), "127.0.0.1:80");
    }
}
