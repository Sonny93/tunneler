use std::fmt;
use std::mem::discriminant;
use std::time::SystemTime;

use thiserror::Error;

use crate::external::PortOwner;

/// Why a tunnel is in the [`TunnelState::Failed`] state.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FailureReason {
    /// Another process already listens on the address.
    #[error("port already in use on {address}")]
    PortInUse {
        /// The `address:port` that could not be bound.
        address: String,
    },
    /// The listening socket could not be created for another reason.
    #[error("cannot listen on {address}: {message}")]
    ListenFailed {
        /// The `address:port` that could not be bound.
        address: String,
        /// What the system reported.
        message: String,
    },
    /// The SSH session could not be opened and retrying would not help.
    #[error("{message}")]
    Connect {
        /// A message ready to be shown to the user.
        message: String,
    },
    /// The SSH server refused to listen for a remote forward.
    #[error("the server refused to listen on {address}")]
    RemoteForwardRefused {
        /// The `address:port` the server was asked to listen on.
        address: String,
    },
}

/// Where a tunnel is in its life cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelState {
    /// Not running.
    Stopped,
    /// Listening and opening the SSH session for the first time.
    Starting,
    /// Forwarding.
    Active,
    /// The session was lost; the tunnel keeps listening and retries.
    Reconnecting {
        /// The number of the attempt about to be made, starting at 1.
        attempt: u32,
        /// When the attempt is made.
        next_retry_at: SystemTime,
    },
    /// The tunnel stopped by itself and will not retry.
    Failed {
        /// Why.
        reason: FailureReason,
    },
    /// Something else already provides the tunnel: an `ssh` process forwards
    /// the same port.
    External {
        /// The process providing the tunnel.
        owner: PortOwner,
    },
    /// Another process holds the listen port.
    Conflict {
        /// The process holding the port; `None` when it cannot be seen.
        owner: Option<PortOwner>,
    },
}

impl TunnelState {
    /// Whether the tunnel holds resources: it is starting, forwarding or
    /// waiting to reconnect.
    pub fn is_running(&self) -> bool {
        matches!(
            self,
            Self::Starting | Self::Active | Self::Reconnecting { .. }
        )
    }

    pub(crate) fn is_same_kind_as(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}

impl fmt::Display for TunnelState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stopped => formatter.write_str("stopped"),
            Self::Starting => formatter.write_str("starting"),
            Self::Active => formatter.write_str("active"),
            Self::Reconnecting { attempt, .. } => {
                write!(formatter, "reconnecting (attempt {attempt})")
            }
            Self::Failed { reason } => write!(formatter, "failed: {reason}"),
            Self::External { owner } => write!(
                formatter,
                "provided by an external {} (pid {})",
                owner.process, owner.pid
            ),
            Self::Conflict { owner: Some(owner) } => {
                write!(
                    formatter,
                    "port held by {} (pid {})",
                    owner.process, owner.pid
                )
            }
            Self::Conflict { owner: None } => {
                formatter.write_str("port held by an unknown process")
            }
        }
    }
}

/// Whether the service a tunnel forwards to answers, independently of the
/// state of the tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetHealth {
    /// Nothing is known yet.
    Unknown,
    /// The last attempt to reach the target succeeded.
    Reachable,
    /// The target refused or could not be reached.
    Unreachable {
        /// When the target first went down; kept while it stays down.
        since: SystemTime,
        /// The latest failure.
        error: String,
    },
}

impl TargetHealth {
    pub(crate) fn after_failure(&self, now: SystemTime, error: String) -> Self {
        let since = match self {
            Self::Unreachable { since, .. } => *since,
            Self::Unknown | Self::Reachable => now,
        };
        Self::Unreachable { since, error }
    }
}

/// The state and the target health of a tunnel at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelStatus {
    /// The life-cycle state.
    pub state: TunnelState,
    /// The health of the target.
    pub health: TargetHealth,
}

