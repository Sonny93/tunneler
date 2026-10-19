//! The command line, as `clap` reads it.

use clap::{Args, Parser, Subcommand};

use super::exit_code::describe_exit_codes;
use super::locations::Locations;
use super::style::ColorChoice;

/// How long `up` waits for the tunnels, in seconds, unless told otherwise.
pub const DEFAULT_WAIT_TIMEOUT_SECONDS: u64 = 30;

const UP_EXAMPLES: &str = "\
Examples:
  tunneler up 3333              Start the tunnel listening on port 3333
  tunneler up nova:3333         Start the tunnel of host nova listening on port 3333
  tunneler up dev               Start every tunnel tagged dev
  tunneler up --no-wait dev     Ask for the start and return at once
  tunneler up --timeout 60 dev  Wait up to a minute

Tunnels have no name. A number selects the tunnel listening on that port (when
several hosts have one, pass host:port), host:port selects a host's tunnel, and
anything else is a tag (ignoring case). The daemon is started in the background
when it is not running. When a password, a passphrase or an unknown host key
has to be confirmed, you are asked here if this is a terminal.";

const DOWN_EXAMPLES: &str = "\
Examples:
  tunneler down 3333            Stop the tunnel listening on port 3333
  tunneler down nova:3333       Stop the tunnel of host nova listening on port 3333
  tunneler down dev             Stop every tunnel tagged dev
  tunneler down --all           Stop every tunnel";

const STATUS_EXAMPLES: &str = "\
Examples:
  tunneler status               Show the daemon version and every tunnel with its state
  tunneler status --json        The same as one line of JSON, for scripts

