use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::error::HostKeyFailure;

/// What the russh handler shares with `connect`: the reason a host key was
/// refused, and whether the handshake is currently waiting for the user.
#[derive(Debug, Default)]
pub(crate) struct HandshakeState {
    failure: Mutex<Option<HostKeyFailure>>,
    is_awaiting_user: AtomicBool,
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

    pub(crate) fn begin_awaiting_user(&self) {
        self.is_awaiting_user.store(true, Ordering::SeqCst);
    }

    pub(crate) fn end_awaiting_user(&self) {
        self.is_awaiting_user.store(false, Ordering::SeqCst);
    }

    pub(crate) fn is_awaiting_user(&self) -> bool {
        self.is_awaiting_user.load(Ordering::SeqCst)
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

    #[test]
    fn should_not_be_awaiting_the_user_when_nothing_was_signaled() {
        assert!(!HandshakeState::default().is_awaiting_user());
    }

    #[test]
    fn should_be_awaiting_the_user_when_waiting_began() {
        let state = HandshakeState::default();
        state.begin_awaiting_user();
        assert!(state.is_awaiting_user());
    }

    #[test]
    fn should_not_be_awaiting_the_user_when_waiting_ended() {
        let state = HandshakeState::default();
        state.begin_awaiting_user();
        state.end_awaiting_user();
        assert!(!state.is_awaiting_user());
    }
}
