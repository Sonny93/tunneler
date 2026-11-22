use std::collections::BTreeMap;

use ipc::protocol::{
    ConfigImportApplyParams, ConfigImportParams, ConfigImportPlanDto, ErrorCode, SnapshotDto,
};
use ipc::{ClientError, IpcClient};

use super::super::args::{HostMapEntry, ImportArguments};
use super::super::import_render::{
    NOTHING_IMPORTED_MESSAGE, count_tunnels, render_import_plan, render_import_plan_json,
    render_import_report, render_import_report_json, selected_indexes,
};
use super::super::import_source::read_import_source;
use super::super::launcher::ensure_daemon_running;
use super::super::session::{ConnectionMode, connect};
use super::super::style::Style;
use super::super::terminal::{ControllingTerminal, Terminal, can_ask_on_controlling_terminal};
use super::super::{CliError, Context};
use crate::logging;

const NOTHING_TO_IMPORT_MESSAGE: &str = "Nothing to import.";

/// What `import` does once the plan is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    /// `--dry-run`: show the plan and stop.
    DryRun,
    /// The plan selects nothing: show it and stop.
    NothingSelected,
    /// `--yes`: apply what the plan selects.
    Apply,
    /// Show the plan, then ask on the terminal.
    Ask,
    /// Show the plan, and fail: it needs a confirmation that cannot be asked.
    Refuse,
}

fn decide(arguments: &ImportArguments, selected_count: usize, can_ask: bool) -> Decision {
    if arguments.dry_run {
        return Decision::DryRun;
    }
    if selected_count == 0 {
        return Decision::NothingSelected;
    }
    if arguments.yes {
        return Decision::Apply;
    }
    if can_ask && !arguments.json {
        return Decision::Ask;
    }
    Decision::Refuse
}

/// The file once read, the plan the daemon made of it, and what is needed to
/// show it and to apply it.
struct ImportRun<'a> {
    client: &'a IpcClient,
    contents: String,
    host_map: BTreeMap<String, String>,
    plan: ConfigImportPlanDto,
    snapshot: SnapshotDto,
    style: Style,
    is_json: bool,
}

/// `tunneler import`: plans the import of a file, then adds the tunnels the
/// plan selects, starting the daemon first when needed.
pub async fn import(context: &Context, arguments: ImportArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    let contents = read_import_source(&arguments.file)?;
    let host_map = build_host_map(&arguments.host_maps)?;
    ensure_daemon_running(&context.endpoint, &context.locations).await?;
    let client = connect(&context.endpoint, ConnectionMode::default()).await?;
    let plan = client
        .preview_config_import(ConfigImportParams {
            contents: contents.clone(),
            host_map: host_map.clone(),
        })
        .await
        .map_err(describe_import_failure)?;
    let snapshot = client.snapshot().await?;
    let run = ImportRun {
        client: &client,
        contents,
        host_map,
        plan,
        snapshot,
        style: context.style,
        is_json: arguments.json,
    };
    let selected_count = selected_indexes(&run.plan).len();
    let can_ask = can_ask_on_controlling_terminal();
    match decide(&arguments, selected_count, can_ask) {
        Decision::DryRun => run.show_plan(),
        Decision::NothingSelected => run.show_nothing_selected(),
        Decision::Apply => run.show_plan_then_apply().await,
        Decision::Ask => run.ask_then_apply(&ControllingTerminal).await,
        Decision::Refuse => run.refuse(),
    }
}

impl ImportRun<'_> {
    fn show_plan(&self) -> Result<(), CliError> {
        if self.is_json {
            println!("{}", render_import_plan_json(&self.plan)?);
            return Ok(());
        }
        println!(
            "{}",
            render_import_plan(&self.plan, &self.snapshot, self.style)
        );
        Ok(())
    }

    fn show_nothing_selected(&self) -> Result<(), CliError> {
        self.show_plan()?;
        if !self.is_json {
            println!("{NOTHING_TO_IMPORT_MESSAGE}");
        }
        Ok(())
    }

    fn refuse(&self) -> Result<(), CliError> {
        self.show_plan()?;
        Err(CliError::ImportConfirmationRequired {
            count: count_tunnels(selected_indexes(&self.plan).len()),
        })
    }

    async fn ask_then_apply(&self, terminal: &dyn Terminal) -> Result<(), CliError> {
        self.show_plan()?;
        let selected_count = selected_indexes(&self.plan).len();
        if confirm_import(selected_count, terminal)? {
            return self.apply().await;
        }
        println!("{NOTHING_IMPORTED_MESSAGE}");
        Ok(())
    }

    async fn show_plan_then_apply(&self) -> Result<(), CliError> {
        if !self.is_json {
            self.show_plan()?;
        }
        self.apply().await
    }

    async fn apply(&self) -> Result<(), CliError> {
        let report = self
            .client
            .apply_config_import(ConfigImportApplyParams {
                contents: self.contents.clone(),
                host_map: self.host_map.clone(),
                indexes: selected_indexes(&self.plan),
            })
            .await
            .map_err(describe_import_failure)?;
        if self.is_json {
            println!("{}", render_import_report_json(&self.plan, &report)?);
            return Ok(());
        }
        println!(
            "{}",
            render_import_report(&report, &self.plan, &self.snapshot, self.style)
        );
        Ok(())
    }
}

