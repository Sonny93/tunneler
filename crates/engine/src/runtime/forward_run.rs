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
use crate::runtime::standby::{BoundedQueue, HELD_CONNECTIONS_LIMIT, HeldClient};

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
/// A run in standby also holds the clients that connect while the tunnel has
/// no session. Dropping it closes the listener and every connection.
pub(crate) struct ForwardRun {
    pub(crate) inbound: InboundSource,
    pub(crate) connections: JoinSet<()>,
    pub(crate) reports_sender: mpsc::UnboundedSender<TargetReport>,
    pub(crate) reports: mpsc::UnboundedReceiver<TargetReport>,
    pub(crate) throttle: MetricsThrottle,
    held: Option<BoundedQueue<HeldClient>>,
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
            InboundSource::Listener {
                listener,
                protocol: ClientProtocol::Forward,
            },
            metrics_event_interval,
        )
    }

    pub(crate) fn dynamic(listener: TcpListener, metrics_event_interval: Duration) -> Self {
        Self::new(
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
            InboundSource::Forwarded { sender, receiver },
            metrics_event_interval,
        )
    }

    fn new(inbound: InboundSource, metrics_event_interval: Duration) -> Self {
        let (reports_sender, reports) = mpsc::unbounded_channel();
        Self {
            inbound,
            connections: JoinSet::new(),
            reports_sender,
            reports,
            throttle: MetricsThrottle::new(metrics_event_interval),
            held: None,
        }
    }

    /// Makes the run a standby one: it holds the clients that connect while
    /// there is no session instead of closing them.
    pub(crate) fn in_standby(mut self) -> Self {
        self.held = Some(BoundedQueue::new(HELD_CONNECTIONS_LIMIT));
        self
    }

    pub(crate) fn is_standby(&self) -> bool {
        self.held.is_some()
    }

    /// Holds `client` until the session is there; gives it back when the run
    /// is not a standby one or when too many clients already wait.
    pub(crate) fn hold(&mut self, client: HeldClient) -> Result<(), HeldClient> {
        match &mut self.held {
            Some(queue) => queue.push(client),
            None => Err(client),
        }
    }

    pub(crate) fn has_held(&self) -> bool {
        self.held.as_ref().is_some_and(|queue| !queue.is_empty())
    }

    /// Takes the clients that wait for the session, oldest first.
    pub(crate) fn take_held(&mut self) -> Vec<HeldClient> {
        self.held
            .as_mut()
            .map(BoundedQueue::take_all)
            .unwrap_or_default()
    }

    pub(crate) async fn close_connections(&mut self) {
        self.connections.abort_all();
        while self.connections.join_next().await.is_some() {}
    }
}
