use std::time::Duration;

use crate::connect_timeouts::ConnectTimeouts;

/// The shortest gap between two metrics events of one tunnel.
pub(crate) const METRICS_EVENT_INTERVAL: Duration = Duration::from_secs(1);

/// How often the listening ports are scanned while port scanning is on.
pub(crate) const PORT_SCAN_INTERVAL: Duration = Duration::from_secs(5);

const SHORTEST_INTERVAL: Duration = Duration::from_millis(1);

/// Tunes the timing of the runtime. The defaults suit production; tests
/// shorten the intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineOptions {
    /// The shortest gap between two metrics events of one tunnel. Defaults to
    /// one second.
    pub metrics_event_interval: Duration,
    /// How often the listening ports are scanned for external tunnels while
    /// port scanning is on. Defaults to five seconds.
    pub port_scan_interval: Duration,
    /// How long opening a session may wait on the network or the SSH agent.
    /// Defaults to 30 seconds for each step.
    pub connect_timeouts: ConnectTimeouts,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            metrics_event_interval: METRICS_EVENT_INTERVAL,
            port_scan_interval: PORT_SCAN_INTERVAL,
            connect_timeouts: ConnectTimeouts::default(),
        }
    }
}

impl EngineOptions {
    pub(crate) fn sanitized(self) -> Self {
        Self {
            metrics_event_interval: self.metrics_event_interval.max(SHORTEST_INTERVAL),
            port_scan_interval: self.port_scan_interval.max(SHORTEST_INTERVAL),
            connect_timeouts: self.connect_timeouts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_emit_metrics_at_most_once_a_second_by_default() {
        assert_eq!(
            EngineOptions::default().metrics_event_interval,
            Duration::from_secs(1)
        );
    }

    #[test]
    fn should_scan_ports_every_five_seconds_by_default() {
        assert_eq!(
            EngineOptions::default().port_scan_interval,
            Duration::from_secs(5)
        );
    }

    #[test]
    fn should_raise_a_zero_scan_interval_when_sanitizing() {
        let options = EngineOptions {
            port_scan_interval: Duration::ZERO,
            ..EngineOptions::default()
        }
        .sanitized();

        assert_eq!(options.port_scan_interval, SHORTEST_INTERVAL);
    }

    #[test]
    fn should_raise_a_zero_interval_when_sanitizing() {
        let options = EngineOptions {
            metrics_event_interval: Duration::ZERO,
            ..EngineOptions::default()
        }
        .sanitized();

        assert_eq!(options.metrics_event_interval, SHORTEST_INTERVAL);
    }
}