impl TunnelStatus {
    pub(crate) fn stopped() -> Self {
        Self {
            state: TunnelState::Stopped,
            health: TargetHealth::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn owner() -> PortOwner {
        PortOwner {
            pid: 4242,
            process: "ssh".to_owned(),
        }
    }

    #[test]
    fn should_not_be_running_when_external_or_in_conflict() {
        let external = TunnelState::External { owner: owner() };
        let conflict = TunnelState::Conflict { owner: None };

        assert!(!external.is_running() && !conflict.is_running());
    }

    #[test]
    fn should_describe_the_owner_when_displaying_an_external_state() {
        let state = TunnelState::External { owner: owner() };

        assert_eq!(state.to_string(), "provided by an external ssh (pid 4242)");
    }

    #[test]
    fn should_describe_the_owner_when_displaying_a_conflict() {
        let state = TunnelState::Conflict {
            owner: Some(owner()),
        };

        assert_eq!(state.to_string(), "port held by ssh (pid 4242)");
    }

    #[test]
    fn should_say_the_owner_is_unknown_when_displaying_a_conflict_without_owner() {
        let state = TunnelState::Conflict { owner: None };

        assert_eq!(state.to_string(), "port held by an unknown process");
    }

    #[test]
    fn should_not_be_running_when_stopped_or_failed() {
        let failed = TunnelState::Failed {
            reason: FailureReason::RemoteForwardRefused {
                address: "localhost:8080".to_owned(),
            },
        };

        assert!(!TunnelState::Stopped.is_running() && !failed.is_running());
    }

    #[test]
    fn should_be_running_when_starting_active_or_reconnecting() {
        let reconnecting = TunnelState::Reconnecting {
            attempt: 1,
            next_retry_at: at(0),
        };

        assert!(
            TunnelState::Starting.is_running()
                && TunnelState::Active.is_running()
                && reconnecting.is_running()
        );
    }

    #[test]
    fn should_be_of_the_same_kind_when_only_the_attempt_differs() {
        let first = TunnelState::Reconnecting {
            attempt: 1,
            next_retry_at: at(0),
        };
        let second = TunnelState::Reconnecting {
            attempt: 2,
            next_retry_at: at(5),
        };

        assert!(first.is_same_kind_as(&second));
    }

    #[test]
    fn should_not_be_of_the_same_kind_when_the_variants_differ() {
        assert!(!TunnelState::Starting.is_same_kind_as(&TunnelState::Stopped));
    }

    #[test]
    fn should_start_the_outage_now_when_the_target_was_reachable() {
        let health = TargetHealth::Reachable.after_failure(at(10), "refused".to_owned());

        assert_eq!(
            health,
            TargetHealth::Unreachable {
                since: at(10),
                error: "refused".to_owned()
            }
        );
    }

    #[test]
    fn should_keep_the_earliest_since_when_the_target_stays_down() {
        let down = TargetHealth::Unreachable {
            since: at(10),
            error: "refused".to_owned(),
        };

        assert_eq!(
            down.after_failure(at(99), "timeout".to_owned()),
            TargetHealth::Unreachable {
                since: at(10),
                error: "timeout".to_owned()
            }
        );
    }

    #[test]
    fn should_describe_the_failure_when_the_port_is_in_use() {
        let reason = FailureReason::PortInUse {
            address: "127.0.0.1:3333".to_owned(),
        };

        assert_eq!(reason.to_string(), "port already in use on 127.0.0.1:3333");
    }

    #[test]
    fn should_name_the_address_when_the_server_refuses_a_remote_forward() {
        let reason = FailureReason::RemoteForwardRefused {
            address: "localhost:8080".to_owned(),
        };

        assert_eq!(
            reason.to_string(),
            "the server refused to listen on localhost:8080"
        );
    }

    #[test]
    fn should_include_the_reason_when_displaying_a_failed_state() {
        let state = TunnelState::Failed {
            reason: FailureReason::Connect {
                message: "authentication failed".to_owned(),
            },
        };

        assert_eq!(state.to_string(), "failed: authentication failed");
    }
}
