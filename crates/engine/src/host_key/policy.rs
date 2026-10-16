use ssh_config::StrictHostKeyChecking;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnknownHostAction {
    Reject,
    Ask,
    AcceptAndSave,
}

pub(crate) fn unknown_host_action(
    strict_host_key_checking: StrictHostKeyChecking,
    is_batch_mode: bool,
) -> UnknownHostAction {
    match strict_host_key_checking {
        StrictHostKeyChecking::Yes => UnknownHostAction::Reject,
        StrictHostKeyChecking::Ask if is_batch_mode => UnknownHostAction::Reject,
        StrictHostKeyChecking::Ask => UnknownHostAction::Ask,
        StrictHostKeyChecking::AcceptNew | StrictHostKeyChecking::No => {
            UnknownHostAction::AcceptAndSave
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_reject_when_strict_checking_is_yes() {
        assert_eq!(
            unknown_host_action(StrictHostKeyChecking::Yes, false),
            UnknownHostAction::Reject
        );
    }

    #[test]
    fn should_ask_when_strict_checking_is_ask() {
        assert_eq!(
            unknown_host_action(StrictHostKeyChecking::Ask, false),
            UnknownHostAction::Ask
        );
    }

    #[test]
    fn should_reject_when_strict_checking_is_ask_in_batch_mode() {
        assert_eq!(
            unknown_host_action(StrictHostKeyChecking::Ask, true),
            UnknownHostAction::Reject
        );
    }

    #[test]
    fn should_accept_and_save_when_strict_checking_is_accept_new() {
        assert_eq!(
            unknown_host_action(StrictHostKeyChecking::AcceptNew, true),
            UnknownHostAction::AcceptAndSave
        );
    }

    #[test]
    fn should_accept_and_save_when_strict_checking_is_no() {
        assert_eq!(
            unknown_host_action(StrictHostKeyChecking::No, false),
            UnknownHostAction::AcceptAndSave
        );
    }
}
