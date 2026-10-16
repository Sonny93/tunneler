use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::time::Instant;

use crate::clock::Clock;

const NO_ACTIVITY: u64 = 0;

/// What a tunnel carried since it was started.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TunnelMetrics {
    /// The connections being forwarded right now.
    pub active_connections: usize,
    /// The connections accepted since the tunnel started.
    pub total_connections: u64,
    /// The bytes received from the target and written to the clients.
    pub bytes_in: u64,
    /// The bytes read from the clients and sent to the target.
    pub bytes_out: u64,
    /// When bytes last moved or a connection was accepted.
    pub last_activity: Option<SystemTime>,
}

/// The live counters behind [`TunnelMetrics`], updated from the connection
/// tasks without locking.
pub(crate) struct MetricsCounters {
    clock: Arc<dyn Clock>,
    active_connections: AtomicUsize,
    total_connections: AtomicU64,
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
    last_activity_milliseconds: AtomicU64,
}

impl MetricsCounters {
    pub(crate) fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            active_connections: AtomicUsize::new(0),
            total_connections: AtomicU64::new(0),
            bytes_in: AtomicU64::new(0),
            bytes_out: AtomicU64::new(0),
            last_activity_milliseconds: AtomicU64::new(NO_ACTIVITY),
        }
    }

    pub(crate) fn snapshot(&self) -> TunnelMetrics {
        TunnelMetrics {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_connections: self.total_connections.load(Ordering::Relaxed),
            bytes_in: self.bytes_in.load(Ordering::Relaxed),
            bytes_out: self.bytes_out.load(Ordering::Relaxed),
            last_activity: self.last_activity(),
        }
    }

    pub(crate) fn reset(&self) {
        self.total_connections.store(0, Ordering::Relaxed);
        self.bytes_in.store(0, Ordering::Relaxed);
        self.bytes_out.store(0, Ordering::Relaxed);
        self.last_activity_milliseconds
            .store(NO_ACTIVITY, Ordering::Relaxed);
    }

    pub(crate) fn begin_connection(self: &Arc<Self>) -> ActiveConnection {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.record_activity();
        ActiveConnection {
            counters: Arc::clone(self),
        }
    }

    pub(crate) fn record_bytes_in(&self, count: usize) {
        self.bytes_in.fetch_add(count as u64, Ordering::Relaxed);
        self.record_activity();
    }

    pub(crate) fn record_bytes_out(&self, count: usize) {
        self.bytes_out.fetch_add(count as u64, Ordering::Relaxed);
        self.record_activity();
    }

    fn record_activity(&self) {
        let milliseconds = self
            .clock
            .now()
            .duration_since(UNIX_EPOCH)
            .map_or(NO_ACTIVITY, |elapsed| {
                u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
            });
        self.last_activity_milliseconds
            .store(milliseconds, Ordering::Relaxed);
    }

    fn last_activity(&self) -> Option<SystemTime> {
        match self.last_activity_milliseconds.load(Ordering::Relaxed) {
            NO_ACTIVITY => None,
            milliseconds => Some(UNIX_EPOCH + Duration::from_millis(milliseconds)),
        }
    }
}

/// Counts one connection as active until it is dropped.
pub(crate) struct ActiveConnection {
    counters: Arc<MetricsCounters>,
}

impl Drop for ActiveConnection {
    fn drop(&mut self) {
        self.counters
            .active_connections
            .fetch_sub(1, Ordering::Relaxed);
    }
}

/// Decides when a metrics event is worth sending: at most one per interval,
/// and only when the numbers changed since the last one.
pub(crate) struct MetricsThrottle {
    minimum_interval: Duration,
    last_emitted_at: Option<Instant>,
    last_emitted: TunnelMetrics,
}

impl MetricsThrottle {
    pub(crate) fn new(minimum_interval: Duration) -> Self {
        Self {
            minimum_interval,
            last_emitted_at: None,
            last_emitted: TunnelMetrics::default(),
        }
    }

    pub(crate) fn should_emit_at(&mut self, now: Instant, metrics: &TunnelMetrics) -> bool {
        if *metrics == self.last_emitted {
            return false;
        }
        let is_too_soon = self
            .last_emitted_at
            .is_some_and(|last| now.saturating_duration_since(last) < self.minimum_interval);
        if is_too_soon {
            return false;
        }
        self.record(now, metrics);
        true
    }

    pub(crate) fn should_flush(&mut self, now: Instant, metrics: &TunnelMetrics) -> bool {
        if *metrics == self.last_emitted {
            return false;
        }
        self.record(now, metrics);
        true
    }

    fn record(&mut self, now: Instant, metrics: &TunnelMetrics) {
        self.last_emitted_at = Some(now);
        self.last_emitted = *metrics;
    }
}

#[cfg(test)]
mod tests {
    use crate::clock::ManualClock;

    use super::*;

    const INTERVAL: Duration = Duration::from_secs(1);

