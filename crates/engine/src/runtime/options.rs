use std::time::Duration;

/// How often an active tunnel probes its target while probing is on.
pub(crate) const HEALTH_PROBE_INTERVAL: Duration = Duration::from_secs(30);

/// The shortest gap between two metrics events of one tunnel.
pub(crate) const METRICS_EVENT_INTERVAL: Duration = Duration::from_secs(1);

const SHORTEST_INTERVAL: Duration = Duration::from_millis(1);

/// Tunes the timing of the runtime. The defaults suit production; tests
/// shorten the intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineOptions {
    /// How often an active tunnel opens and closes a channel to its target
    /// while health probing is on. Defaults to 30 seconds.
    pub health_probe_interval: Duration,
    /// The shortest gap between two metrics events of one tunnel. Defaults to
    /// one second.
    pub metrics_event_interval: Duration,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            health_probe_interval: HEALTH_PROBE_INTERVAL,
            metrics_event_interval: METRICS_EVENT_INTERVAL,
        }
    }
}

impl EngineOptions {
    pub(crate) fn sanitized(self) -> Self {
        Self {
            health_probe_interval: self.health_probe_interval.max(SHORTEST_INTERVAL),
            metrics_event_interval: self.metrics_event_interval.max(SHORTEST_INTERVAL),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_probe_every_thirty_seconds_by_default() {
        assert_eq!(
            EngineOptions::default().health_probe_interval,
            Duration::from_secs(30)
        );
    }

    #[test]
    fn should_emit_metrics_at_most_once_a_second_by_default() {
        assert_eq!(
            EngineOptions::default().metrics_event_interval,
            Duration::from_secs(1)
        );
    }

    #[test]
    fn should_raise_a_zero_interval_when_sanitizing() {
        let options = EngineOptions {
            health_probe_interval: Duration::ZERO,
            metrics_event_interval: Duration::ZERO,
        }
        .sanitized();

        assert_eq!(options.health_probe_interval, SHORTEST_INTERVAL);
    }
}
