use std::future::pending;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use crate::remote_forwards::{ForwardedConnection, ForwardedSender};
use crate::runtime::local_forward::TargetReport;
use crate::runtime::metrics::MetricsThrottle;
use crate::store::TunnelKind;

/// What a client that connects to the listener of a tunnel speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientProtocol {
    /// Raw TCP: everything goes to the fixed target.
    Forward,
    /// SOCKS5: the client names the target.
    Socks,
}

/// Something that reached the tunnel.
pub(crate) enum Inbound {
    /// A local client connected, or accepting it failed.
    Client {
        accepted: io::Result<(TcpStream, SocketAddr)>,
        protocol: ClientProtocol,
    },
    /// The server opened a connection for a remote forward.
    Forwarded(ForwardedConnection),
}

/// Where the things that reach a tunnel come from.
pub(crate) enum InboundSource {
    Listener {
        listener: TcpListener,
        protocol: ClientProtocol,
    },
    Forwarded {
        sender: ForwardedSender,
        receiver: mpsc::UnboundedReceiver<ForwardedConnection>,
    },
}

/// The resources of one run of a forward: the listener of a local or dynamic
/// forward, which stays bound for the whole run, or the channel that receives
/// the connections of a remote forward, and the tasks working for it.
/// Dropping it closes the listener and every connection.
pub(crate) struct ForwardRun {
    pub(crate) kind: TunnelKind,
    pub(crate) inbound: InboundSource,
    pub(crate) connections: JoinSet<()>,
    pub(crate) probes: JoinSet<()>,
    pub(crate) reports_sender: mpsc::UnboundedSender<TargetReport>,
    pub(crate) reports: mpsc::UnboundedReceiver<TargetReport>,
    pub(crate) throttle: MetricsThrottle,
}

impl InboundSource {
    /// Waits for the next thing that reaches the tunnel.
    pub(crate) async fn next(&mut self) -> Inbound {
        match self {
            InboundSource::Listener { listener, protocol } => Inbound::Client {
                accepted: listener.accept().await,
                protocol: *protocol,
            },
            InboundSource::Forwarded { receiver, .. } => match receiver.recv().await {
                Some(connection) => Inbound::Forwarded(connection),
                None => pending().await,
            },
        }
    }

    /// Where the server's connections for this run must be sent; `None` when
    /// the run is not a remote forward.
    pub(crate) fn forwarded_sender(&self) -> Option<ForwardedSender> {
        match self {
            InboundSource::Forwarded { sender, .. } => Some(sender.clone()),
            InboundSource::Listener { .. } => None,
        }
    }
}

impl ForwardRun {
    pub(crate) fn local(listener: TcpListener, metrics_event_interval: Duration) -> Self {
        Self::new(
            TunnelKind::Local,
            InboundSource::Listener {
                listener,
                protocol: ClientProtocol::Forward,
            },
            metrics_event_interval,
        )
    }

    pub(crate) fn dynamic(listener: TcpListener, metrics_event_interval: Duration) -> Self {
        Self::new(
            TunnelKind::Dynamic,
            InboundSource::Listener {
                listener,
                protocol: ClientProtocol::Socks,
            },
            metrics_event_interval,
        )
    }

    pub(crate) fn remote(metrics_event_interval: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self::new(
            TunnelKind::Remote,
            InboundSource::Forwarded { sender, receiver },
            metrics_event_interval,
        )
    }

    fn new(kind: TunnelKind, inbound: InboundSource, metrics_event_interval: Duration) -> Self {
        let (reports_sender, reports) = mpsc::unbounded_channel();
        Self {
            kind,
            inbound,
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
