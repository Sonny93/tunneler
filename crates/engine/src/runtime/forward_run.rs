use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use crate::runtime::local_forward::TargetReport;
use crate::runtime::metrics::MetricsThrottle;

/// The resources of one run of a local forward: the listener, which stays
/// bound for the whole run, and the tasks working for it. Dropping it closes
/// the listener and every connection.
pub(crate) struct ForwardRun {
    pub(crate) listener: TcpListener,
    pub(crate) connections: JoinSet<()>,
    pub(crate) probes: JoinSet<()>,
    pub(crate) reports_sender: mpsc::UnboundedSender<TargetReport>,
    pub(crate) reports: mpsc::UnboundedReceiver<TargetReport>,
    pub(crate) throttle: MetricsThrottle,
}

impl ForwardRun {
    pub(crate) fn new(listener: TcpListener, metrics_event_interval: Duration) -> Self {
        let (reports_sender, reports) = mpsc::unbounded_channel();
        Self {
            listener,
            connections: JoinSet::new(),
            probes: JoinSet::new(),
            reports_sender,
            reports,
            throttle: MetricsThrottle::new(metrics_event_interval),
        }
    }

    pub(crate) async fn close_connections(&mut self) {
        self.connections.abort_all();
        self.probes.abort_all();
        while self.connections.join_next().await.is_some() {}
        while self.probes.join_next().await.is_some() {}
    }
}
