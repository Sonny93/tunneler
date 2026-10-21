use std::time::Duration;

use crate::error::ConnectError;
use crate::runtime::backoff::backoff_delay;
use crate::runtime::failure::is_retryable;
use crate::runtime::state::FailureReason;
use crate::store::Settings;

/// What a tunnel does after a failed connection attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RetryDecision {
    /// Wait, then try again.
    Retry {
        /// How long to wait.
        delay: Duration,
    },
    /// Give up.
    Fail(FailureReason),
}

/// The wait before reconnection attempt number `attempt` (1-based).
pub(crate) fn reconnect_delay(attempt: u32, settings: &Settings, jitter_fraction: f64) -> Duration {
    backoff_delay(
        attempt,
        settings.reconnect_initial_delay,
        settings.reconnect_max_delay,
        jitter_fraction,
    )
}

/// Decides, for the failed attempt number `attempt` (1-based), whether to
/// retry and after how long.
pub(crate) fn decide_after_connect_error(
    error: &ConnectError,
    attempt: u32,
    settings: &Settings,
    jitter_fraction: f64,
) -> RetryDecision {
    if !is_retryable(error) {
        return RetryDecision::Fail(failure_reason_of(error));
    }
    RetryDecision::Retry {
        delay: reconnect_delay(attempt, settings, jitter_fraction),
    }
}

/// The reason shown to the user when `error` ends a connection attempt.
pub(crate) fn failure_reason_of(error: &ConnectError) -> FailureReason {
    match error {
        ConnectError::AgentUnresponsive { timeout } => {
            FailureReason::AgentUnresponsive { timeout: *timeout }
        }
        ConnectError::Jump { source, .. } => failure_reason_of(source),
        _ => FailureReason::Connect {
            message: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use crate::error::TransportError;

    use super::*;

    fn refused() -> ConnectError {
        ConnectError::Transport(TransportError::Connect {
            address: SocketAddr::from(([127, 0, 0, 1], 22)),
            source: std::io::Error::from(std::io::ErrorKind::ConnectionRefused),
        })
    }

    #[test]
    fn should_retry_with_the_backoff_delay_when_the_error_is_retryable() {
        let settings = Settings::default();

        let decision = decide_after_connect_error(&refused(), 1, &settings, 1.0);

        assert_eq!(
            decision,
            RetryDecision::Retry {
                delay: settings.reconnect_initial_delay
            }
        );
    }

    #[test]
    fn should_wait_longer_when_the_attempt_number_grows() {
        let settings = Settings::default();

        let decision = decide_after_connect_error(&refused(), 3, &settings, 1.0);

        assert_eq!(
            decision,
            RetryDecision::Retry {
                delay: settings.reconnect_initial_delay * 4
            }
        );
    }

    #[test]
    fn should_fail_with_the_message_when_the_error_is_fatal() {
        let error = ConnectError::Authentication { tried: Vec::new() };

        let decision = decide_after_connect_error(&error, 1, &Settings::default(), 0.5);

        assert_eq!(
            decision,
            RetryDecision::Fail(FailureReason::Connect {
                message: error.to_string()
            })
        );
    }

    #[test]
    fn should_fail_with_an_unresponsive_agent_when_the_agent_does_not_answer() {
        let timeout = Duration::from_secs(30);
        let error = ConnectError::AgentUnresponsive { timeout };

        let decision = decide_after_connect_error(&error, 1, &Settings::default(), 0.5);

        assert_eq!(
            decision,
            RetryDecision::Fail(FailureReason::AgentUnresponsive { timeout })
        );
    }

    #[test]
    fn should_fail_with_an_unresponsive_agent_when_a_jump_host_agent_does_not_answer() {
        let timeout = Duration::from_secs(30);
        let error = ConnectError::Jump {
            hop: "bastion".to_owned(),
            source: Box::new(ConnectError::AgentUnresponsive { timeout }),
        };

        let decision = decide_after_connect_error(&error, 1, &Settings::default(), 0.5);

        assert_eq!(
            decision,
            RetryDecision::Fail(FailureReason::AgentUnresponsive { timeout })
        );
    }

    #[test]
    fn should_retry_when_authentication_times_out() {
        let error = ConnectError::AuthenticationTimeout {
            timeout: Duration::from_secs(30),
        };

        let decision = decide_after_connect_error(&error, 1, &Settings::default(), 1.0);

        assert!(matches!(decision, RetryDecision::Retry { .. }));
    }
}
