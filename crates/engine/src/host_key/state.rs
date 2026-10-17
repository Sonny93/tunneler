use std::sync::{Mutex, PoisonError};

use crate::bounded::{DeadlinePause, PauseGuard};
use crate::error::HostKeyFailure;

/// What the russh handler shares with `connect`: the reason a host key was
/// refused, and the pause of the handshake clock while the user decides.
#[derive(Debug, Default)]
pub(crate) struct HandshakeState {
    failure: Mutex<Option<HostKeyFailure>>,
    deadline_pause: DeadlinePause,
}

impl HandshakeState {
    pub(crate) fn record_failure(&self, failure: HostKeyFailure) {
        *self.failure.lock().unwrap_or_else(PoisonError::into_inner) = Some(failure);
    }

    pub(crate) fn take_failure(&self) -> Option<HostKeyFailure> {
        self.failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// Stops the handshake clock until the guard is dropped.
    pub(crate) fn pause_for_user(&self) -> PauseGuard<'_> {
        self.deadline_pause.pause()
    }

    pub(crate) fn deadline_pause(&self) -> &DeadlinePause {
        &self.deadline_pause
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure() -> HostKeyFailure {
        HostKeyFailure::UnknownRejectedByPolicy {
            presented_fingerprint: "SHA256:abc".to_owned(),
        }
    }

    #[test]
    fn should_hand_over_the_failure_once_when_it_was_recorded() {
        let state = HandshakeState::default();
        state.record_failure(failure());
        assert!(state.take_failure().is_some());
        assert!(state.take_failure().is_none());
    }
}
