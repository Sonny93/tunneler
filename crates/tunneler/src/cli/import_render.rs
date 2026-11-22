//! How the plan and the result of `tunneler import` are shown: one line per
//! entry of the file, then a summary. Every decision is a symbol plus a word,
//! and every risk and warning is a labelled sentence, so nothing depends on
//! color.

use ipc::protocol::{
    ConfigImportEntryDto, ConfigImportEntryWarningDto, ConfigImportPlanDto, ConfigImportReportDto,
    ConfigImportRiskDto, ConfigImportStatusDto, SnapshotDto,
};
use serde::Serialize;

use super::render::{describe_spec_route, describe_tags, describe_tunnel_route};
use super::style::{Style, Tone};

const EMPTY_FILE_MESSAGE: &str = "The file holds no tunnels.";
/// What is said when nothing was added.
pub const NOTHING_IMPORTED_MESSAGE: &str = "Nothing was imported.";
const UNKNOWN_TUNNEL_LABEL: &str = "a saved tunnel";
const NOT_SELECTED_HINT: &str = "Entries marked \"not selected\" are left out: pass --include-risky to import them too, edit the file to remove what makes them risky, or import them from the Tunneler window.";
const START_HINT: &str =
    "The imported tunnels are stopped: start one with `tunneler up <port>` or `tunneler up <tag>`.";
const HOST_MAP_PLACEHOLDER: &str = "ALIAS";

/// What a run does with the entries the plan marks as risky.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskPolicy {
    /// Only the entries the plan selects by default: risky ones are left out.
    LeaveOut,
    /// Every importable entry, risky or not (`--include-risky`).
    Include,
}

/// The plan and the indexes this run selects, as one line of JSON.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanJson<'a> {
    plan: &'a ConfigImportPlanDto,
    selected: &'a [u32],
}

/// The plan, the indexes this run selects and what applying it did.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportJson<'a> {
    plan: &'a ConfigImportPlanDto,
    selected: &'a [u32],
    report: &'a ConfigImportReportDto,
}

/// The plan and the indexes this run selects, as one line of JSON.
pub fn render_import_plan_json(
    plan: &ConfigImportPlanDto,
    policy: RiskPolicy,
) -> Result<String, serde_json::Error> {
    let selected = selected_indexes(plan, policy);
    serde_json::to_string(&PlanJson {
        plan,
        selected: &selected,
    })
}

/// The plan, the indexes this run selects and the report as one line of JSON.
pub fn render_import_report_json(
    plan: &ConfigImportPlanDto,
    policy: RiskPolicy,
    report: &ConfigImportReportDto,
) -> Result<String, serde_json::Error> {
    let selected = selected_indexes(plan, policy);
    serde_json::to_string(&ImportJson {
        plan,
        selected: &selected,
        report,
    })
}

fn is_selected(entry: &ConfigImportEntryDto, policy: RiskPolicy) -> bool {
    match policy {
        RiskPolicy::LeaveOut => entry.selected_by_default,
        RiskPolicy::Include => entry.status == ConfigImportStatusDto::Importable,
    }
}

fn is_risky(entry: &ConfigImportEntryDto) -> bool {
    !entry.risks.is_empty()
}

/// The entries this run selects, as the daemon numbers them.
pub fn selected_indexes(plan: &ConfigImportPlanDto, policy: RiskPolicy) -> Vec<u32> {
    plan.entries
        .iter()
        .filter(|entry| is_selected(entry, policy))
        .map(|entry| entry.index)
        .collect()
}

/// How many of the entries this run selects carry a risk.
pub fn count_selected_risky(plan: &ConfigImportPlanDto, policy: RiskPolicy) -> usize {
    plan.entries
        .iter()
        .filter(|entry| is_selected(entry, policy) && is_risky(entry))
        .count()
}

/// `1 tunnel` or `3 tunnels`.
pub fn count_tunnels(count: usize) -> String {
    match count {
        1 => "1 tunnel".to_owned(),
        _ => format!("{count} tunnels"),
    }
}

