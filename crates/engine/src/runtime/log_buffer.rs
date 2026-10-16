use std::collections::VecDeque;
use std::time::SystemTime;

/// How many entries a tunnel keeps; the oldest are dropped first.
pub(crate) const TUNNEL_LOG_CAPACITY: usize = 500;

/// How serious a log entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Something expected happened.
    Info,
    /// Something went wrong but the tunnel carries on.
    Warn,
    /// Something went wrong and the tunnel stopped working.
    Error,
}

/// One line of a tunnel's log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelLogEntry {
    /// When it happened.
    pub at: SystemTime,
    /// How serious it is.
    pub level: LogLevel,
    /// What happened. Never contains secrets.
    pub message: String,
}

/// The most recent log entries of one tunnel.
#[derive(Debug)]
pub(crate) struct LogBuffer {
    capacity: usize,
    entries: VecDeque<TunnelLogEntry>,
}

impl LogBuffer {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::with_capacity(capacity),
        }
    }

    pub(crate) fn push(&mut self, entry: TunnelLogEntry) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub(crate) fn entries(&self) -> Vec<TunnelLogEntry> {
        self.entries.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;

    fn entry(number: u64) -> TunnelLogEntry {
        TunnelLogEntry {
            at: UNIX_EPOCH + Duration::from_secs(number),
            level: LogLevel::Info,
            message: format!("entry {number}"),
        }
    }

    fn messages(buffer: &LogBuffer) -> Vec<String> {
        buffer
            .entries()
            .into_iter()
            .map(|entry| entry.message)
            .collect()
    }

    #[test]
    fn should_keep_the_entries_in_order_when_under_capacity() {
        let mut buffer = LogBuffer::new(3);

        buffer.push(entry(1));
        buffer.push(entry(2));

        assert_eq!(messages(&buffer), ["entry 1", "entry 2"]);
    }

    #[test]
    fn should_drop_the_oldest_entry_when_over_capacity() {
        let mut buffer = LogBuffer::new(2);

        (1..=3).for_each(|number| buffer.push(entry(number)));

        assert_eq!(messages(&buffer), ["entry 2", "entry 3"]);
    }

    #[test]
    fn should_never_hold_more_than_the_capacity_when_pushing_many_entries() {
        let mut buffer = LogBuffer::new(TUNNEL_LOG_CAPACITY);

        (0..2000).for_each(|number| buffer.push(entry(number)));

        assert_eq!(buffer.entries().len(), TUNNEL_LOG_CAPACITY);
    }

    #[test]
    fn should_start_empty_when_created() {
        assert_eq!(LogBuffer::new(3).entries().len(), 0);
    }
}
