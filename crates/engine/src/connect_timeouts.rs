use std::time::Duration;

/// The default for every bound of [`ConnectTimeouts`].
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_AUTHENTICATION_STEP_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_AGENT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// How long opening a session may wait on something that is not a person.
///
/// None of these bounds ever runs while the user is being asked for a
/// passphrase, a password or a host key decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectTimeouts {
    /// The TCP connection, and the SSH handshake of a host whose
    /// `ConnectTimeout` is not set.
    pub connect: Duration,
    /// The whole authentication phase, without the time spent on prompts and
    /// on the SSH agent.
    pub authentication_step: Duration,
    /// Each request to the SSH agent, such as a signature that the agent
    /// waits for the user to confirm.
    pub agent_response: Duration,
}

impl Default for ConnectTimeouts {
    fn default() -> Self {
        Self {
            connect: DEFAULT_CONNECT_TIMEOUT,
            authentication_step: DEFAULT_AUTHENTICATION_STEP_TIMEOUT,
            agent_response: DEFAULT_AGENT_RESPONSE_TIMEOUT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_bound_every_step_to_thirty_seconds_by_default() {
        let thirty_seconds = Duration::from_secs(30);
        assert_eq!(
            ConnectTimeouts::default(),
            ConnectTimeouts {
                connect: thirty_seconds,
                authentication_step: thirty_seconds,
                agent_response: thirty_seconds,
            }
        );
    }
}