/// One line per entry, then the summary, the hosts the `ssh_config` does not
/// name and the hints that apply.
pub fn render_import_plan(
    plan: &ConfigImportPlanDto,
    snapshot: &SnapshotDto,
    style: Style,
    policy: RiskPolicy,
) -> String {
    if plan.entries.is_empty() {
        return EMPTY_FILE_MESSAGE.to_owned();
    }
    let mut lines: Vec<String> = plan
        .entries
        .iter()
        .map(|entry| render_entry(entry, plan, snapshot, style, policy))
        .collect();
    lines.push(render_summary(plan, policy));
    lines.extend(render_unknown_hosts(plan));
    lines.extend(render_not_selected_hint(plan, policy));
    lines.join("\n")
}

/// What applying did: the tunnels added, then the entries that were left out.
pub fn render_import_report(
    report: &ConfigImportReportDto,
    plan: &ConfigImportPlanDto,
    snapshot: &SnapshotDto,
    style: Style,
) -> String {
    let mut lines = Vec::new();
    if report.added.is_empty() {
        lines.push(NOTHING_IMPORTED_MESSAGE.to_owned());
    } else {
        lines.push(format!("Imported {}.", count_tunnels(report.added.len())));
        lines.extend(report.added.iter().map(|added| {
            format!(
                "  {}",
                describe_entry_route(added.index, plan, snapshot, style)
            )
        }));
    }
    lines.extend(report.skipped.iter().map(|skipped| {
        let label = describe_entry_route(skipped.index, plan, snapshot, style);
        let reason = describe_skip(&skipped.status, plan, snapshot, style);
        format!("Skipped {label}: {reason}")
    }));
    if !report.added.is_empty() {
        lines.push(START_HINT.to_owned());
    }
    lines.join("\n")
}

fn render_entry(
    entry: &ConfigImportEntryDto,
    plan: &ConfigImportPlanDto,
    snapshot: &SnapshotDto,
    style: Style,
    policy: RiskPolicy,
) -> String {
    let position = entry.index.saturating_add(1);
    let mut parts = vec![
        format!("{position}. {}", describe_decision(entry, policy, style)),
        describe_spec_route(&entry.tunnel, &snapshot.settings, style),
    ];
    parts.extend(describe_tags(&entry.tunnel.tags));
    parts.extend(describe_reason(&entry.status, plan, snapshot, style));
    parts.extend(
        entry
            .risks
            .iter()
            .map(|risk| style.paint(Tone::Warning, &format!("risk: {}", describe_risk(risk)))),
    );
    parts.extend(entry.warnings.iter().map(|warning| {
        style.paint(
            Tone::Warning,
            &format!("warning: {}", describe_warning(warning)),
        )
    }));
    parts.join("  ")
}

fn describe_decision(entry: &ConfigImportEntryDto, policy: RiskPolicy, style: Style) -> String {
    let (unicode_symbol, ascii_symbol, words, tone) = match &entry.status {
        ConfigImportStatusDto::Importable if is_selected(entry, policy) && is_risky(entry) => {
            ("!", "[!]", "will import (risky)", Tone::Warning)
        }
        ConfigImportStatusDto::Importable if is_selected(entry, policy) => {
            ("✔", "[ok]", "will import", Tone::Good)
        }
        ConfigImportStatusDto::Importable => ("!", "[!]", "not selected", Tone::Warning),
        ConfigImportStatusDto::Invalid { .. } => ("✖", "[x]", "invalid", Tone::Bad),
        _ => ("○", "[ ]", "skipped", Tone::Muted),
    };
    let symbol = if style.unicode {
        unicode_symbol
    } else {
        ascii_symbol
    };
    style.paint(tone, &format!("{symbol} {words}"))
}

fn describe_reason(
    status: &ConfigImportStatusDto,
    plan: &ConfigImportPlanDto,
    snapshot: &SnapshotDto,
    style: Style,
) -> Option<String> {
    match status {
        ConfigImportStatusDto::Importable => None,
        other => Some(describe_skip(other, plan, snapshot, style)),
    }
}

