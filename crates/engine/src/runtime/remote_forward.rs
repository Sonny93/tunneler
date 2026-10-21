use std::io;
use std::sync::Arc;
use std::time::Duration;

use russh::ChannelOpenFailure;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;

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
