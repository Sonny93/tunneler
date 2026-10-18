//! What the main window does once the daemon has answered `ui.claim`: the
//! window starts hidden so a second UI can leave without ever showing.

use ipc::ClientError;

/// What to do with the hidden main window at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupAction {
    /// Show and focus the window.
    ShowWindow,
    /// Another UI is already open and was brought to the front: exit.
    Exit,
}

/// Decides from the daemon's answer to `ui.claim`; `None` means no answer was
/// obtained. Only an explicit refusal exits: every other outcome (an older
/// daemon answering `unknown_method` included) shows the window, so the user
/// is never left with an invisible app.
pub fn startup_action(claim: Option<&Result<bool, ClientError>>) -> StartupAction {
    match claim {
        Some(Ok(false)) => StartupAction::Exit,
        Some(Ok(true) | Err(_)) | None => StartupAction::ShowWindow,
    }
}

#[cfg(test)]
mod tests {
    use ipc::protocol::ErrorCode;

    use super::*;

    #[test]
    fn should_show_the_window_when_the_claim_is_granted() {
        assert_eq!(startup_action(Some(&Ok(true))), StartupAction::ShowWindow);
    }

    #[test]
    fn should_exit_when_the_claim_is_refused() {
        assert_eq!(startup_action(Some(&Ok(false))), StartupAction::Exit);
    }

    #[test]
    fn should_show_the_window_when_the_daemon_does_not_know_the_method() {
        let claim = Err(ClientError::Remote {
            code: ErrorCode::UnknownMethod,
            message: "unknown method".to_owned(),
            data: None,
        });

        assert_eq!(startup_action(Some(&claim)), StartupAction::ShowWindow);
    }

    #[test]
    fn should_show_the_window_when_the_connection_closes_during_the_claim() {
        let claim = Err(ClientError::ConnectionClosed);

        assert_eq!(startup_action(Some(&claim)), StartupAction::ShowWindow);
    }

    #[test]
    fn should_show_the_window_when_no_answer_was_obtained() {
        assert_eq!(startup_action(None), StartupAction::ShowWindow);
    }
}
