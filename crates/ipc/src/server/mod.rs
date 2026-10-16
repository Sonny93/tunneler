//! The server side: accepts connections, runs the handshake, hands requests to
//! a [`RequestHandler`] and pushes events to subscribed clients.

mod connection;
mod handler;
mod outbound;
mod platform;
mod registry;

use std::future::pending;
use std::sync::Arc;
use std::time::Duration;

use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::traits::tokio::Listener as _;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::endpoint::Endpoint;
use crate::error::BindError;
use crate::protocol::EventMessage;
use connection::{Connection, ConnectionSettings};
use registry::Registry;

pub use handler::{ConnectionId, ConnectionInfo, PeerInfo, RequestHandler};
pub use registry::{ConnectionNotification, ServerHandle};

/// How many messages may wait for a slow client before it is disconnected.
pub const OUTBOUND_QUEUE_CAPACITY: usize = 256;

const ACCEPT_RETRY_DELAY: Duration = Duration::from_millis(100);

/// What a server can be tuned with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerOptions {
    /// The version reported to clients in the handshake.
    pub daemon_version: String,
    /// How many messages may wait for a slow client before it is disconnected.
    pub outbound_queue_capacity: usize,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
            outbound_queue_capacity: OUTBOUND_QUEUE_CAPACITY,
        }
    }
}

/// A bound endpoint, not yet serving.
///
/// Binding is what makes a daemon the single instance: it succeeds for one
/// process only. The listener releases the endpoint when dropped.
pub struct IpcServer {
    listener: Listener,
    registry: Arc<Registry>,
    options: ServerOptions,
}

impl IpcServer {
    /// Starts listening on `endpoint`, restricted to the current user.
    ///
    /// # Errors
    ///
    /// [`BindError::AddressInUse`] when a socket already exists there, whether
    /// a daemon owns it or crashed and left it; see
    /// [`Endpoint::is_daemon_alive`] and [`Endpoint::reclaim_stale`].
    pub fn bind(endpoint: &Endpoint) -> Result<Self, BindError> {
        Ok(Self {
            listener: platform::create_listener(endpoint)?,
            registry: Arc::new(Registry::new()),
            options: ServerOptions::default(),
        })
    }

    /// Replaces the default [`ServerOptions`].
    #[must_use]
    pub fn with_options(mut self, options: ServerOptions) -> Self {
        self.options = options;
        self
    }

    /// A handle to talk to clients; usable before and after [`serve`](Self::serve).
    pub fn handle(&self) -> ServerHandle {
        ServerHandle::new(Arc::clone(&self.registry))
    }

    /// Accepts clients until `shutdown` is cancelled.
    ///
    /// Events received from `events` are pushed to the connections that
    /// subscribed. On shutdown every connection is closed and the returned task
    /// completes once they are all gone.
    pub fn serve(
        self,
        handler: Arc<dyn RequestHandler>,
        events: broadcast::Receiver<EventMessage>,
        shutdown: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(self.run(handler, events, shutdown))
    }

    async fn run(
        self,
        handler: Arc<dyn RequestHandler>,
        events: broadcast::Receiver<EventMessage>,
        shutdown: CancellationToken,
    ) {
        let settings = Arc::new(ConnectionSettings {
            daemon_version: self.options.daemon_version.clone(),
            outbound_queue_capacity: self.options.outbound_queue_capacity,
        });
        let tracker = TaskTracker::new();
        let mut events = Some(events);
        loop {
            tokio::select! {
                () = shutdown.cancelled() => break,
                accepted = self.listener.accept() => match accepted {
                    Ok(stream) => self.spawn_connection(&tracker, stream, &handler, &settings, &shutdown),
                    Err(error) => {
                        tracing::warn!(%error, "cannot accept a connection");
                        tokio::time::sleep(ACCEPT_RETRY_DELAY).await;
                    }
                },
                received = next_event(&mut events) => self.forward(received, &mut events),
            }
        }
        tracker.close();
        tracker.wait().await;
    }

    fn spawn_connection(
        &self,
        tracker: &TaskTracker,
        stream: interprocess::local_socket::tokio::Stream,
        handler: &Arc<dyn RequestHandler>,
        settings: &Arc<ConnectionSettings>,
        shutdown: &CancellationToken,
    ) {
        let Some(peer) = platform::inspect_peer(&stream) else {
            return;
        };
        let connection = Connection {
            id: self.registry.allocate_id(),
            peer,
            registry: Arc::clone(&self.registry),
            handler: Arc::clone(handler),
            settings: Arc::clone(settings),
            cancellation: shutdown.child_token(),
        };
        tracker.spawn(connection.run(stream));
    }

    fn forward(
        &self,
        received: Result<EventMessage, RecvError>,
        events: &mut Option<broadcast::Receiver<EventMessage>>,
    ) {
        match received {
            Ok(event) => self.registry.publish(&event),
            Err(RecvError::Lagged(skipped)) => {
                tracing::warn!(skipped, "the event bus overran the server");
            }
            Err(RecvError::Closed) => *events = None,
        }
    }
}

async fn next_event(
    events: &mut Option<broadcast::Receiver<EventMessage>>,
) -> Result<EventMessage, RecvError> {
    match events {
        Some(receiver) => receiver.recv().await,
        None => pending().await,
    }
}
