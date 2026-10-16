use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::broadcast;

use crate::error::SendError;
use crate::protocol::{EventMessage, encode_event};
use crate::server::handler::{ConnectionId, ConnectionInfo};
use crate::server::outbound::OutboundQueue;

const NOTIFICATION_CAPACITY: usize = 64;

/// A connection joined or left the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionNotification {
    /// A client completed its handshake.
    Opened(ConnectionInfo),
    /// A connection that had completed its handshake ended.
    Closed(ConnectionId),
}

struct Entry {
    info: ConnectionInfo,
    queue: OutboundQueue,
}

pub(crate) struct Registry {
    entries: Mutex<HashMap<ConnectionId, Entry>>,
    notifications: broadcast::Sender<ConnectionNotification>,
    next_identifier: AtomicU64,
}

impl Registry {
    pub(crate) fn new() -> Self {
        let (notifications, _) = broadcast::channel(NOTIFICATION_CAPACITY);
        Self {
            entries: Mutex::new(HashMap::new()),
            notifications,
            next_identifier: AtomicU64::new(1),
        }
    }

    pub(crate) fn allocate_id(&self) -> ConnectionId {
        ConnectionId(self.next_identifier.fetch_add(1, Ordering::Relaxed))
    }

    pub(crate) fn register(&self, info: ConnectionInfo, queue: OutboundQueue) {
        let notification = ConnectionNotification::Opened(info.clone());
        self.lock().insert(info.id, Entry { info, queue });
        let _ = self.notifications.send(notification);
    }

    pub(crate) fn unregister(&self, id: ConnectionId) {
        if self.lock().remove(&id).is_some() {
            let _ = self.notifications.send(ConnectionNotification::Closed(id));
        }
    }

    pub(crate) fn publish(&self, event: &EventMessage) {
        let line = match encode_event(event) {
            Ok(line) => Arc::<str>::from(line),
            Err(error) => {
                tracing::error!(%error, "cannot encode an event");
                return;
            }
        };
        for entry in self
            .lock()
            .values()
            .filter(|entry| entry.info.subscribes_to_events)
        {
            deliver(entry, &line);
        }
    }

    pub(crate) fn send_to(&self, id: ConnectionId, event: &EventMessage) -> Result<(), SendError> {
        let line = Arc::<str>::from(encode_event(event)?);
        let entries = self.lock();
        let entry = entries.get(&id).ok_or(SendError::NotConnected)?;
        if deliver(entry, &line) {
            return Ok(());
        }
        Err(SendError::QueueFull)
    }

    pub(crate) fn connections(&self) -> Vec<ConnectionInfo> {
        let mut connections: Vec<ConnectionInfo> = self
            .lock()
            .values()
            .map(|entry| entry.info.clone())
            .collect();
        connections.sort_by_key(|info| info.id);
        connections
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<ConnectionNotification> {
        self.notifications.subscribe()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<ConnectionId, Entry>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn deliver(entry: &Entry, line: &Arc<str>) -> bool {
    if entry.queue.enqueue_line(Arc::clone(line)) {
        return true;
    }
    tracing::warn!(connection = %entry.info.id, "dropping a client that does not keep up");
    false
}

/// A cheap, cloneable view of the running server, for the daemon to talk to
/// individual clients and to follow who is connected.
#[derive(Clone)]
pub struct ServerHandle {
    registry: Arc<Registry>,
}

impl ServerHandle {
    pub(crate) fn new(registry: Arc<Registry>) -> Self {
        Self { registry }
    }

    /// The connections that completed their handshake, oldest first.
    pub fn connections(&self) -> Vec<ConnectionInfo> {
        self.registry.connections()
    }

    /// Sends an event to one connection, whether or not it subscribed.
    ///
    /// A connection whose outbound queue is full is disconnected.
    ///
    /// # Errors
    ///
    /// Fails when the connection is not open, when it was too slow, or when the
    /// event cannot be encoded.
    pub fn send_to(&self, id: ConnectionId, event: &EventMessage) -> Result<(), SendError> {
        self.registry.send_to(id, event)
    }

    /// Follows connections as they open and close.
    ///
    /// A receiver that falls behind misses notifications; read
    /// [`connections`](Self::connections) to catch up.
    pub fn subscribe_connections(&self) -> broadcast::Receiver<ConnectionNotification> {
        self.registry.subscribe()
    }
}
