use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::protocol::{Outcome, encode_response};

pub(crate) enum Frame {
    Line(Arc<str>),
    Close,
}

/// The bounded queue between the tasks producing messages for a connection
/// and the task writing them to its socket.
///
/// A queue that is full means the client stopped reading: the connection is
/// cancelled rather than letting the daemon wait or buffer without bound.
#[derive(Clone)]
pub(crate) struct OutboundQueue {
    sender: mpsc::Sender<Frame>,
    cancellation: CancellationToken,
}

impl OutboundQueue {
    pub(crate) fn new(
        capacity: usize,
        cancellation: CancellationToken,
    ) -> (Self, mpsc::Receiver<Frame>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (
            Self {
                sender,
                cancellation,
            },
            receiver,
        )
    }

    pub(crate) fn enqueue_line(&self, line: Arc<str>) -> bool {
        if self.sender.try_send(Frame::Line(line)).is_ok() {
            return true;
        }
        self.cancellation.cancel();
        false
    }

    pub(crate) fn respond(&self, id: Option<u64>, outcome: &Outcome) {
        match encode_response(id, outcome) {
            Ok(line) => {
                self.enqueue_line(Arc::from(line));
            }
            Err(error) => tracing::error!(%error, "cannot encode a response"),
        }
    }

    pub(crate) fn close(&self) {
        if self.sender.try_send(Frame::Close).is_err() {
            self.cancellation.cancel();
        }
    }
}
