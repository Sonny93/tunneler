use std::io;
use std::path::PathBuf;
use std::time::Duration;

use ipc::protocol::ErrorCode;
use ipc::{ClientError, EndpointError};
use thiserror::Error;

use super::exit_code;
use super::forward_spec::ForwardSpecError;
use super::target::ResolveError;
use crate::daemon::DaemonError;
use crate::layout::LayoutError;
use crate::menu_entry::MenuEntryError;

const BYTES_PER_KIB: usize = 1024;
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
    /// The default location of the program's files could not be determined.
    #[error(transparent)]
    Layout(#[from] LayoutError),
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
    /// Removing files needs a confirmation that cannot be asked.
    #[error("purge removes files and needs a confirmation")]
    PurgeConfirmationRequired,
    /// Some of the items to remove could not be removed.
    #[error("{failed} of {total} items could not be removed")]
    PurgeIncomplete {
        /// How many removals failed.
        failed: usize,
        /// How many removals were attempted.
        total: usize,
    },
    /// The application menu entry could not be installed or removed.
    #[error(transparent)]
    MenuEntry(#[from] MenuEntryError),
    /// The running daemon could not launch the UI.
    #[error("{message}")]
    UiUnavailable {
        /// What the daemon reported.
        message: String,
    },
    /// The daemon refused to open the tunnel in a browser.
    #[error("the tunnel {label} cannot be opened: {reason}")]
    NotOpenable {
        /// The tunnel route.
        label: String,
        /// Why, as the daemon said it.
        reason: String,
    },
    /// The tunnel did not become ready or the browser could not be launched.
    #[error("cannot open {label}: {message}")]
    OpenFailed {
        /// The tunnel route.
        label: String,
        /// What the daemon reported.
        message: String,
    },
    /// The application asked for with `--with` does not exist here.
    #[error("{message}")]
    UnknownOpener {
        /// What the daemon reported: the available ids included.
        message: String,
    },
    /// The host lists no instance with this name.
    #[error("host {host} does not list the instance \"{instance}\"")]
    DevslotInstanceNotFound {
        /// The host alias.
        host: String,
        /// The instance that was asked for.
        instance: String,
    },
    /// The alias is not a `Host` of the `ssh_config`.
    #[error("{message}")]
    UnknownHost {
        /// What the daemon reported.
        message: String,
    },
    /// A command run on the host did not end in time.
    #[error("{message}")]
    RemoteCommandTimedOut {
        /// What the daemon reported.
        message: String,
    },
    /// Some web ports of a devslot instance were left out or failed.
    #[error("{count} of {total} web ports of {instance} are not tunneled")]
    PortsNotTunneled {
        /// The instance that was opened.
        instance: String,
        /// How many ports failed or were skipped.
        count: usize,
        /// How many web ports the instance has.
        total: usize,
    },
    /// The file to import cannot be read.
    #[error("cannot read {source_name}: {source}")]
    ImportUnreadable {
        /// The file, or `the standard input`.
        source_name: String,
        /// Why.
        #[source]
        source: io::Error,
    },
    /// The file to import is larger than the daemon accepts.
    #[error("{source_name} is larger than the limit of {} KiB", .limit_bytes / BYTES_PER_KIB)]
    ImportTooLarge {
        /// The file, or `the standard input`.
        source_name: String,
        /// The most bytes an import file may hold.
        limit_bytes: usize,
    },
    /// The file to import is not text.
    #[error("{source_name} is not valid UTF-8 text")]
    ImportNotText {
        /// The file, or `the standard input`.
        source_name: String,
    },
    /// The file to import holds control characters.
    #[error("{source_name} holds control characters")]
    ImportHasControlCharacters {
        /// The file, or `the standard input`.
        source_name: String,
    },
    /// The daemon refused to export: the file would exceed the import limit.
    #[error("{message}")]
    ExportTooLarge {
        /// What the daemon said.
        message: String,
    },
    /// The same host is mapped twice by `--host-map`.
    #[error("--host-map names \"{file_host}\" twice")]
    DuplicateHostMap {
        /// The host of the file.
        file_host: String,
    },
    /// The daemon refused the file as a whole.
    #[error("{message}")]
    ImportRefused {
        /// What the daemon said, with the reason.
        message: String,
    },
    /// An import needs a confirmation that cannot be asked.
    #[error("{count} to import: the import needs a confirmation")]
    ImportConfirmationRequired {
        /// How many tunnels the plan selects, as text.
        count: String,
    },
    /// The file to write exists and `--force` was not given.
    #[error("{} already exists", .path.display())]
    OutputExists {
        /// The file.
        path: PathBuf,
    },
    /// The file to write cannot be written.
    #[error("cannot write {}: {source}", .path.display())]
    OutputWrite {
        /// The file.
        path: PathBuf,
        /// Why.
        #[source]
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
            Self::ForwardSpec(_)
            | Self::ForwardNeedsOneHost { .. }
            | Self::DuplicateHostMap { .. }
            | Self::MenuEntry(
                MenuEntryError::Unsupported | MenuEntryError::PackageManaged { .. },
            ) => exit_code::USAGE,
            Self::TargetsFailed { .. } | Self::PortsNotTunneled { .. } => exit_code::TARGETS_FAILED,
            Self::TargetUnreachable { .. } => exit_code::TARGET_UNREACHABLE,
            Self::OpenFailed { .. } => exit_code::OPEN_FAILED,
            Self::UnknownOpener { .. } => exit_code::UNKNOWN_OPENER,
            Self::ConfirmationRequired { .. }
            | Self::PurgeConfirmationRequired
            | Self::ImportConfirmationRequired { .. } => exit_code::CONFIRMATION_REQUIRED,
            Self::TimedOutWaiting { .. }
            | Self::DaemonStopTimedOut { .. }
            | Self::RemoteCommandTimedOut { .. } => exit_code::TIMED_OUT,
            Self::DaemonSpawn(_)
            | Self::DaemonExited { .. }
            | Self::DaemonStartTimedOut { .. }
            | Self::Client(_)
            | Self::Daemon(_)
            | Self::Resolve(_)
            | Self::Endpoint(_)
            | Self::Layout(_)
            | Self::ConnectionLost
            | Self::NotOpenable { .. }
            | Self::TargetCheckInconclusive { .. }
            | Self::PurgeIncomplete { .. }
            | Self::MenuEntry(_)
            | Self::UiUnavailable { .. }
            | Self::DevslotInstanceNotFound { .. }
            | Self::UnknownHost { .. }
            | Self::ImportUnreadable { .. }
            | Self::ImportTooLarge { .. }
            | Self::ImportNotText { .. }
            | Self::ImportHasControlCharacters { .. }
            | Self::ImportRefused { .. }
            | Self::ExportTooLarge { .. }
            | Self::OutputExists { .. }
            | Self::OutputWrite { .. }
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
            Self::PurgeConfirmationRequired => Some(
                "run it in a terminal to be asked, pass --yes to remove without asking, or --dry-run to only list"
                    .to_owned(),
            ),
            Self::ImportConfirmationRequired { .. }
            | Self::ImportTooLarge { .. }
            | Self::ImportNotText { .. }
            | Self::ImportHasControlCharacters { .. }
            | Self::ExportTooLarge { .. }
            | Self::DuplicateHostMap { .. }
            | Self::ImportRefused { .. }
            | Self::OutputExists { .. } => import_hint(self),
            Self::PurgeIncomplete { .. } => Some(
                "the errors above say which item failed and why: fix the cause and run `tunneler purge` again"
                    .to_owned(),
            ),
            Self::OpenFailed { .. } => Some(
                "see `tunneler status` for the tunnel state, or open its address in your browser yourself"
                    .to_owned(),
            ),
            Self::NotOpenable { .. } => Some("run `tunneler list` to see the kinds".to_owned()),
            Self::UnknownOpener { .. } => Some(
                "use `browser` or one of the ids above, or add an application under [[openers]] in tunnels.toml"
                    .to_owned(),
            ),
            Self::UiUnavailable { .. } => Some(UI_UNAVAILABLE_HINT.to_owned()),
            Self::DevslotInstanceNotFound { .. } => Some(
                "the instance is the name that `ds ls` prints on the host, such as web-shop-098a"
                    .to_owned(),
            ),
            Self::UnknownHost { .. } => {
                Some("use a Host alias of your ssh_config, as `ssh <alias>` does".to_owned())
            }
            Self::RemoteCommandTimedOut { .. } => Some(
                "run `ds ls --json` on the host to see why it is slow, then try again".to_owned(),
            ),
            Self::PortsNotTunneled { .. } => Some(
                "the lines above say why; `tunneler status` shows the tunnels that are up"
                    .to_owned(),
            ),
            Self::MenuEntry(error) => menu_entry_hint(error),
            Self::TargetUnreachable { .. } => {
                Some("see `tunneler status` for the tunnel state".to_owned())
            }
            Self::DaemonSpawn(_)
            | Self::TargetCheckInconclusive { .. }
            | Self::Daemon(_)
            | Self::Endpoint(_)
            | Self::Layout(_)
            | Self::Client(_)
            | Self::ImportUnreadable { .. }
            | Self::OutputWrite { .. }
            | Self::Terminal(_)
            | Self::Encoding(_)
            | Self::Runtime(_) => None,
        }
    }
}

