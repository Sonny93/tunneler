use std::io;
use std::sync::Arc;
use std::time::Duration;

use russh::ChannelOpenFailure;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::debug;

use crate::remote_forwards::ForwardedConnection;
use crate::runtime::counting_stream::CountingStream;
use crate::runtime::local_forward::{ForwardTarget, TargetReport, relay};
use crate::runtime::log_buffer::LogLevel;
use crate::runtime::shared::TunnelShared;

const LOCAL_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Everything a task serving one connection of a remote forward needs.
pub(crate) struct ForwardedJob {
    pub(crate) connection: ForwardedConnection,
    pub(crate) target: ForwardTarget,
    pub(crate) shared: Arc<TunnelShared>,
    pub(crate) reports: mpsc::UnboundedSender<TargetReport>,
}

/// Serves one connection the server opened: connects to the local target and
/// relays, or refuses the connection when the target does not answer.
pub(crate) async fn serve_forwarded_connection(job: ForwardedJob) {
    let ForwardedJob {
        connection,
        target,
        shared,
        reports,
    } = job;
    let _active = shared.counters().begin_connection();
    let ForwardedConnection {
        channel,
        reply,
        originator,
    } = connection;
    let local = match connect_to_target(&target).await {
        Ok(local) => local,
        Err(error) => {
            reply.reject(ChannelOpenFailure::ConnectFailed).await;
            shared.log(
                LogLevel::Warn,
                format!(
                    "connection from {originator} closed: cannot reach {}:{}: {error}",
                    target.host, target.port
                ),
            );
            let _ = reports.send(TargetReport::Unreachable(error.to_string()));
            return;
        }
    };
    reply.accept().await;
    let _ = reports.send(TargetReport::Reachable);
    let counted_target = CountingStream::on_target_side(local, Arc::clone(shared.counters()));
    relay(counted_target, channel.into_stream()).await;
}

/// Connects to the target and closes the connection right away.
pub(crate) async fn probe_local_target(
    target: ForwardTarget,
    reports: mpsc::UnboundedSender<TargetReport>,
) {
    match connect_to_target(&target).await {
        Ok(_connection) => {
            let _ = reports.send(TargetReport::Reachable);
        }
        Err(error) => {
            debug!(%error, "health probe cannot reach the target");
            let _ = reports.send(TargetReport::Unreachable(error.to_string()));
        }
    }
}

async fn connect_to_target(target: &ForwardTarget) -> io::Result<TcpStream> {
    timeout(
        LOCAL_CONNECT_TIMEOUT,
        TcpStream::connect((target.host.as_str(), target.port)),
    )
    .await
    .map_err(|_| {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!("no answer within {LOCAL_CONNECT_TIMEOUT:?}"),
        )
    })?
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;
    use tokio::sync::mpsc::unbounded_channel;

    use super::*;

    async fn free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    }

    #[tokio::test]
    async fn should_report_reachable_when_the_probe_connects() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (reports, mut received) = unbounded_channel();

        probe_local_target(
            ForwardTarget {
                host: "127.0.0.1".to_owned(),
                port,
            },
            reports,
        )
        .await;

        assert_eq!(received.recv().await, Some(TargetReport::Reachable));
    }

    #[tokio::test]
    async fn should_report_unreachable_when_nothing_listens_on_the_probed_port() {
        let port = free_port().await;
        let (reports, mut received) = unbounded_channel();

        probe_local_target(
            ForwardTarget {
                host: "127.0.0.1".to_owned(),
                port,
            },
            reports,
        )
        .await;

        assert!(matches!(
            received.recv().await,
            Some(TargetReport::Unreachable(_))
        ));
    }
}