    fn counters() -> (Arc<ManualClock>, Arc<MetricsCounters>) {
        let clock = Arc::new(ManualClock::default());
        let counters = Arc::new(MetricsCounters::new(Arc::clone(&clock) as _));
        (clock, counters)
    }

    fn busy() -> TunnelMetrics {
        TunnelMetrics {
            bytes_in: 10,
            ..TunnelMetrics::default()
        }
    }

    #[test]
    fn should_start_with_empty_metrics_when_nothing_happened() {
        let (_, counters) = counters();

        assert_eq!(counters.snapshot(), TunnelMetrics::default());
    }

    #[test]
    fn should_count_the_connection_as_active_while_the_guard_lives() {
        let (_, counters) = counters();

        let _guard = counters.begin_connection();

        assert_eq!(counters.snapshot().active_connections, 1);
    }

    #[test]
    fn should_stop_counting_the_connection_as_active_when_the_guard_drops() {
        let (_, counters) = counters();

        drop(counters.begin_connection());

        assert_eq!(counters.snapshot().active_connections, 0);
    }

    #[test]
    fn should_keep_counting_the_connection_in_the_total_when_the_guard_drops() {
        let (_, counters) = counters();

        drop(counters.begin_connection());

        assert_eq!(counters.snapshot().total_connections, 1);
    }

    #[test]
    fn should_accumulate_bytes_in_each_direction_when_recording() {
        let (_, counters) = counters();

        counters.record_bytes_in(5);
        counters.record_bytes_in(7);
        counters.record_bytes_out(3);

        let snapshot = counters.snapshot();
        assert_eq!((snapshot.bytes_in, snapshot.bytes_out), (12, 3));
    }

    #[test]
    fn should_report_the_clock_time_when_bytes_moved() {
        let (clock, counters) = counters();
        let before = clock.now();

        counters.record_bytes_out(1);

        assert_eq!(counters.snapshot().last_activity, Some(before));
    }

    #[test]
    fn should_report_no_activity_when_nothing_moved() {
        let (_, counters) = counters();

        assert_eq!(counters.snapshot().last_activity, None);
    }

    #[test]
    fn should_clear_the_totals_when_reset() {
        let (_, counters) = counters();
        counters.record_bytes_in(5);

        counters.reset();

        assert_eq!(counters.snapshot(), TunnelMetrics::default());
    }

    #[test]
    fn should_keep_the_active_connections_when_reset() {
        let (_, counters) = counters();
        let _guard = counters.begin_connection();

        counters.reset();

        assert_eq!(counters.snapshot().active_connections, 1);
    }

    #[test]
    fn should_not_emit_when_nothing_changed_since_the_start() {
        let mut throttle = MetricsThrottle::new(INTERVAL);

        assert!(!throttle.should_emit_at(Instant::now(), &TunnelMetrics::default()));
    }

    #[test]
    fn should_emit_when_the_metrics_changed_and_nothing_was_sent_yet() {
        let mut throttle = MetricsThrottle::new(INTERVAL);

        assert!(throttle.should_emit_at(Instant::now(), &busy()));
    }

    #[test]
    fn should_not_emit_twice_when_the_metrics_did_not_change() {
        let mut throttle = MetricsThrottle::new(INTERVAL);
        let start = Instant::now();
        throttle.should_emit_at(start, &busy());

        assert!(!throttle.should_emit_at(start + INTERVAL * 2, &busy()));
    }

    #[test]
    fn should_not_emit_when_the_last_event_is_more_recent_than_the_interval() {
        let mut throttle = MetricsThrottle::new(INTERVAL);
        let start = Instant::now();
        throttle.should_emit_at(start, &busy());
        let changed = TunnelMetrics {
            bytes_in: 99,
            ..busy()
        };

        assert!(!throttle.should_emit_at(start + INTERVAL / 2, &changed));
    }

    #[test]
    fn should_emit_when_the_interval_elapsed_and_the_metrics_changed() {
        let mut throttle = MetricsThrottle::new(INTERVAL);
        let start = Instant::now();
        throttle.should_emit_at(start, &busy());
        let changed = TunnelMetrics {
            bytes_in: 99,
            ..busy()
        };

        assert!(throttle.should_emit_at(start + INTERVAL, &changed));
    }

    #[test]
    fn should_flush_a_change_when_the_interval_has_not_elapsed() {
        let mut throttle = MetricsThrottle::new(INTERVAL);
        let start = Instant::now();
        throttle.should_emit_at(start, &busy());
        let changed = TunnelMetrics {
            bytes_in: 99,
            ..busy()
        };

        assert!(throttle.should_flush(start, &changed));
    }

    #[test]
    fn should_not_flush_when_the_metrics_did_not_change() {
        let mut throttle = MetricsThrottle::new(INTERVAL);
        let start = Instant::now();
        throttle.should_emit_at(start, &busy());

        assert!(!throttle.should_flush(start, &busy()));
    }
}
