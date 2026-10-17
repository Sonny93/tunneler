use std::fmt;
use std::time::SystemTime;

/// Where the X11 anchor of a host is in its life cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X11AnchorState {
    /// Not running.
    Stopped,
    /// Opening the SSH session and asking the server for X11 forwarding.
    Starting,
    /// The server forwards X11 to this machine.
    Active {
        /// The `DISPLAY` to set on the server, such as `localhost:10.0`.
        display: String,
    },
    /// The session was lost; the anchor retries.
    Reconnecting {
        /// The number of the attempt about to be made, starting at 1.
        attempt: u32,
        /// When the attempt is made.
        next_retry_at: SystemTime,
    },
    /// The anchor stopped by itself and will not retry.
    Failed {
        /// Why, ready to be shown to the user.
        message: String,
    },
}

impl X11AnchorState {
    /// Whether the anchor holds resources: it is starting, active or waiting
    /// to reconnect.
    pub fn is_running(&self) -> bool {
        matches!(
            self,
            Self::Starting | Self::Active { .. } | Self::Reconnecting { .. }
        )
    }
}

impl fmt::Display for X11AnchorState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stopped => formatter.write_str("stopped"),
            Self::Starting => formatter.write_str("starting"),
            Self::Active { display } => write!(formatter, "active (DISPLAY={display})"),
            Self::Reconnecting { attempt, .. } => {
                write!(formatter, "reconnecting (attempt {attempt})")
            }
            Self::Failed { message } => write!(formatter, "failed: {message}"),
        }
    }
}

/// The X11 anchor of a host at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11AnchorSnapshot {
    /// The `ssh_config` alias the anchor runs on.
    pub host: String,
    /// Its state.
    pub state: X11AnchorState,
}

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;

    use super::*;

    #[test]
    fn should_describe_the_display_when_the_anchor_is_active() {
        let state = X11AnchorState::Active {
            display: "localhost:10.0".to_owned(),
        };

        assert_eq!(state.to_string(), "active (DISPLAY=localhost:10.0)");
    }

    #[test]
    fn should_describe_the_attempt_when_the_anchor_is_reconnecting() {
        let state = X11AnchorState::Reconnecting {
            attempt: 3,
            next_retry_at: UNIX_EPOCH,
        };

        assert_eq!(state.to_string(), "reconnecting (attempt 3)");
    }

    #[test]
    fn should_describe_the_message_when_the_anchor_failed() {
        let state = X11AnchorState::Failed {
            message: "the server refused X11 forwarding".to_owned(),
        };

        assert_eq!(
            state.to_string(),
            "failed: the server refused X11 forwarding"
        );
    }

    #[test]
    fn should_hold_resources_when_the_anchor_is_starting() {
        assert!(X11AnchorState::Starting.is_running());
    }

    #[test]
    fn should_hold_no_resources_when_the_anchor_failed() {
        let state = X11AnchorState::Failed {
            message: String::new(),
        };

        assert!(!state.is_running());
    }

    #[test]
    fn should_hold_no_resources_when_the_anchor_is_stopped() {
        assert!(!X11AnchorState::Stopped.is_running());
    }
}
