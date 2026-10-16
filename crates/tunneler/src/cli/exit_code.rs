//! The exit codes of the `tunneler` command, as documented in `--help`.

/// Everything went well.
pub const SUCCESS: u8 = 0;
/// Something failed; the message on stderr says what.
pub const ERROR: u8 = 1;
/// The command line is invalid. Returned by the argument parser itself.
pub const USAGE: u8 = 2;
/// The command needs the daemon and it is not running.
pub const DAEMON_NOT_RUNNING: u8 = 3;
/// `up`: at least one tunnel failed to start.
pub const TARGETS_FAILED: u8 = 4;
/// A confirmation is needed but cannot be asked: not a terminal and no `--yes`.
pub const CONFIRMATION_REQUIRED: u8 = 5;
/// The command timed out waiting for the daemon or the tunnels.
pub const TIMED_OUT: u8 = 6;

/// The exit codes in the form `--help` shows them.
pub fn describe_exit_codes() -> String {
    [
        (SUCCESS, "success"),
        (ERROR, "error"),
        (USAGE, "invalid usage"),
        (DAEMON_NOT_RUNNING, "the daemon is not running"),
        (TARGETS_FAILED, "some tunnels failed to start (up)"),
        (
            CONFIRMATION_REQUIRED,
            "confirmation required but not possible: not a terminal, pass --yes",
        ),
        (TIMED_OUT, "timed out waiting"),
    ]
    .map(|(code, meaning)| format!("  {code}  {meaning}"))
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_list_every_code_with_its_meaning_when_describing_exit_codes() {
        let description = describe_exit_codes();

        assert!(
            [
                SUCCESS,
                ERROR,
                USAGE,
                DAEMON_NOT_RUNNING,
                TARGETS_FAILED,
                CONFIRMATION_REQUIRED,
                TIMED_OUT
            ]
            .iter()
            .all(|code| description.contains(&format!("  {code}  ")))
        );
    }
}