fn describe_skip(
    status: &ConfigImportStatusDto,
    plan: &ConfigImportPlanDto,
    snapshot: &SnapshotDto,
    style: Style,
) -> String {
    match status {
        ConfigImportStatusDto::Importable => "it can be imported".to_owned(),
        ConfigImportStatusDto::AlreadyConfigured { ignored_tags, .. } => {
            describe_already_configured(ignored_tags)
        }
        ConfigImportStatusDto::ListenConflict { holder_id } => format!(
            "the listening port is already used by {}",
            describe_holder(holder_id, snapshot, style)
        ),
        ConfigImportStatusDto::DuplicateInFile { earlier_index } => format!(
            "same route as tunnel {} of the file{}",
            earlier_index.saturating_add(1),
            describe_earlier_entry(*earlier_index, plan)
        ),
        ConfigImportStatusDto::Invalid { messages } => format!("invalid: {}", messages.join("; ")),
    }
}

fn describe_already_configured(ignored_tags: &[String]) -> String {
    if ignored_tags.is_empty() {
        return "already configured".to_owned();
    }
    format!(
        "already configured (tags not applied: {})",
        ignored_tags.join(", ")
    )
}

fn describe_earlier_entry(earlier_index: u32, plan: &ConfigImportPlanDto) -> String {
    let is_listed = plan
        .entries
        .iter()
        .any(|entry| entry.index == earlier_index);
    if is_listed {
        String::new()
    } else {
        " (not shown)".to_owned()
    }
}

fn describe_holder(holder_id: &str, snapshot: &SnapshotDto, style: Style) -> String {
    let label = snapshot
        .tunnels
        .iter()
        .find(|tunnel| tunnel.definition.id == holder_id)
        .map(|tunnel| describe_tunnel_route(tunnel, &snapshot.settings, style));
    match label {
        Some(label) => format!("the saved tunnel {label}"),
        None => UNKNOWN_TUNNEL_LABEL.to_owned(),
    }
}

fn describe_risk(risk: &ConfigImportRiskDto) -> String {
    match risk {
        ConfigImportRiskDto::NonLoopbackListen { address } => {
            format!("listens on {address}, which is not loopback: other machines can reach it")
        }
        ConfigImportRiskDto::RemoteNonLoopbackTarget { target_host } => {
            format!("forwards to {target_host}, which is not loopback")
        }
        ConfigImportRiskDto::StartsOnLaunch { tags } => format!(
            "starts when Tunneler launches, through the {}: {}",
            if tags.len() == 1 { "tag" } else { "tags" },
            tags.join(", ")
        ),
    }
}

fn describe_warning(warning: &ConfigImportEntryWarningDto) -> String {
    match warning {
        ConfigImportEntryWarningDto::UnknownHost { host } => {
            format!("no Host entry of your ssh_config names \"{host}\" exactly")
        }
        ConfigImportEntryWarningDto::OpenerDropped { opener } => {
            format!("the opener \"{opener}\" is not available here and was dropped")
        }
    }
}

fn describe_entry_route(
    index: u32,
    plan: &ConfigImportPlanDto,
    snapshot: &SnapshotDto,
    style: Style,
) -> String {
    match plan.entries.iter().find(|entry| entry.index == index) {
        Some(entry) => describe_spec_route(&entry.tunnel, &snapshot.settings, style),
        None => format!("tunnel {} of the file", index.saturating_add(1)),
    }
}

fn render_summary(plan: &ConfigImportPlanDto, policy: RiskPolicy) -> String {
    let count_where = |predicate: &dyn Fn(&ConfigImportEntryDto) -> bool| {
        plan.entries.iter().filter(|entry| predicate(entry)).count()
    };
    let to_import = count_where(&|entry| is_selected(entry, policy));
    let risky = count_selected_risky(plan, policy);
    let not_selected = count_where(&|entry| {
        entry.status == ConfigImportStatusDto::Importable && !is_selected(entry, policy)
    });
    let invalid =
        count_where(&|entry| matches!(entry.status, ConfigImportStatusDto::Invalid { .. }));
    let skipped = plan.entries.len() - to_import - not_selected - invalid;
    let to_import_label = if risky == 0 {
        format!("{to_import} to import")
    } else {
        format!("{to_import} to import ({risky} risky)")
    };
    format!(
        "{} in the file: {to_import_label}, {not_selected} not selected, {skipped} skipped, {invalid} invalid",
        count_tunnels(plan.entries.len())
    )
}

