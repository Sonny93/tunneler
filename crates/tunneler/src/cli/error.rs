use std::io;
use std::time::Duration;

use engine::StoreError;
use ipc::protocol::ErrorCode;
use ipc::{ClientError, EndpointError};
use thiserror::Error;

use super::exit_code;
use super::forward_spec::ForwardSpecError;
use super::target::ResolveError;
use crate::daemon::DaemonError;

const UI_UNAVAILABLE_HINT: &str = "the UI program `tunneler-ui` must sit next to `tunneler`, be on your PATH, or be set with TUNNELER_UI_BINARY";

/// Why a command failed. Each variant has an exit code and, usually, a hint on
/// what to do next.
#[derive(Debug, Error)]
pub enum CliError {
    /// The command needs the daemon and nothing answers.
    #[error("the Tunneler daemon is not running")]
    DaemonNotRunning,
    /// The spawned daemon could not even be started.
    #[error("cannot start the daemon: {0}")]
    DaemonSpawn(#[source] io::Error),
    /// The spawned daemon ended right away.
    #[error("the daemon stopped right after starting ({status})")]
    DaemonExited {
        /// How it ended, ready to be shown.
        status: String,
    },
    /// The spawned daemon did not answer in time.
    #[error("the daemon did not answer within {} seconds", .waited.as_secs())]
    DaemonStartTimedOut {
        /// How long it was waited for.
        waited: Duration,
    },
    /// The daemon did not stop in time after `quit`.
    #[error("the daemon is still running {} seconds after being asked to quit", .waited.as_secs())]
    DaemonStopTimedOut {
        /// How long it was waited for.
        waited: Duration,
    },
    /// A call to the daemon failed.
    #[error(transparent)]
    Client(#[from] ClientError),
    /// The daemon part of the program failed.
    #[error(transparent)]
    Daemon(#[from] DaemonError),
    /// An argument on the command line designates no tunnel, or too many.
    #[error(transparent)]
    Resolve(#[from] ResolveError),
    /// A `-L`, `-R` or `-D` specification cannot become a tunnel.
    #[error(transparent)]
    ForwardSpec(#[from] ForwardSpecError),
    /// A forward was given with something other than one host.
    #[error("a forward goes through exactly one host, but {count} were given")]
    ForwardNeedsOneHost {
        /// How many targets were given.
        count: usize,
    },
    /// The endpoint could not be determined.
    #[error(transparent)]
    Endpoint(#[from] EndpointError),
    /// The default tunnels file could not be located.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The tunnels did not all become active in time.
    #[error("timed out after {} seconds waiting for: {}", .waited.as_secs(), .waiting_for.join(", "))]
    TimedOutWaiting {
        /// How long it was waited for.
        waited: Duration,
        /// The labels of the tunnels that were still starting.
        waiting_for: Vec<String>,
    },
    /// Some tunnels failed to start.
    #[error("{failed} of {total} tunnels failed to start")]
    TargetsFailed {
        /// How many failed.
        failed: usize,
        /// How many were started.
        total: usize,
    },
    /// The target of a tunnel did not answer the check.
    #[error("the target of {label} is unreachable: {error}")]
    TargetUnreachable {
        /// The tunnel route.
        label: String,
        /// Why the target did not answer.
        error: String,
    },
    /// The tunnel stopped before the check could tell anything.
    #[error("the check of {label} was inconclusive: the tunnel stopped meanwhile")]
    TargetCheckInconclusive {
        /// The tunnel route.
        label: String,
    },
    /// The daemon closed the connection while the command was waiting.
    #[error("the daemon closed the connection")]
    ConnectionLost,
    /// Stopping requires a confirmation that cannot be asked.
    #[error("{running} tunnels or X11 sessions are active and stopping them needs a confirmation")]
    ConfirmationRequired {
        /// How many tunnels run.
        running: usize,
    },
    /// The running daemon could not launch the UI.
    #[error("{message}")]
    UiUnavailable {
        /// What the daemon reported.
        message: String,
    },
    /// The tunnel cannot be opened in a browser.
    #[error("the tunnel {label} is a {kind} tunnel: there is nothing to open on this machine")]
    NotOpenable {
        /// The tunnel route.
        label: String,
        /// Its kind.
        kind: &'static str,
    },
    /// The browser could not be launched.
    #[error("cannot open {url}: {source}")]
    OpenFailed {
        /// What was to be opened.
        url: String,
        /// Why it failed.
        source: io::Error,
    },
    /// Reading the answer of the user failed.
    #[error("cannot read the answer: {0}")]
    Terminal(#[source] io::Error),
    /// The output could not be encoded.
    #[error("cannot encode the output: {0}")]
    Encoding(#[from] serde_json::Error),
    /// The async runtime could not start.
    #[error("cannot start the runtime: {0}")]
    Runtime(#[source] io::Error),
}

impl CliError {
    /// The exit code of the process.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::DaemonNotRunning | Self::Client(ClientError::DaemonNotRunning) => {
                exit_code::DAEMON_NOT_RUNNING
            }
            Self::ForwardSpec(_) | Self::ForwardNeedsOneHost { .. } => exit_code::USAGE,
            Self::TargetsFailed { .. } => exit_code::TARGETS_FAILED,
            Self::TargetUnreachable { .. } => exit_code::TARGET_UNREACHABLE,
            Self::ConfirmationRequired { .. } => exit_code::CONFIRMATION_REQUIRED,
            Self::TimedOutWaiting { .. } | Self::DaemonStopTimedOut { .. } => exit_code::TIMED_OUT,
            Self::DaemonSpawn(_)
            | Self::DaemonExited { .. }
            | Self::DaemonStartTimedOut { .. }
            | Self::Client(_)
            | Self::Daemon(_)
            | Self::Resolve(_)
            | Self::Endpoint(_)
            | Self::Store(_)
            | Self::ConnectionLost
            | Self::NotOpenable { .. }
            | Self::OpenFailed { .. }
            | Self::TargetCheckInconclusive { .. }
            | Self::UiUnavailable { .. }
            | Self::Terminal(_)
            | Self::Encoding(_)
            | Self::Runtime(_) => exit_code::ERROR,
        }
    }

    /// What the user can do about it, when there is something.
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::DaemonNotRunning | Self::Client(ClientError::DaemonNotRunning) => {
                Some("start it with `tunneler daemon` or run `tunneler up <tag>`".to_owned())
            }
            Self::DaemonExited { .. } | Self::DaemonStartTimedOut { .. } => {
                Some("run `tunneler daemon` in a terminal to see why it does not start".to_owned())
            }
            Self::DaemonStopTimedOut { .. } => {
                Some("check with `tunneler status`, or stop the process by hand".to_owned())
            }
            Self::Client(ClientError::ProtocolMismatch { .. }) => Some(
                "the running daemon is from another version: stop it, then run this command again"
                    .to_owned(),
            ),
            Self::Client(ClientError::ConnectionClosed) | Self::ConnectionLost => {
                Some("check the daemon with `tunneler status`".to_owned())
            }
            Self::Client(ClientError::Remote { code, .. }) => remote_hint(*code),
            Self::Daemon(DaemonError::AlreadyRunning) => Some(
                "see what it does with `tunneler status`, or stop it with `tunneler quit`"
                    .to_owned(),
            ),
            Self::Resolve(error) => Some(error.hint()),
            Self::ForwardSpec(_) | Self::ForwardNeedsOneHost { .. } => {
                Some("see `tunneler up --help` for the forward syntax".to_owned())
            }
            Self::TimedOutWaiting { .. } => Some(
                "raise --timeout, or look at `tunneler status` to see what they are doing"
                    .to_owned(),
            ),
            Self::TargetsFailed { .. } => Some("see `tunneler status` for the reasons".to_owned()),
            Self::ConfirmationRequired { .. } => Some(
                "run it in a terminal to be asked, or pass --yes to stop them and quit".to_owned(),
            ),
            Self::OpenFailed { url, .. } => Some(format!("open {url} in your browser yourself")),
            Self::NotOpenable { .. } => Some("run `tunneler list` to see the kinds".to_owned()),
            Self::UiUnavailable { .. } => Some(UI_UNAVAILABLE_HINT.to_owned()),
            Self::TargetUnreachable { .. } => {
                Some("see `tunneler status` for the tunnel state".to_owned())
            }
            Self::DaemonSpawn(_)
            | Self::TargetCheckInconclusive { .. }
            | Self::Daemon(_)
            | Self::Endpoint(_)
            | Self::Store(_)
            | Self::Client(_)
            | Self::Terminal(_)
            | Self::Encoding(_)
            | Self::Runtime(_) => None,
        }
    }
}

fn remote_hint(code: ErrorCode) -> Option<String> {
    match code {
        ErrorCode::NotFound => Some("run `tunneler list` to see the tunnels".to_owned()),
        ErrorCode::InvalidState => Some(
            "only a running local or remote tunnel can be checked: see `tunneler status`"
                .to_owned(),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn remote(code: ErrorCode) -> CliError {
        CliError::Client(ClientError::Remote {
            code,
            message: "boom".to_owned(),
            data: None,
        })
    }

    #[test]
    fn should_exit_with_3_when_the_daemon_is_not_running() {
        assert_eq!(
            [
                CliError::DaemonNotRunning.exit_code(),
                CliError::Client(ClientError::DaemonNotRunning).exit_code()
            ],
            [exit_code::DAEMON_NOT_RUNNING; 2]
        );
    }

    #[test]
    fn should_exit_with_4_when_targets_failed() {
        assert_eq!(
            CliError::TargetsFailed {
                failed: 1,
                total: 2
            }
            .exit_code(),
            exit_code::TARGETS_FAILED
        );
    }

    #[test]
    fn should_exit_with_7_when_the_target_is_unreachable() {
        assert_eq!(
            CliError::TargetUnreachable {
                label: "3333".to_owned(),
                error: "refused".to_owned()
            }
            .exit_code(),
            exit_code::TARGET_UNREACHABLE
        );
    }

    #[test]
    fn should_exit_with_5_when_a_confirmation_is_required() {
        assert_eq!(
            CliError::ConfirmationRequired { running: 2 }.exit_code(),
            exit_code::CONFIRMATION_REQUIRED
        );
    }

    #[test]
    fn should_exit_with_6_when_waiting_timed_out() {
        let error = CliError::TimedOutWaiting {
            waited: Duration::from_secs(30),
            waiting_for: vec!["3333 → nova:3333".to_owned()],
        };

        assert_eq!(error.exit_code(), exit_code::TIMED_OUT);
    }

    #[test]
    fn should_exit_with_2_when_a_forward_goes_through_several_hosts() {
        assert_eq!(
            CliError::ForwardNeedsOneHost { count: 2 }.exit_code(),
            exit_code::USAGE
        );
    }

    #[test]
    fn should_exit_with_1_when_a_target_is_unknown() {
        let error = CliError::Resolve(ResolveError::UnknownTag {
            tag: "ghost".to_owned(),
        });

        assert_eq!(error.exit_code(), exit_code::ERROR);
    }

    #[test]
    fn should_exit_with_1_when_the_daemon_is_already_running() {
        assert_eq!(
            CliError::Daemon(DaemonError::AlreadyRunning).exit_code(),
            exit_code::ERROR
        );
    }

    #[test]
    fn should_exit_with_1_when_the_daemon_answers_with_an_error() {
        assert_eq!(remote(ErrorCode::Internal).exit_code(), exit_code::ERROR);
    }

    #[test]
    fn should_suggest_starting_the_daemon_when_it_is_not_running() {
        let hint = CliError::DaemonNotRunning.hint().unwrap();

        assert_eq!(
            hint,
            "start it with `tunneler daemon` or run `tunneler up <tag>`"
        );
    }

    #[test]
    fn should_suggest_listing_the_tunnels_when_a_target_is_unknown() {
        let error = CliError::Resolve(ResolveError::UnknownTag {
            tag: "ghost".to_owned(),
        });

        assert!(error.hint().unwrap().contains("tunneler list"));
    }

    #[test]
    fn should_suggest_the_yes_flag_when_a_confirmation_is_required() {
        let hint = CliError::ConfirmationRequired { running: 2 }
            .hint()
            .unwrap();

        assert!(hint.contains("--yes"));
    }

    #[test]
    fn should_name_the_tunnels_still_starting_when_waiting_timed_out() {
        let error = CliError::TimedOutWaiting {
            waited: Duration::from_secs(30),
            waiting_for: vec!["3333 → nova:3333".to_owned(), "5432 → nova:5432".to_owned()],
        };

        assert_eq!(
            error.to_string(),
            "timed out after 30 seconds waiting for: 3333 → nova:3333, 5432 → nova:5432"
        );
    }

    #[test]
    fn should_exit_with_1_when_the_ui_cannot_be_launched() {
        let error = CliError::UiUnavailable {
            message: "boom".to_owned(),
        };

        assert_eq!(error.exit_code(), exit_code::ERROR);
    }

    #[test]
    fn should_say_where_the_ui_program_must_be_when_the_ui_cannot_be_launched() {
        let hint = CliError::UiUnavailable {
            message: "boom".to_owned(),
        }
        .hint()
        .unwrap();

        assert!(hint.contains("TUNNELER_UI_BINARY"));
    }

    #[test]
    fn should_carry_the_daemon_message_when_the_ui_cannot_be_launched() {
        let error = CliError::UiUnavailable {
            message: "cannot start the UI program `tunneler-ui`".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "cannot start the UI program `tunneler-ui`"
        );
    }

    #[test]
    fn should_have_no_hint_when_the_daemon_failed_internally() {
        assert_eq!(remote(ErrorCode::Internal).hint(), None);
    }
}