fn import_hint(error: &CliError) -> Option<String> {
    let hint = match error {
        CliError::ImportConfirmationRequired { .. } => {
            "run it in a terminal to be asked, pass --yes to import what the plan selects, or --dry-run to only see the plan"
        }
        CliError::ImportTooLarge { .. } => {
            "an export file is a few KiB: check that this is the right file"
        }
        CliError::ImportNotText { .. } | CliError::ImportHasControlCharacters { .. } => {
            "pass a file written by `tunneler export`, or a tunnels.toml"
        }
        CliError::DuplicateHostMap { .. } => {
            "give each host of the file once, as --host-map OLD=NEW"
        }
        CliError::ImportRefused { .. } => {
            "pass a file written by `tunneler export`, or a tunnels.toml: see `tunneler import --help`"
        }
        CliError::ExportTooLarge { .. } => {
            "pass --host or --tag to export a part of the tunnels, then the others in another file"
        }
        CliError::OutputExists { .. } => "pick another name, or pass --force to replace it",
        _ => return None,
    };
    Some(hint.to_owned())
}

fn menu_entry_hint(error: &MenuEntryError) -> Option<String> {
    match error {
        MenuEntryError::UiProgramMissing { .. } => Some(
            "run `tunneler desktop-entry install` from the folder that holds both programs"
                .to_owned(),
        ),
        MenuEntryError::ForeignEntry { .. } => Some(
            "move or delete that file yourself if it is not wanted, then run the command again"
                .to_owned(),
        ),
        MenuEntryError::PackageManaged { .. } => {
            Some("the package already installed the menu entry, nothing to do".to_owned())
        }
        _ => None,
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
    fn should_exit_with_4_when_ports_of_an_instance_are_not_tunneled() {
        assert_eq!(
            CliError::PortsNotTunneled {
                instance: "web-shop-098a".to_owned(),
                count: 1,
                total: 2
            }
            .exit_code(),
            exit_code::TARGETS_FAILED
        );
    }

    #[test]
    fn should_exit_with_6_when_a_command_on_the_host_timed_out() {
        assert_eq!(
            CliError::RemoteCommandTimedOut {
                message: "too slow".to_owned()
            }
            .exit_code(),
            exit_code::TIMED_OUT
        );
    }

    #[test]
    fn should_exit_with_1_when_the_host_does_not_list_the_instance() {
        assert_eq!(
            CliError::DevslotInstanceNotFound {
                host: "vega".to_owned(),
                instance: "web-shop-098a".to_owned()
            }
            .exit_code(),
            exit_code::ERROR
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
    fn should_exit_with_8_when_opening_failed() {
        assert_eq!(
            CliError::OpenFailed {
                label: "3333".to_owned(),
                message: "no browser".to_owned()
            }
            .exit_code(),
            exit_code::OPEN_FAILED
        );
    }

    #[test]
    fn should_exit_with_9_when_the_opener_is_unknown() {
        assert_eq!(
            CliError::UnknownOpener {
                message: "no opener".to_owned()
            }
            .exit_code(),
            exit_code::UNKNOWN_OPENER
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
    fn should_exit_with_5_when_purge_cannot_ask_for_a_confirmation() {
        assert_eq!(
            CliError::PurgeConfirmationRequired.exit_code(),
            exit_code::CONFIRMATION_REQUIRED
        );
    }

    #[test]
    fn should_suggest_the_yes_flag_when_purge_cannot_ask_for_a_confirmation() {
        let hint = CliError::PurgeConfirmationRequired.hint().unwrap();

        assert!(hint.contains("--yes"));
    }

    #[test]
    fn should_exit_with_1_when_some_removals_failed() {
        let error = CliError::PurgeIncomplete {
            failed: 1,
            total: 4,
        };

        assert_eq!(error.exit_code(), exit_code::ERROR);
        assert_eq!(error.to_string(), "1 of 4 items could not be removed");
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
    fn should_exit_with_5_and_suggest_the_yes_flag_when_an_import_cannot_ask_for_a_confirmation() {
        let error = CliError::ImportConfirmationRequired {
            count: "2 tunnels".to_owned(),
        };

        assert_eq!(error.exit_code(), exit_code::CONFIRMATION_REQUIRED);
        assert!(error.hint().unwrap().contains("--yes"));
    }

    #[test]
    fn should_exit_with_1_and_name_the_limit_when_the_file_to_import_is_too_large() {
        let error = CliError::ImportTooLarge {
            source_name: "backup.toml".to_owned(),
            limit_bytes: 262_144,
        };

        assert_eq!(error.exit_code(), exit_code::ERROR);
        assert_eq!(
            error.to_string(),
            "backup.toml is larger than the limit of 256 KiB"
        );
    }

    #[test]
    fn should_exit_with_2_when_a_host_is_mapped_twice() {
        assert_eq!(
            CliError::DuplicateHostMap {
                file_host: "oldbox".to_owned()
            }
            .exit_code(),
            exit_code::USAGE
        );
    }

    #[test]
    fn should_suggest_the_force_flag_when_the_output_file_exists() {
        let error = CliError::OutputExists {
            path: PathBuf::from("backup.toml"),
        };

        assert_eq!(
            (error.exit_code(), error.to_string(), error.hint().unwrap()),
            (
                exit_code::ERROR,
                "backup.toml already exists".to_owned(),
                "pick another name, or pass --force to replace it".to_owned()
            )
        );
    }

    #[test]
    fn should_exit_with_2_when_a_forward_goes_through_several_hosts() {
        assert_eq!(
            CliError::ForwardNeedsOneHost { count: 2 }.exit_code(),
            exit_code::USAGE
        );
    }

    #[test]
    fn should_exit_with_1_and_suggest_listing_the_tunnels_when_a_target_is_unknown() {
        let error = CliError::Resolve(ResolveError::UnknownTag {
            tag: "ghost".to_owned(),
        });

        assert_eq!(error.exit_code(), exit_code::ERROR);
        assert!(error.hint().unwrap().contains("tunneler list"));
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