With --json the output is {\"daemonVersion\": ..., \"tunnels\": [...], \"settings\": {...}, \"tray\": {...}}:
the tunnels, settings and tray status are the objects of the IPC protocol, with their
state, target health and traffic, in camelCase.";

const LIST_EXAMPLES: &str = "\
Examples:
  tunneler list                 Show the configured tunnels
  tunneler list --json          The same as one line of JSON, for scripts

With --json the output is {\"tunnels\": [...]}: the tunnel definitions of the IPC protocol,
in camelCase.";

const OPEN_EXAMPLES: &str = "\
Examples:
  tunneler open 3333            Open http://localhost:3333 in the browser
  tunneler open nova:3333       The same, picking the tunnel by host and port
  tunneler open adonis          The same, by a tag that only one tunnel carries";

const QUIT_EXAMPLES: &str = "\
Examples:
  tunneler quit                 Stop the daemon; asks first when tunnels are active
  tunneler quit --yes           Stop the tunnels and the daemon without asking";

const DAEMON_EXAMPLES: &str = "\
Examples:
  tunneler daemon               Run the daemon in the foreground
  TUNNELER_LOG=debug tunneler daemon

Logs go to daily files under the user data directory (7 are kept), and to stderr
when it is a terminal. Normally the daemon is started for you by `tunneler up`.";

fn top_level_help() -> String {
    format!(
        "\
Examples:
  tunneler up dev               Start the tunnels tagged dev (starts the daemon if needed)
  tunneler status               Show what is running
  tunneler down --all           Stop every tunnel
  tunneler quit                 Stop the daemon

Without a command, tunneler starts the daemon, or asks the running one to show
its window.

Exit codes:
{}

Environment:
  NO_COLOR             disable color
  TUNNELER_LOG         log filter for the daemon and the commands, such as debug
  TUNNELER_SOCKET      same as --socket
  TUNNELER_CONFIG      same as --config
  TUNNELER_STATE       same as --state
  TUNNELER_SSH_CONFIG  same as --ssh-config
  TUNNELER_NO_TRAY     same as --no-tray
  TUNNELER_UI_BINARY   the tunneler-ui program the daemon launches",
        describe_exit_codes()
    )
}

/// SSH tunnel manager: runs the daemon and controls it.
#[derive(Debug, Parser)]
#[command(name = "tunneler", version, after_help = top_level_help())]
pub struct Cli {
    /// Where the daemon and its files are
    #[command(flatten)]
    pub locations: Locations,
    /// When to color the output
    #[arg(long, global = true, value_enum, default_value_t, value_name = "WHEN")]
    pub color: ColorChoice,
    /// What to do; without one, start the daemon or show its window
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// What the command does.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the daemon in the foreground
    #[command(after_help = DAEMON_EXAMPLES)]
    Daemon,
    /// Start tunnels, by port, host:port or tag
    #[command(after_help = UP_EXAMPLES)]
    Up(UpArguments),
    /// Stop tunnels, by port, host:port or tag
    #[command(after_help = DOWN_EXAMPLES)]
    Down(DownArguments),
    /// Show the daemon and the state of every tunnel
    #[command(after_help = STATUS_EXAMPLES)]
    Status(OutputArguments),
    /// Show the configured tunnels
    #[command(after_help = LIST_EXAMPLES)]
    List(OutputArguments),
    /// Open a tunnel in the browser
    #[command(after_help = OPEN_EXAMPLES)]
    Open(OpenArguments),
    /// Stop the daemon
    #[command(after_help = QUIT_EXAMPLES)]
    Quit(QuitArguments),
}

/// The arguments of `up`.
#[derive(Debug, Args)]
pub struct UpArguments {
    /// Read the targets as tags, never as ports or host:port pairs
    #[arg(long)]
    pub tag: bool,
    /// Return once the start is requested, without waiting for the tunnels
    #[arg(long)]
    pub no_wait: bool,
    /// How long to wait for the tunnels, in seconds
    #[arg(long, value_name = "SECONDS", default_value_t = DEFAULT_WAIT_TIMEOUT_SECONDS)]
    pub timeout: u64,
    /// The ports, host:port pairs or tags to start
    #[arg(required = true, value_name = "TARGET")]
    pub targets: Vec<String>,
}

/// The arguments of `down`.
#[derive(Debug, Args)]
pub struct DownArguments {
    /// Read the targets as tags, never as ports or host:port pairs
    #[arg(long, conflicts_with = "all")]
    pub tag: bool,
    /// Stop every tunnel
    #[arg(long, conflicts_with = "targets")]
    pub all: bool,
    /// The ports, host:port pairs or tags to stop
    #[arg(required_unless_present = "all", value_name = "TARGET")]
    pub targets: Vec<String>,
}

/// The arguments of `status` and `list`.
#[derive(Debug, Args)]
pub struct OutputArguments {
    /// Print one line of JSON instead of text
    #[arg(long)]
    pub json: bool,
}

/// The arguments of `open`.
#[derive(Debug, Args)]
pub struct OpenArguments {
    /// The port, host:port pair or tag of the tunnel to open
    #[arg(value_name = "TARGET")]
    pub target: String,
}

/// The arguments of `quit`.
#[derive(Debug, Args)]
pub struct QuitArguments {
    /// Stop the active tunnels too, without asking
    #[arg(short, long)]
    pub yes: bool,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(arguments: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("tunneler").chain(arguments.iter().copied()))
    }

    #[test]
    fn should_define_a_consistent_command_line() {
        Cli::command().debug_assert();
    }

    #[test]
    fn should_have_no_command_when_none_is_given() {
        assert!(parse(&[]).unwrap().command.is_none());
    }

    #[test]
    fn should_default_the_timeout_to_thirty_seconds() {
        let Some(Command::Up(arguments)) = parse(&["up", "dev"]).unwrap().command else {
            panic!("expected up");
        };

        assert_eq!(arguments.timeout, 30);
    }

    #[test]
    fn should_require_a_target_when_starting() {
        assert!(parse(&["up"]).is_err());
    }

    #[test]
    fn should_accept_the_global_options_after_the_subcommand() {
        let cli = parse(&["status", "--socket", "/tmp/s", "--color", "never"]).unwrap();

        assert_eq!(
            (cli.locations.socket, cli.color),
            (Some("/tmp/s".into()), ColorChoice::Never)
        );
    }

    #[test]
    fn should_require_all_or_targets_when_stopping() {
        assert!(parse(&["down"]).is_err());
    }

    #[test]
    fn should_refuse_targets_together_with_all_when_stopping() {
        assert!(parse(&["down", "--all", "dev"]).is_err());
    }

    #[test]
    fn should_accept_all_alone_when_stopping() {
        assert!(parse(&["down", "--all"]).is_ok());
    }

    #[test]
    fn should_accept_the_short_yes_flag_when_quitting() {
        let Some(Command::Quit(arguments)) = parse(&["quit", "-y"]).unwrap().command else {
            panic!("expected quit");
        };

        assert!(arguments.yes);
    }

    #[test]
    fn should_reject_an_unknown_color_choice() {
        assert!(parse(&["--color", "sometimes", "status"]).is_err());
    }

    #[test]
    fn should_list_the_exit_codes_in_the_top_level_help() {
        let help = Cli::command().render_long_help().to_string();

        assert!(help.contains("Exit codes:") && help.contains("  3  the daemon is not running"));
    }

    #[test]
    fn should_give_examples_in_the_help_of_every_subcommand() {
        let command = Cli::command();

        assert!(command.get_subcommands().all(|subcommand| {
            subcommand
                .get_after_help()
                .is_some_and(|text| text.to_string().contains("Examples:"))
        }));
    }
}
