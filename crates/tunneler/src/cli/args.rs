//! The command line, as `clap` reads it.

use clap::{Args, Parser, Subcommand};

use super::exit_code::describe_exit_codes;
use super::forward_spec::ForwardOption;
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
  tunneler up nova -L 5000      Temporary tunnel: local 5000 to port 5000 of nova
  tunneler up nova -L 5000:db.internal:5432
                                Temporary tunnel: local 5000 to db.internal:5432 via nova
  tunneler up nova -R 9000:localhost:3000
                                Temporary tunnel: nova:9000 to local port 3000
  tunneler up nova -D 1080      Temporary SOCKS proxy on local port 1080 via nova

Tunnels have no name. A number selects the tunnel listening on that port (when
several hosts have one, pass host:port), host:port selects a host's tunnel, and
anything else is a tag (ignoring case). The daemon is started in the background
when it is not running. When a password, a passphrase or an unknown host key
has to be confirmed, you are asked here if this is a terminal.

With -L, -R or -D the only argument is the ssh_config host, and the forward uses
the syntax of ssh: -L [bind_address:]port[:host:hostport], -R
[bind_address:]port[:host:hostport], -D [bind_address:]port. A bare port stands
for the same port on the host's loopback. When a saved tunnel already does that
forward, it is started instead. Otherwise a temporary tunnel is started: it is
never saved, and `tunneler down` (or stopping it in the app) removes it, as does
quitting the daemon.";

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
  tunneler open 3333            Open the tunnel listening on port 3333 in the browser
  tunneler open nova:3333       The same, picking the tunnel by host and port
  tunneler open adonis          The same, by a tag that only one tunnel carries
  tunneler open 5999 --with browser   Open a VNC tunnel in the browser instead of a viewer

A tunnel opens with its saved application (`open.with`), else the application
suggested for its target port (Remmina, a VNC or RDP viewer...), else the
browser. `--with` overrides it; an unknown id lists the available ones.
A stopped tunnel is started first. The address follows the tunnel's `open`
options (scheme, path, host) and the `open_host` setting.";

const CHECK_EXAMPLES: &str = "\
Examples:
  tunneler check 3333           Try the target of the tunnel listening on port 3333
  tunneler check nova:3333      The same, picking the tunnel by host and port
  tunneler check adonis         The same, by a tag that only one tunnel carries

The tunnel must be running, and be a local or a remote one: a dynamic tunnel has
no fixed target. The check opens one connection to the target (through the SSH
session for a local tunnel, from this machine for a remote one) and closes it
at once; it never runs on its own, and the result also updates the target health
shown by `tunneler status`. Some services count a connection that closes without
speaking their protocol as a failed login, so do not script it in a loop.

Exits with 0 when the target answers, and 7 when it does not.";

const QUIT_EXAMPLES: &str = "\
Examples:
  tunneler quit                 Stop the daemon; asks first when tunnels are active
  tunneler quit --yes           Stop the tunnels and the daemon without asking";

const PURGE_EXAMPLES: &str = "\
Examples:
  tunneler purge --dry-run      List what would be removed, remove nothing
  tunneler purge                List what will be removed, ask, then remove it
  tunneler purge --yes          Remove it without asking
  tunneler purge --keep-config  Keep tunnels.toml, state.toml and the stored passphrases

Stops the daemon when one runs, then removes: the launch-at-login entry, the log
files and the data of the UI's webview. Unless --keep-config is given, it also
removes tunnels.toml, state.toml and the passphrases stored in the OS keyring
(those listed in state.toml). Directories left empty are removed too.

It never removes the program files (tunneler, tunneler-ui, the `portable` marker
file), the install directory or anything under ~/.ssh. A file given with
--config or --state (or TUNNELER_CONFIG, TUNNELER_STATE) is never removed.

In portable mode the files are those next to the executable. Run it before
uninstalling a package that leaves the user's files alone, such as a .deb.
Without a terminal, --yes is required (exit code 5).";

const DAEMON_EXAMPLES: &str = "\
Examples:
  tunneler daemon               Run the daemon in the foreground
  TUNNELER_LOG=debug tunneler daemon

Logs go to daily files under the user data directory, or data/logs next to the
executable in portable mode (7 are kept), and to stderr when it is a terminal.
Normally the daemon is started for you by `tunneler up`.";

