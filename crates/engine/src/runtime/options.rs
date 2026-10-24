use std::time::Duration;

use crate::connect_timeouts::ConnectTimeouts;

/// The shortest gap between two metrics events of one tunnel.
pub(crate) const METRICS_EVENT_INTERVAL: Duration = Duration::from_secs(1);

/// How often the listening ports are scanned while port scanning is on.
pub(crate) const PORT_SCAN_INTERVAL: Duration = Duration::from_secs(5);

/// How long [`Engine::start_tunnel_and_wait`](crate::Engine::start_tunnel_and_wait)
/// waits for a tunnel to become ready. Long, because a credential prompt may
/// be waiting for the user.
pub(crate) const TUNNEL_READY_TIMEOUT: Duration = Duration::from_secs(120);

/// How long starting an X server waits for its display to accept connections.
pub(crate) const X11_SERVER_START_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the display of a starting X server is tried.
pub(crate) const X11_SERVER_POLL_INTERVAL: Duration = Duration::from_millis(200);

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
    /// How long starting a tunnel and waiting for it to be ready may take.
    /// Defaults to 120 seconds, since a credential prompt may be pending.
    pub tunnel_ready_timeout: Duration,
    /// How long starting an X server waits for its display to accept
    /// connections. Defaults to 10 seconds.
    pub x11_server_start_timeout: Duration,
    /// How often the display of a starting X server is tried. Defaults to
    /// 200 milliseconds.
    pub x11_server_poll_interval: Duration,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            metrics_event_interval: METRICS_EVENT_INTERVAL,
            port_scan_interval: PORT_SCAN_INTERVAL,
            connect_timeouts: ConnectTimeouts::default(),
            tunnel_ready_timeout: TUNNEL_READY_TIMEOUT,
            x11_server_start_timeout: X11_SERVER_START_TIMEOUT,
            x11_server_poll_interval: X11_SERVER_POLL_INTERVAL,
        }
    }
}

impl EngineOptions {
    pub(crate) fn sanitized(self) -> Self {
        Self {
            metrics_event_interval: self.metrics_event_interval.max(SHORTEST_INTERVAL),
            port_scan_interval: self.port_scan_interval.max(SHORTEST_INTERVAL),
            connect_timeouts: self.connect_timeouts,
            tunnel_ready_timeout: self.tunnel_ready_timeout.max(SHORTEST_INTERVAL),
            x11_server_start_timeout: self.x11_server_start_timeout.max(SHORTEST_INTERVAL),
            x11_server_poll_interval: self.x11_server_poll_interval.max(SHORTEST_INTERVAL),
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
    fn should_wait_two_minutes_for_a_tunnel_to_be_ready_by_default() {
        assert_eq!(
            EngineOptions::default().tunnel_ready_timeout,
            Duration::from_secs(120)
        );
    }

    #[test]
    fn should_wait_ten_seconds_for_an_x_server_polled_every_200_milliseconds_by_default() {
        let options = EngineOptions::default();

        assert_eq!(options.x11_server_start_timeout, Duration::from_secs(10));
        assert_eq!(options.x11_server_poll_interval, Duration::from_millis(200));
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