fn confirm_import(selected_count: usize, terminal: &dyn Terminal) -> Result<bool, CliError> {
    let question = format!("Import {}? [y/N] ", count_tunnels(selected_count));
    let answer = terminal.read_line(&question).map_err(CliError::Terminal)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn build_host_map(entries: &[HostMapEntry]) -> Result<BTreeMap<String, String>, CliError> {
    let mut host_map = BTreeMap::new();
    for entry in entries {
        let previous = host_map.insert(entry.file_host.clone(), entry.local_alias.clone());
        if previous.is_some() {
            return Err(CliError::DuplicateHostMap {
                file_host: entry.file_host.clone(),
            });
        }
    }
    Ok(host_map)
}

fn describe_import_failure(error: ClientError) -> CliError {
    match error {
        ClientError::Remote {
            code: ErrorCode::InvalidImportFile,
            message,
            ..
        } => CliError::ImportRefused { message },
        other => CliError::Client(other),
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Mutex;

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

    fn arguments(dry_run: bool, yes: bool, json: bool) -> ImportArguments {
        ImportArguments {
            file: "backup.toml".to_owned(),
            host_maps: Vec::new(),
            dry_run,
            yes,
            json,
        }
    }

    #[test]
    fn should_only_show_the_plan_when_dry_run_is_given_even_with_yes() {
        assert_eq!(
            decide(&arguments(true, true, false), 2, true),
            Decision::DryRun
        );
    }

    #[test]
    fn should_stop_when_the_plan_selects_nothing_even_with_yes() {
        assert_eq!(
            decide(&arguments(false, true, false), 0, true),
            Decision::NothingSelected
        );
    }

    #[test]
    fn should_apply_without_asking_when_yes_is_given() {
        assert_eq!(
            decide(&arguments(false, true, false), 2, false),
            Decision::Apply
        );
    }

    #[test]
    fn should_ask_when_a_terminal_can_answer_and_yes_is_not_given() {
        assert_eq!(
            decide(&arguments(false, false, false), 2, true),
            Decision::Ask
        );
    }

    #[test]
    fn should_refuse_when_there_is_no_terminal_and_yes_is_not_given() {
        assert_eq!(
            decide(&arguments(false, false, false), 2, false),
            Decision::Refuse
        );
    }

    #[test]
    fn should_refuse_instead_of_asking_when_the_output_is_json() {
        assert_eq!(
            decide(&arguments(false, false, true), 2, true),
            Decision::Refuse
        );
    }

    #[test]
    fn should_ask_how_many_tunnels_to_import() {
        let terminal = ScriptedTerminal::answering("y");

        confirm_import(3, &terminal).unwrap();

        assert_eq!(
            terminal.asked.lock().unwrap().clone(),
            vec!["Import 3 tunnels? [y/N] ".to_owned()]
        );
    }

    #[test]
    fn should_use_the_singular_when_asking_to_import_one_tunnel() {
        let terminal = ScriptedTerminal::answering("y");

        confirm_import(1, &terminal).unwrap();

        assert_eq!(
            terminal.asked.lock().unwrap().clone(),
            vec!["Import 1 tunnel? [y/N] ".to_owned()]
        );
    }

    #[test]
    fn should_confirm_when_the_user_answers_yes_in_any_case() {
        let terminal = ScriptedTerminal::answering(" Yes ");

        assert!(confirm_import(2, &terminal).unwrap());
    }

    #[test]
    fn should_not_confirm_when_the_user_answers_nothing() {
        let terminal = ScriptedTerminal::answering("");

        assert!(!confirm_import(2, &terminal).unwrap());
    }

    #[test]
    fn should_not_confirm_when_the_user_answers_no() {
        let terminal = ScriptedTerminal::answering("n");

        assert!(!confirm_import(2, &terminal).unwrap());
    }

    fn entry(file_host: &str, local_alias: &str) -> HostMapEntry {
        HostMapEntry {
            file_host: file_host.to_owned(),
            local_alias: local_alias.to_owned(),
        }
    }

    #[test]
    fn should_collect_the_host_map_entries_by_host_of_the_file() {
        let host_map = build_host_map(&[entry("oldbox", "nova"), entry("other", "vega")]).unwrap();

        assert_eq!(
            host_map,
            BTreeMap::from([
                ("oldbox".to_owned(), "nova".to_owned()),
                ("other".to_owned(), "vega".to_owned())
            ])
        );
    }

    #[test]
    fn should_refuse_a_host_mapped_twice() {
        let outcome = build_host_map(&[entry("oldbox", "nova"), entry("oldbox", "vega")]);

        assert!(matches!(
            outcome,
            Err(CliError::DuplicateHostMap { file_host }) if file_host == "oldbox"
        ));
    }

    #[test]
    fn should_turn_a_refused_file_into_an_import_error_carrying_the_daemon_message() {
        let error = describe_import_failure(ClientError::Remote {
            code: ErrorCode::InvalidImportFile,
            message: "the file is not valid".to_owned(),
            data: None,
        });

        assert!(matches!(
            error,
            CliError::ImportRefused { message } if message == "the file is not valid"
        ));
    }

    #[test]
    fn should_keep_the_other_daemon_errors_as_they_are() {
        let error = describe_import_failure(ClientError::Remote {
            code: ErrorCode::InvalidState,
            message: "the tunnels file on disk is invalid".to_owned(),
            data: None,
        });

        assert!(matches!(error, CliError::Client(_)));
    }
}