fn render_unknown_hosts(plan: &ConfigImportPlanDto) -> Vec<String> {
    let Some(first) = plan.unknown_hosts.first() else {
        return Vec::new();
    };
    vec![
        format!(
            "Hosts that no Host entry of your ssh_config names exactly: {}",
            plan.unknown_hosts.join(", ")
        ),
        format!(
            "Use --host-map OLD=NEW to go through one of your aliases instead, for example --host-map {first}={HOST_MAP_PLACEHOLDER}"
        ),
    ]
}

fn render_not_selected_hint(plan: &ConfigImportPlanDto, policy: RiskPolicy) -> Option<String> {
    if policy == RiskPolicy::Include {
        return None;
    }
    plan.entries
        .iter()
        .any(is_risky)
        .then(|| NOT_SELECTED_HINT.to_owned())
}

#[cfg(test)]
mod tests {
    use ipc::protocol::{
        ConfigImportAddedDto, ConfigImportSkippedDto, ConfigImportSourceDto, DefaultOpenerDto,
        DevslotStalePolicyDto, LanguageDto, MetricsDto, OpenDefaultDto, OpenHostDto,
        OpenOptionsDto, SettingsDto, StandbyNotifyDto, StandbySoundDto, TargetHealthDto,
        TrayStatusDto, TunnelDefinitionDto, TunnelDto, TunnelKindDto, TunnelSpecDto,
        TunnelStateDto, X11ServerStartDto,
    };

    use super::*;

    const PLAIN: Style = Style {
        color: false,
        unicode: true,
    };
    const ASCII: Style = Style {
        color: false,
        unicode: false,
    };
    const HOLDER_ID: &str = "01J0000000000000000000000A";

    fn spec(listen_port: u16, tags: &[&str]) -> TunnelSpecDto {
        TunnelSpecDto {
            host: "nova".to_owned(),
            kind: TunnelKindDto::Local,
            listen_port,
            listen_address: None,
            target_host: None,
            target_port: None,
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            open: OpenOptionsDto::default(),
            standby: None,
        }
    }

