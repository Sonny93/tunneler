use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::Value;
use tokio::sync::oneshot;

use crate::error::ClientError;
use crate::protocol::IpcError;

type Answer = Result<Value, IpcError>;

struct State {
    waiting: HashMap<u64, oneshot::Sender<Answer>>,
    is_closed: bool,
}

/// The calls sent and not yet answered, by request identifier.
pub(crate) struct PendingCalls {
    state: Mutex<State>,
    next_identifier: AtomicU64,
}

impl PendingCalls {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(State {
                waiting: HashMap::new(),
                is_closed: false,
            }),
            next_identifier: AtomicU64::new(1),
        }
    }

    pub(crate) fn register(&self) -> Result<(u64, oneshot::Receiver<Answer>), ClientError> {
        let mut state = self.lock();
        if state.is_closed {
            return Err(ClientError::ConnectionClosed);
        }
        let id = self.next_identifier.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        state.waiting.insert(id, sender);
        Ok((id, receiver))
    }

    pub(crate) fn forget(&self, id: u64) {
        self.lock().waiting.remove(&id);
    }

    pub(crate) fn resolve(&self, id: u64, answer: Answer) {
        let Some(sender) = self.lock().waiting.remove(&id) else {
            tracing::debug!(id, "response to an unknown request");
            return;
        };
        let _ = sender.send(answer);
    }

    pub(crate) fn close(&self) {
        let mut state = self.lock();
        state.is_closed = true;
        state.waiting.clear();
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