fn top_level_help() -> String {
    format!(
        "\
Examples:
  tunneler up dev               Start the tunnels tagged dev (starts the daemon if needed)
  tunneler status               Show what is running
  tunneler down --all           Stop every tunnel
  tunneler quit                 Stop the daemon
  tunneler purge --dry-run      List everything Tunneler left on this machine

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
    /// Start tunnels, by port, host:port or tag, or a temporary forward
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
    /// Open a tunnel in the browser, starting it when stopped
    #[command(after_help = OPEN_EXAMPLES)]
    Open(OpenArguments),
    /// Try the target of a running tunnel once
    #[command(after_help = CHECK_EXAMPLES)]
    Check(CheckArguments),
    /// Stop the daemon
    #[command(after_help = QUIT_EXAMPLES)]
    Quit(QuitArguments),
    /// Remove everything Tunneler left on this machine, except the programs
    #[command(after_help = PURGE_EXAMPLES)]
    Purge(PurgeArguments),
}

/// The arguments of `up`.
#[derive(Debug, Args)]
pub struct UpArguments {
    /// Read the targets as tags, never as ports or host:port pairs
    #[arg(long, conflicts_with_all = ["local_forward", "remote_forward", "dynamic_forward"])]
    pub tag: bool,
    /// Start a temporary local forward through the host, as ssh -L does
    #[arg(
        short = 'L',
        value_name = "SPEC",
        conflicts_with_all = ["remote_forward", "dynamic_forward"]
    )]
    pub local_forward: Option<String>,
    /// Start a temporary remote forward through the host, as ssh -R does
    #[arg(
        short = 'R',
        value_name = "SPEC",
        conflicts_with_all = ["local_forward", "dynamic_forward"]
    )]
    pub remote_forward: Option<String>,
    /// Start a temporary SOCKS proxy through the host, as ssh -D does
    #[arg(
        short = 'D',
        value_name = "SPEC",
        conflicts_with_all = ["local_forward", "remote_forward"]
    )]
    pub dynamic_forward: Option<String>,
    /// Return once the start is requested, without waiting for the tunnels
    #[arg(long)]
    pub no_wait: bool,
    /// How long to wait for the tunnels, in seconds
    #[arg(long, value_name = "SECONDS", default_value_t = DEFAULT_WAIT_TIMEOUT_SECONDS)]
    pub timeout: u64,
    /// The ports, host:port pairs or tags to start; with -L, -R or -D, the host
    #[arg(required = true, value_name = "TARGET")]
    pub targets: Vec<String>,
}

impl UpArguments {
    /// The forward asked for with `-L`, `-R` or `-D`, when there is one.
    pub fn forward(&self) -> Option<(ForwardOption, &str)> {
        [
            (ForwardOption::Local, &self.local_forward),
            (ForwardOption::Remote, &self.remote_forward),
            (ForwardOption::Dynamic, &self.dynamic_forward),
        ]
        .into_iter()
        .find_map(|(option, specification)| specification.as_deref().map(|text| (option, text)))
    }
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
    /// Open with `browser` or the id of an application instead of the default
    #[arg(long, value_name = "APP")]
    pub with: Option<String>,
}

/// The arguments of `check`.
#[derive(Debug, Args)]
pub struct CheckArguments {
    /// The port, host:port pair or tag of the tunnel to check
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

/// The arguments of `purge`.
#[derive(Debug, Args)]
pub struct PurgeArguments {
    /// Keep tunnels.toml, state.toml and the passphrases stored in the keyring
    #[arg(long)]
    pub keep_config: bool,
    /// List what would be removed and remove nothing
    #[arg(long)]
    pub dry_run: bool,
    /// Remove without asking for a confirmation
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
    fn should_default_the_timeout_to_thirty_seconds_and_have_no_forward_when_up_is_given_only_targets()
     {
        let Some(Command::Up(arguments)) = parse(&["up", "dev"]).unwrap().command else {
            panic!("expected up");
        };

        assert_eq!(arguments.timeout, 30);
        assert_eq!(arguments.forward(), None);
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
    fn should_read_the_forward_when_up_is_given_a_local_forward() {
        let Some(Command::Up(arguments)) = parse(&["up", "nova", "-L", "5000"]).unwrap().command
        else {
            panic!("expected up");
        };

        assert_eq!(arguments.forward(), Some((ForwardOption::Local, "5000")));
        assert_eq!(arguments.targets, ["nova".to_owned()]);
    }

    #[test]
    fn should_read_the_forward_when_up_is_given_a_remote_forward() {
        let Some(Command::Up(arguments)) = parse(&["up", "-R", "9000:localhost:3000", "nova"])
            .unwrap()
            .command
        else {
            panic!("expected up");
        };

        assert_eq!(
            arguments.forward(),
            Some((ForwardOption::Remote, "9000:localhost:3000"))
        );
    }

    #[test]
    fn should_read_the_forward_when_up_is_given_a_dynamic_forward() {
        let Some(Command::Up(arguments)) = parse(&["up", "nova", "-D", "1080"]).unwrap().command
        else {
            panic!("expected up");
        };

        assert_eq!(arguments.forward(), Some((ForwardOption::Dynamic, "1080")));
    }

    #[test]
    fn should_refuse_two_forwards_when_starting() {
        assert!(parse(&["up", "nova", "-L", "5000", "-D", "1080"]).is_err());
    }

    #[test]
    fn should_refuse_tag_mode_together_with_a_forward() {
        assert!(parse(&["up", "--tag", "nova", "-L", "5000"]).is_err());
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
    fn should_read_every_flag_when_purging() {
        let Some(Command::Purge(arguments)) = parse(&["purge", "--keep-config", "--dry-run", "-y"])
            .unwrap()
            .command
        else {
            panic!("expected purge");
        };

        assert!(arguments.keep_config && arguments.dry_run && arguments.yes);
    }

    #[test]
    fn should_default_every_flag_to_off_when_purging() {
        let Some(Command::Purge(arguments)) = parse(&["purge"]).unwrap().command else {
            panic!("expected purge");
        };

        assert!(!arguments.keep_config && !arguments.dry_run && !arguments.yes);
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