    fn entry(index: u32, listen_port: u16, status: ConfigImportStatusDto) -> ConfigImportEntryDto {
        ConfigImportEntryDto {
            index,
            tunnel: spec(listen_port, &["dev"]),
            file_host: "nova".to_owned(),
            effective_listen_address: "127.0.0.1".to_owned(),
            selected_by_default: status == ConfigImportStatusDto::Importable,
            status,
            risks: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn plan_of(entries: Vec<ConfigImportEntryDto>) -> ConfigImportPlanDto {
        ConfigImportPlanDto {
            source: ConfigImportSourceDto::Export,
            entries,
            unknown_hosts: Vec::new(),
        }
    }

    fn settings() -> SettingsDto {
        SettingsDto {
            default_bind_address: "127.0.0.1".to_owned(),
            default_remote_bind_address: "localhost".to_owned(),
            keepalive_interval_ms: 15_000,
            keepalive_max: 3,
            session_idle_grace_ms: 30_000,
            reconnect_initial_delay_ms: 1000,
            reconnect_max_delay_ms: 60_000,
            restore_on_launch: true,
            start_on_launch_tags: Vec::new(),
            launch_at_login: false,
            use_os_keyring: false,
            x11_display: None,
            x11_server_start: X11ServerStartDto::Ask,
            open_host: OpenHostDto::Localhost,
            open_default: OpenDefaultDto::Suggested,
            standby: false,
            standby_idle_s: 300,
            standby_notify: StandbyNotifyDto::Visual,
            standby_sound: StandbySoundDto::Pluck,
            standby_volume: 50,
            language: LanguageDto::System,
            check_for_updates: true,
            devslot_persist: false,
            devslot_stale: DevslotStalePolicyDto::Ask,
        }
    }

    fn holder_tunnel() -> TunnelDto {
        TunnelDto {
            definition: TunnelDefinitionDto {
                id: HOLDER_ID.to_owned(),
                spec: spec(4444, &[]),
            },
            state: TunnelStateDto::Stopped,
            health: TargetHealthDto::Unknown,
            metrics: MetricsDto::default(),
            is_awake: false,
            is_temporary: false,
            devslot: None,
            default_opener: DefaultOpenerDto {
                id: "browser".to_owned(),
                name: "Browser".to_owned(),
            },
        }
    }

    fn snapshot_with_holder() -> SnapshotDto {
        SnapshotDto {
            tunnels: vec![holder_tunnel()],
            settings: settings(),
            tray: TrayStatusDto::default(),
            externals: Vec::new(),
            x11_anchors: Vec::new(),
            x11_launchers: Vec::new(),
            x11_host_displays: std::collections::BTreeMap::new(),
            openers: Vec::new(),
        }
    }

    fn render(plan: &ConfigImportPlanDto, style: Style) -> String {
        render_with_policy(plan, style, RiskPolicy::LeaveOut)
    }

    fn render_with_policy(plan: &ConfigImportPlanDto, style: Style, policy: RiskPolicy) -> String {
        render_import_plan(plan, &snapshot_with_holder(), style, policy)
    }

    #[test]
    fn should_say_the_file_holds_no_tunnels_when_the_plan_is_empty() {
        assert_eq!(render(&plan_of(Vec::new()), PLAIN), EMPTY_FILE_MESSAGE);
    }

    #[test]
    fn should_show_a_selected_entry_with_its_route_and_tags_when_it_will_be_imported() {
        let plan = plan_of(vec![entry(0, 3333, ConfigImportStatusDto::Importable)]);

        let first_line = render(&plan, PLAIN).lines().next().unwrap().to_owned();

        assert_eq!(first_line, "1. ✔ will import  3333 → nova:3333  [dev]");
    }

    #[test]
    fn should_use_ascii_symbols_when_unicode_is_off() {
        let plan = plan_of(vec![entry(0, 3333, ConfigImportStatusDto::Importable)]);

        let first_line = render(&plan, ASCII).lines().next().unwrap().to_owned();

        assert_eq!(first_line, "1. [ok] will import  3333 -> nova:3333  [dev]");
    }

    #[test]
    fn should_number_the_entries_from_one_as_the_file_reads() {
        let plan = plan_of(vec![entry(2, 3333, ConfigImportStatusDto::Importable)]);

        assert!(render(&plan, PLAIN).starts_with("3. "));
    }

    #[test]
    fn should_say_the_entry_is_already_configured_and_list_the_tags_not_applied() {
        let status = ConfigImportStatusDto::AlreadyConfigured {
            existing_id: HOLDER_ID.to_owned(),
            ignored_tags: vec!["mail".to_owned(), "ops".to_owned()],
        };
        let plan = plan_of(vec![entry(0, 3333, status)]);

        assert!(render(&plan, PLAIN).starts_with(
            "1. ○ skipped  3333 → nova:3333  [dev]  already configured (tags not applied: mail, ops)"
        ));
    }

    #[test]
    fn should_say_the_entry_is_already_configured_without_tags_when_none_is_left_out() {
        let status = ConfigImportStatusDto::AlreadyConfigured {
            existing_id: HOLDER_ID.to_owned(),
            ignored_tags: Vec::new(),
        };
        let plan = plan_of(vec![entry(0, 3333, status)]);

        assert!(
            render(&plan, PLAIN)
                .starts_with("1. ○ skipped  3333 → nova:3333  [dev]  already configured\n")
        );
    }

    #[test]
    fn should_name_the_saved_tunnel_that_holds_the_port_when_the_listen_address_conflicts() {
        let status = ConfigImportStatusDto::ListenConflict {
            holder_id: HOLDER_ID.to_owned(),
        };
        let plan = plan_of(vec![entry(0, 4444, status)]);

        assert!(render(&plan, PLAIN).starts_with(
            "1. ○ skipped  4444 → nova:4444  [dev]  the listening port is already used by the saved tunnel 4444 → nova:4444"
        ));
    }

    #[test]
    fn should_not_name_the_holder_when_the_saved_tunnel_is_gone() {
        let status = ConfigImportStatusDto::ListenConflict {
            holder_id: "01J0000000000000000000000Z".to_owned(),
        };
        let plan = plan_of(vec![entry(0, 4444, status)]);

        assert!(render(&plan, PLAIN).contains("already used by a saved tunnel"));
    }

    #[test]
    fn should_point_to_the_earlier_entry_when_the_route_repeats_in_the_file() {
        let status = ConfigImportStatusDto::DuplicateInFile { earlier_index: 0 };
        let plan = plan_of(vec![
            entry(0, 3333, ConfigImportStatusDto::Importable),
            entry(1, 3333, status),
        ]);

        assert!(
            render(&plan, PLAIN).contains(
                "2. ○ skipped  3333 → nova:3333  [dev]  same route as tunnel 1 of the file"
            )
        );
    }

    #[test]
    fn should_show_the_messages_when_the_entry_is_invalid() {
        let status = ConfigImportStatusDto::Invalid {
            messages: vec!["port 0 is not valid".to_owned(), "tag is empty".to_owned()],
        };
        let plan = plan_of(vec![entry(0, 3333, status)]);

        assert!(render(&plan, PLAIN).starts_with(
            "1. ✖ invalid  3333 → nova:3333  [dev]  invalid: port 0 is not valid; tag is empty"
        ));
    }

    fn risky_entry(risk: ConfigImportRiskDto) -> ConfigImportEntryDto {
        ConfigImportEntryDto {
            risks: vec![risk],
            selected_by_default: false,
            ..entry(0, 3333, ConfigImportStatusDto::Importable)
        }
    }

    #[test]
    fn should_mark_a_non_loopback_listen_as_not_selected_with_its_risk_in_words() {
        let plan = plan_of(vec![risky_entry(ConfigImportRiskDto::NonLoopbackListen {
            address: "0.0.0.0".to_owned(),
        })]);

        assert!(render(&plan, PLAIN).starts_with(
            "1. ! not selected  3333 → nova:3333  [dev]  risk: listens on 0.0.0.0, which is not loopback: other machines can reach it"
        ));
    }

    #[test]
    fn should_describe_a_remote_target_that_is_not_loopback() {
        let plan = plan_of(vec![risky_entry(
            ConfigImportRiskDto::RemoteNonLoopbackTarget {
                target_host: "db.internal".to_owned(),
            },
        )]);

        assert!(
            render(&plan, PLAIN).contains("risk: forwards to db.internal, which is not loopback")
        );
    }

    #[test]
    fn should_describe_the_tags_that_start_the_tunnel_when_tunneler_launches() {
        let plan = plan_of(vec![risky_entry(ConfigImportRiskDto::StartsOnLaunch {
            tags: vec!["dev".to_owned(), "ops".to_owned()],
        })]);

        assert!(
            render(&plan, PLAIN)
                .contains("risk: starts when Tunneler launches, through the tags: dev, ops")
        );
    }

    #[test]
    fn should_tell_how_to_import_the_entries_that_are_not_selected() {
        let plan = plan_of(vec![risky_entry(ConfigImportRiskDto::StartsOnLaunch {
            tags: vec!["dev".to_owned()],
        })]);

        assert!(render(&plan, PLAIN).contains(
            "pass --include-risky to import them too, edit the file to remove what makes them risky"
        ));
    }

    #[test]
    fn should_not_show_the_not_selected_hint_when_risky_entries_are_included() {
        let plan = plan_of(vec![risky_entry(ConfigImportRiskDto::StartsOnLaunch {
            tags: vec!["dev".to_owned()],
        })]);

        let text = render_with_policy(&plan, PLAIN, RiskPolicy::Include);

        assert!(!text.contains("not selected\" are left out"));
    }

    #[test]
    fn should_mark_a_risky_entry_as_will_import_risky_with_its_risk_when_risky_entries_are_included()
     {
        let plan = plan_of(vec![risky_entry(ConfigImportRiskDto::NonLoopbackListen {
            address: "0.0.0.0".to_owned(),
        })]);

        assert!(render_with_policy(&plan, PLAIN, RiskPolicy::Include).starts_with(
            "1. ! will import (risky)  3333 → nova:3333  [dev]  risk: listens on 0.0.0.0, which is not loopback: other machines can reach it"
        ));
    }

    #[test]
    fn should_use_ascii_symbols_for_a_risky_selected_entry_when_unicode_is_off() {
        let plan = plan_of(vec![risky_entry(ConfigImportRiskDto::StartsOnLaunch {
            tags: vec!["dev".to_owned()],
        })]);

        assert!(
            render_with_policy(&plan, ASCII, RiskPolicy::Include)
                .starts_with("1. [!] will import (risky)  3333 -> nova:3333")
        );
    }

    #[test]
    fn should_count_the_risky_entries_in_the_summary_when_they_are_included() {
        let plan = plan_of(vec![
            entry(0, 1, ConfigImportStatusDto::Importable),
            risky_entry(ConfigImportRiskDto::StartsOnLaunch { tags: vec![] }),
            entry(
                2,
                3,
                ConfigImportStatusDto::DuplicateInFile { earlier_index: 0 },
            ),
        ]);

        assert!(
            render_with_policy(&plan, PLAIN, RiskPolicy::Include).contains(
                "3 tunnels in the file: 2 to import (1 risky), 0 not selected, 1 skipped, 0 invalid"
            )
        );
    }

    #[test]
    fn should_show_the_warnings_as_labelled_text() {
        let mut planned = entry(0, 3333, ConfigImportStatusDto::Importable);
        planned.warnings = vec![
            ConfigImportEntryWarningDto::UnknownHost {
                host: "nova".to_owned(),
            },
            ConfigImportEntryWarningDto::OpenerDropped {
                opener: "vlc".to_owned(),
            },
        ];

        assert!(render(&plan_of(vec![planned]), PLAIN).starts_with(
            "1. ✔ will import  3333 → nova:3333  [dev]  warning: no Host entry of your ssh_config names \"nova\" exactly  warning: the opener \"vlc\" is not available here and was dropped"
        ));
    }

    #[test]
    fn should_summarize_every_kind_of_entry_in_one_line() {
        let plan = plan_of(vec![
            entry(0, 1, ConfigImportStatusDto::Importable),
            risky_entry(ConfigImportRiskDto::StartsOnLaunch { tags: vec![] }),
            entry(
                2,
                3,
                ConfigImportStatusDto::DuplicateInFile { earlier_index: 0 },
            ),
            entry(
                3,
                4,
                ConfigImportStatusDto::Invalid {
                    messages: vec!["x".to_owned()],
                },
            ),
        ]);

        assert!(
            render(&plan, PLAIN).contains(
                "4 tunnels in the file: 1 to import, 1 not selected, 1 skipped, 1 invalid"
            )
        );
    }

    #[test]
    fn should_use_the_singular_in_the_summary_when_the_file_holds_one_tunnel() {
        let plan = plan_of(vec![entry(0, 1, ConfigImportStatusDto::Importable)]);

        assert!(render(&plan, PLAIN).contains("1 tunnel in the file: 1 to import"));
    }

    #[test]
    fn should_list_the_unknown_hosts_with_the_host_map_hint() {
        let mut plan = plan_of(vec![entry(0, 1, ConfigImportStatusDto::Importable)]);
        plan.unknown_hosts = vec!["oldbox".to_owned(), "other".to_owned()];

        let text = render(&plan, PLAIN);

        assert!(text.contains("names exactly: oldbox, other"));
        assert!(text.contains("--host-map OLD=NEW"));
        assert!(text.ends_with("for example --host-map oldbox=ALIAS"));
    }

    #[test]
    fn should_not_use_escape_sequences_when_color_is_off() {
        let plan = plan_of(vec![risky_entry(ConfigImportRiskDto::StartsOnLaunch {
            tags: vec!["dev".to_owned()],
        })]);

        assert!(!render(&plan, PLAIN).contains('\x1b'));
    }

    #[test]
    fn should_paint_the_decision_when_color_is_on() {
        let plan = plan_of(vec![entry(0, 1, ConfigImportStatusDto::Importable)]);
        let colored = Style {
            color: true,
            unicode: true,
        };

        assert!(render(&plan, colored).contains("\x1b[32m✔ will import\x1b[0m"));
    }

    fn plan_with_a_risky_entry_and_unselectable_ones() -> ConfigImportPlanDto {
        let risky = ConfigImportEntryDto {
            index: 1,
            ..risky_entry(ConfigImportRiskDto::StartsOnLaunch { tags: vec![] })
        };
        plan_of(vec![
            entry(0, 1, ConfigImportStatusDto::Importable),
            risky,
            entry(
                2,
                3,
                ConfigImportStatusDto::DuplicateInFile { earlier_index: 0 },
            ),
            entry(
                3,
                4,
                ConfigImportStatusDto::Invalid {
                    messages: vec!["x".to_owned()],
                },
            ),
            entry(4, 5, ConfigImportStatusDto::Importable),
        ])
    }

    #[test]
    fn should_leave_the_risky_entries_out_when_the_policy_is_leave_out() {
        let plan = plan_with_a_risky_entry_and_unselectable_ones();

        assert_eq!(selected_indexes(&plan, RiskPolicy::LeaveOut), vec![0, 4]);
    }

    #[test]
    fn should_select_the_risky_importable_entries_but_never_skipped_or_invalid_ones_when_the_policy_is_include()
     {
        let plan = plan_with_a_risky_entry_and_unselectable_ones();

        assert_eq!(selected_indexes(&plan, RiskPolicy::Include), vec![0, 1, 4]);
    }

    #[test]
    fn should_count_no_risky_entry_when_the_policy_is_leave_out() {
        let plan = plan_with_a_risky_entry_and_unselectable_ones();

        assert_eq!(
            (
                count_selected_risky(&plan, RiskPolicy::LeaveOut),
                count_selected_risky(&plan, RiskPolicy::Include)
            ),
            (0, 1)
        );
    }

    #[test]
    fn should_report_the_imported_tunnels_and_say_they_are_stopped() {
        let plan = plan_of(vec![entry(0, 3333, ConfigImportStatusDto::Importable)]);
        let report = ConfigImportReportDto {
            added: vec![ConfigImportAddedDto {
                index: 0,
                id: HOLDER_ID.to_owned(),
            }],
            skipped: Vec::new(),
        };

        let text = render_import_report(&report, &plan, &snapshot_with_holder(), PLAIN);

        assert_eq!(
            text,
            format!("Imported 1 tunnel.\n  3333 → nova:3333\n{START_HINT}")
        );
    }

    #[test]
    fn should_report_the_entries_that_changed_since_the_plan() {
        let plan = plan_of(vec![entry(0, 3333, ConfigImportStatusDto::Importable)]);
        let report = ConfigImportReportDto {
            added: Vec::new(),
            skipped: vec![ConfigImportSkippedDto {
                index: 0,
                status: ConfigImportStatusDto::ListenConflict {
                    holder_id: HOLDER_ID.to_owned(),
                },
            }],
        };

        let text = render_import_report(&report, &plan, &snapshot_with_holder(), PLAIN);

        assert_eq!(
            text,
            "Nothing was imported.\nSkipped 3333 → nova:3333: the listening port is already used by the saved tunnel 4444 → nova:4444"
        );
    }

    #[test]
    fn should_print_the_plan_as_one_line_of_json() {
        let plan = plan_of(vec![entry(0, 3333, ConfigImportStatusDto::Importable)]);

        let json = render_import_plan_json(&plan, RiskPolicy::LeaveOut).unwrap();

        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            (
                json.contains('\n'),
                value["plan"]["source"].clone(),
                value["plan"]["entries"][0]["status"]["kind"].clone()
            ),
            (
                false,
                serde_json::json!("export"),
                serde_json::json!("importable")
            )
        );
    }

    #[test]
    fn should_print_the_selected_indexes_next_to_the_plan_in_the_plan_json() {
        let plan = plan_with_a_risky_entry_and_unselectable_ones();

        let left_out = render_import_plan_json(&plan, RiskPolicy::LeaveOut).unwrap();
        let included = render_import_plan_json(&plan, RiskPolicy::Include).unwrap();

        let selected_of = |json: &str| {
            serde_json::from_str::<serde_json::Value>(json).unwrap()["selected"].clone()
        };
        assert_eq!(
            (selected_of(&left_out), selected_of(&included)),
            (serde_json::json!([0, 4]), serde_json::json!([0, 1, 4]))
        );
    }

    #[test]
    fn should_print_the_plan_the_selected_indexes_and_the_report_under_their_names() {
        let plan = plan_with_a_risky_entry_and_unselectable_ones();
        let report = ConfigImportReportDto {
            added: Vec::new(),
            skipped: Vec::new(),
        };

        let json = render_import_report_json(&plan, RiskPolicy::Include, &report).unwrap();

        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            (
                value["plan"]["entries"].as_array().unwrap().len(),
                value["selected"].clone(),
                value["report"]["added"].clone()
            ),
            (5, serde_json::json!([0, 1, 4]), serde_json::json!([]))
        );
    }
}
