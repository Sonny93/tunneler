//! `tunneler purge`: removes what Tunneler left on the machine, except the
//! programs themselves.

mod execution;
mod plan;
mod system;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use engine::{FileStateStore, StateStore, StoreError};

use self::execution::{DaemonControl, Purger};
use self::plan::{PurgeTarget, build_plan};
use self::system::{EndpointDaemon, SystemFilesystem};
use super::super::args::PurgeArguments;
use super::super::terminal::{StandardTerminal, Terminal, is_interactive};
use super::super::{CliError, Context};
use crate::daemon::login_launch::system_login_launch;
use crate::daemon::notification_identity::system_notification_identity;
use crate::daemon::os_keyring::OsKeyring;
use crate::layout::DataLayout;
use crate::logging;

/// `tunneler purge`: lists what it removes, asks, stops the daemon and removes.
pub async fn purge(context: &Context, arguments: &PurgeArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    let layout = DataLayout::resolve()?;
    let keyring_entries = if arguments.keep_config {
        BTreeSet::new()
    } else {
        read_keyring_entries(&layout.state_file)
    };
    let daemon = EndpointDaemon {
        endpoint: &context.endpoint,
    };
    let login_launch = system_login_launch();
    let notification_identity = system_notification_identity();
    let purger = Purger {
        daemon: &daemon,
        filesystem: &SystemFilesystem,
        login_launch: login_launch.as_ref(),
        notification_identity: notification_identity.as_ref(),
        keyring: &OsKeyring,
    };
    let plan = purger.retain_present(build_plan(
        &layout,
        arguments.keep_config,
        &keyring_entries,
        cfg!(windows),
    ));
    let is_daemon_running = daemon.is_running().await;
    print_summary(
        &plan,
        is_daemon_running,
        &overrides_left_alone(context, arguments),
        arguments.dry_run,
    );
    if arguments.dry_run {
        return Ok(());
    }
    if !confirm(arguments.yes, &StandardTerminal, is_interactive())? {
        return Err(CliError::PurgeConfirmationRequired);
    }
    let failures = purger.purge(&plan).await?;
    report(&failures, plan.len())
}

fn read_keyring_entries(state_file: &Path) -> BTreeSet<PathBuf> {
    let loaded: Result<_, StoreError> = FileStateStore::new(state_file.to_path_buf()).load();
    match loaded {
        Ok(state) => state.keyring_entries,
        Err(error) => {
            eprintln!(
                "warning: cannot read {}, so the passphrases it lists are not removed: {error}",
                state_file.display()
            );
            BTreeSet::new()
        }
    }
}

/// The files the user pointed at with `--config` and `--state` (or their
/// variables): they are not Tunneler's to remove.
fn overrides_left_alone(context: &Context, arguments: &PurgeArguments) -> Vec<PathBuf> {
    if arguments.keep_config {
        return Vec::new();
    }
    [&context.locations.config, &context.locations.state]
        .into_iter()
        .flatten()
        .cloned()
        .collect()
}

fn print_summary(
    plan: &[PurgeTarget],
    is_daemon_running: bool,
    overrides: &[PathBuf],
    is_dry_run: bool,
) {
    for line in describe_summary(plan, is_daemon_running, overrides, is_dry_run) {
        println!("{line}");
    }
}

fn describe_summary(
    plan: &[PurgeTarget],
    is_daemon_running: bool,
    overrides: &[PathBuf],
    is_dry_run: bool,
) -> Vec<String> {
    let header = if is_dry_run {
        "Would remove (dry run, nothing is removed):"
    } else {
        "Will remove:"
    };
    let mut lines = vec![header.to_owned()];
    if is_daemon_running {
        lines.push("  - stop the running daemon and its tunnels".to_owned());
    }
    lines.extend(
        plan.iter()
            .map(|target| format!("  - {}", target.describe())),
    );
    lines.extend(overrides.iter().map(|path| {
        format!(
            "Left alone, given with --config or --state: {}",
            path.display()
        )
    }));
    lines.push("The programs and ~/.ssh are never touched.".to_owned());
    lines
}

/// Asks whether to go on. Never asks without a terminal: then `--yes` is
/// required.
fn confirm(assume_yes: bool, terminal: &dyn Terminal, is_terminal: bool) -> Result<bool, CliError> {
    if assume_yes {
        return Ok(true);
    }
    if !is_terminal {
        return Err(CliError::PurgeConfirmationRequired);
    }
    let answer = terminal
        .read_line("Remove all of the above? [y/N] ")
        .map_err(CliError::Terminal)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn report(failures: &[execution::PurgeError], total: usize) -> Result<(), CliError> {
    for failure in failures {
        eprintln!("error: {failure}");
    }
    if !failures.is_empty() {
        return Err(CliError::PurgeIncomplete {
            failed: failures.len(),
            total,
        });
    }
    println!("Tunneler's files were removed.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Mutex;

    use pretty_assertions::assert_eq;
    use zeroize::Zeroizing;

    use super::*;

    struct ScriptedTerminal {
        answer: String,
        asked: Mutex<Vec<String>>,
    }

    impl ScriptedTerminal {
        fn answering(answer: &str) -> Self {
            Self {
                answer: answer.to_owned(),
                asked: Mutex::new(Vec::new()),
            }
        }
    }

    impl Terminal for ScriptedTerminal {
        fn write(&self, _text: &str) {}

        fn read_secret(&self, _prompt: &str) -> io::Result<Zeroizing<String>> {
            Err(io::Error::from(io::ErrorKind::Unsupported))
        }

        fn read_line(&self, prompt: &str) -> io::Result<String> {
            self.asked.lock().unwrap().push(prompt.to_owned());
            Ok(self.answer.clone())
        }
    }

    #[test]
    fn should_confirm_without_asking_when_yes_is_given() {
        let terminal = ScriptedTerminal::answering("n");

        assert!(confirm(true, &terminal, false).unwrap());
        assert!(terminal.asked.lock().unwrap().is_empty());
    }

    #[test]
    fn should_require_yes_when_there_is_no_terminal() {
        let terminal = ScriptedTerminal::answering("y");

        let outcome = confirm(false, &terminal, false);

        assert!(matches!(outcome, Err(CliError::PurgeConfirmationRequired)));
        assert!(terminal.asked.lock().unwrap().is_empty());
    }

    #[test]
    fn should_confirm_when_the_user_answers_yes() {
        assert!(confirm(false, &ScriptedTerminal::answering("Yes"), true).unwrap());
    }

    #[test]
    fn should_not_confirm_when_the_user_answers_nothing() {
        assert!(!confirm(false, &ScriptedTerminal::answering(""), true).unwrap());
    }

    #[test]
    fn should_list_the_daemon_the_targets_and_the_untouched_files_when_describing_the_summary() {
        let plan = [PurgeTarget::File("/config/tunnels.toml".into())];

        assert_eq!(
            describe_summary(&plan, true, &[], false),
            vec![
                "Will remove:",
                "  - stop the running daemon and its tunnels",
                "  - file /config/tunnels.toml",
                "The programs and ~/.ssh are never touched.",
            ]
        );
    }

    #[test]
    fn should_say_nothing_is_removed_when_describing_a_dry_run() {
        let lines = describe_summary(&[], false, &[], true);

        assert_eq!(lines[0], "Would remove (dry run, nothing is removed):");
    }

    #[test]
    fn should_name_the_files_left_alone_when_an_override_is_set() {
        let lines = describe_summary(&[], false, &["/tmp/mine.toml".into()], false);

        assert!(
            lines
                .contains(&"Left alone, given with --config or --state: /tmp/mine.toml".to_owned())
        );
    }
}
