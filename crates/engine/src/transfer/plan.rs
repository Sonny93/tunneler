use std::collections::{BTreeMap, BTreeSet};

use super::classify::{RouteConflict, classify_definitions};
use super::parse::ParsedImportFile;
use super::plan_types::{
    ConfigImportEntry, ConfigImportEntryWarning, ConfigImportPlan, ConfigImportStatus,
};
use super::prepare::{PreparedEntry, prepare_entry};
use super::risk::find_risks;
use crate::store::{Settings, TunnelDefinition, find_ssh_host_problem};

/// What the entries of a file are compared with.
pub(crate) struct PlanContext<'context> {
    /// The saved tunnels.
    pub(crate) existing: &'context [TunnelDefinition],
    /// The settings of this machine.
    pub(crate) settings: &'context Settings,
    /// The aliases the `ssh_config` names, as `hosts.list` gives them.
    pub(crate) known_hosts: &'context [String],
    /// The ids of the openers this machine has: the browser is always there.
    pub(crate) opener_ids: &'context [String],
}

/// Reads `contents` and plans what importing it would do.
#[cfg(test)]
pub(crate) fn plan_import(
    contents: &str,
    host_map: &BTreeMap<String, String>,
    context: &PlanContext<'_>,
) -> Result<ConfigImportPlan, super::ImportFileError> {
    Ok(plan_parsed(
        super::parse::parse_import_file(contents)?,
        host_map,
        context,
    ))
}

/// Plans what importing an already parsed file would do.
///
/// Nothing is resolved against `ssh_config`: the known aliases are given.
pub(crate) fn plan_parsed(
    file: ParsedImportFile,
    host_map: &BTreeMap<String, String>,
    context: &PlanContext<'_>,
) -> ConfigImportPlan {
    let ParsedImportFile { source, tunnels } = file;
    let prepared: Vec<PreparedEntry> = tunnels
        .into_iter()
        .map(|tunnel| prepare_entry(tunnel, host_map, context.opener_ids))
        .collect();
    let statuses = classify_entries(&prepared, context);
    let unknown_hosts = find_unknown_hosts(&prepared, host_map, context.known_hosts);
    let entries = prepared
        .into_iter()
        .zip(statuses)
        .enumerate()
        .map(|(index, (entry, status))| build_entry(index, entry, status, context))
        .collect();
    ConfigImportPlan {
        source,
        entries,
        unknown_hosts,
    }
}

fn classify_entries(
    prepared: &[PreparedEntry],
    context: &PlanContext<'_>,
) -> Vec<ConfigImportStatus> {
    let valid_positions: Vec<usize> = prepared
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.is_valid())
        .map(|(position, _)| position)
        .collect();
    let candidates: Vec<TunnelDefinition> = valid_positions
        .iter()
        .map(|position| prepared[*position].definition())
        .collect();
    let conflicts = classify_definitions(context.existing, context.settings, &candidates);
    let mut statuses: Vec<ConfigImportStatus> = prepared
        .iter()
        .map(|entry| ConfigImportStatus::Invalid {
            messages: entry.problems.clone(),
        })
        .collect();
    for (candidate_index, conflict) in conflicts.into_iter().enumerate() {
        statuses[valid_positions[candidate_index]] = status_of(
            conflict,
            &candidates[candidate_index],
            &valid_positions,
            context,
        );
    }
    statuses
}

fn status_of(
    conflict: Option<RouteConflict>,
    candidate: &TunnelDefinition,
    valid_positions: &[usize],
    context: &PlanContext<'_>,
) -> ConfigImportStatus {
    match conflict {
        None => ConfigImportStatus::Importable,
        Some(RouteConflict::AlreadyConfigured { existing_id }) => {
            ConfigImportStatus::AlreadyConfigured {
                existing_id,
                ignored_tags: ignored_tags(candidate, existing_id, context.existing),
            }
        }
        Some(RouteConflict::ListenConflict { holder_id }) => {
            ConfigImportStatus::ListenConflict { holder_id }
        }
        Some(RouteConflict::DuplicateInImport { earlier_index }) => {
            ConfigImportStatus::DuplicateInFile {
                earlier_index: valid_positions[earlier_index],
            }
        }
    }
}

fn ignored_tags(
    candidate: &TunnelDefinition,
    existing_id: crate::store::TunnelId,
    existing: &[TunnelDefinition],
) -> Vec<String> {
    let Some(holder) = existing.iter().find(|tunnel| tunnel.id == existing_id) else {
        return candidate.tags.clone();
    };
    candidate
        .tags
        .iter()
        .filter(|tag| !holder.tags.contains(tag))
        .cloned()
        .collect()
}

fn find_unknown_hosts(
    prepared: &[PreparedEntry],
    host_map: &BTreeMap<String, String>,
    known_hosts: &[String],
) -> Vec<String> {
    let unknown: BTreeSet<&String> = prepared
        .iter()
        .map(|entry| &entry.file_host)
        .filter(|host| find_ssh_host_problem(host).is_none())
        .filter(|host| !host_map.contains_key(*host) && !known_hosts.contains(*host))
        .collect();
    unknown.into_iter().cloned().collect()
}

fn build_entry(
    index: usize,
    prepared: PreparedEntry,
    status: ConfigImportStatus,
    context: &PlanContext<'_>,
) -> ConfigImportEntry {
    let definition = prepared.definition();
    let is_invalid = matches!(status, ConfigImportStatus::Invalid { .. });
    let risks = if is_invalid {
        Vec::new()
    } else {
        find_risks(&definition, context.settings)
    };
    let mut warnings = prepared.warnings;
    if find_ssh_host_problem(&prepared.tunnel.host).is_none()
        && !context.known_hosts.contains(&prepared.tunnel.host)
    {
        warnings.insert(
            0,
            ConfigImportEntryWarning::UnknownHost {
                host: prepared.tunnel.host.clone(),
            },
        );
    }
    let selected_by_default = status == ConfigImportStatus::Importable && risks.is_empty();
    ConfigImportEntry {
        index,
        effective_listen_address: definition
            .effective_listen_address(context.settings)
            .to_owned(),
        tunnel: prepared.tunnel,
        file_host: prepared.file_host,
        status,
        risks,
        warnings,
        selected_by_default,
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::runtime::NewTunnel;
    use crate::store::test_support::{tunnel, tunnel_on};
    use crate::store::{OpenOptions, TunnelKind};
    use crate::transfer::{ConfigImportRisk, ConfigImportSource};

    const HEADER: &str = "format = \"tunneler-export\"\nversion = 1\n";

    struct Fixture {
        existing: Vec<TunnelDefinition>,
        settings: Settings,
        known_hosts: Vec<String>,
        opener_ids: Vec<String>,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                existing: Vec::new(),
                settings: Settings::default(),
                known_hosts: vec!["nova".to_owned(), "atlas".to_owned()],
                opener_ids: vec!["viewer".to_owned()],
            }
        }

        fn plan(&self, entries: &[&str]) -> ConfigImportPlan {
            self.plan_with_map(entries, &BTreeMap::new())
        }

        fn plan_with_map(
            &self,
            entries: &[&str],
            host_map: &BTreeMap<String, String>,
        ) -> ConfigImportPlan {
            let contents = format!("{HEADER}{}", entries.join(""));
            let context = PlanContext {
                existing: &self.existing,
                settings: &self.settings,
                known_hosts: &self.known_hosts,
                opener_ids: &self.opener_ids,
            };
            plan_import(&contents, host_map, &context).unwrap()
        }
    }

    fn entry(host: &str, kind: &str, port: u16, extra: &str) -> String {
        format!(
            "\n[[tunnels]]\nhost = \"{host}\"\nkind = \"{kind}\"\nlisten_port = {port}\n{extra}"
        )
    }

    fn statuses(plan: &ConfigImportPlan) -> Vec<&ConfigImportStatus> {
        plan.entries.iter().map(|entry| &entry.status).collect()
    }

    #[test]
    fn should_plan_an_importable_entry_selected_by_default() {
        let plan = Fixture::new().plan(&[&entry("nova", "local", 4000, "")]);

        let planned = &plan.entries[0];
        assert_eq!(plan.source, ConfigImportSource::Export);
        assert_eq!(planned.status, ConfigImportStatus::Importable);
        assert_eq!(planned.risks, Vec::new());
        assert_eq!(planned.warnings, Vec::new());
        assert!(planned.selected_by_default);
        assert_eq!(planned.effective_listen_address, "127.0.0.1");
        assert_eq!(planned.file_host, "nova");
        assert_eq!(plan.unknown_hosts, Vec::<String>::new());
    }

    #[test]
    fn should_flag_already_configured_with_the_tags_the_existing_tunnel_lacks() {
        let existing = TunnelDefinition {
            tags: vec!["dev".to_owned()],
            ..tunnel(TunnelKind::Local)
        };
        let mut fixture = Fixture::new();
        fixture.existing = vec![existing.clone()];

        let plan = fixture.plan(&[&entry("nova", "local", 3333, "tags = [\"dev\", \"new\"]\n")]);

        assert_eq!(
            plan.entries[0].status,
            ConfigImportStatus::AlreadyConfigured {
                existing_id: existing.id,
                ignored_tags: vec!["new".to_owned()]
            }
        );
        assert!(!plan.entries[0].selected_by_default);
    }

    #[test]
    fn should_flag_a_listen_conflict_with_the_holder_when_another_tunnel_listens_there() {
        let existing = tunnel(TunnelKind::Local);
        let mut fixture = Fixture::new();
        fixture.existing = vec![existing.clone()];

        let plan = fixture.plan(&[&entry("atlas", "local", 3333, "")]);

        assert_eq!(
            plan.entries[0].status,
            ConfigImportStatus::ListenConflict {
                holder_id: existing.id
            }
        );
    }

    #[test]
    fn should_flag_a_duplicate_in_the_file_with_the_position_of_the_earlier_entry() {
        let plan = Fixture::new().plan(&[
            &entry("nova", "local", 4000, ""),
            &entry("nova", "local", 4001, ""),
            &entry("atlas", "dynamic", 4000, ""),
        ]);

        assert_eq!(
            statuses(&plan),
            vec![
                &ConfigImportStatus::Importable,
                &ConfigImportStatus::Importable,
                &ConfigImportStatus::DuplicateInFile { earlier_index: 0 },
            ]
        );
    }

    #[test]
    fn should_not_count_an_invalid_entry_as_the_earlier_holder_of_a_port() {
        let plan = Fixture::new().plan(&[
            &entry("nova", "local", 4000, "tags = [\"a b\"]\n"),
            &entry("nova", "local", 4000, ""),
        ]);

        assert_eq!(plan.entries[1].status, ConfigImportStatus::Importable);
    }

    #[test]
    fn should_report_the_messages_when_an_entry_is_invalid() {
        let plan = Fixture::new().plan(&[&entry("nova", "local", 0, "")]);

        let ConfigImportStatus::Invalid { messages } = &plan.entries[0].status else {
            panic!("expected an invalid entry");
        };
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("listen_port must be between 1 and 65535"));
        assert!(!messages[0].contains("0000000000"));
        assert_eq!(plan.entries[0].risks, Vec::new());
        assert!(!plan.entries[0].selected_by_default);
    }

    #[test]
    fn should_keep_the_raw_values_when_an_entry_is_invalid() {
        let plan = Fixture::new().plan(&[&entry("nova", "local", 4000, "tags = [\"a b\"]\n")]);

        assert_eq!(plan.entries[0].tunnel.tags, vec!["a b".to_owned()]);
    }

    #[test]
    fn should_not_select_by_default_when_the_entry_listens_beyond_loopback() {
        let plan = Fixture::new().plan(&[&entry(
            "nova",
            "local",
            4000,
            "listen_address = \"0.0.0.0\"\n",
        )]);

        assert_eq!(
            plan.entries[0].risks,
            vec![ConfigImportRisk::NonLoopbackListen {
                address: "0.0.0.0".to_owned()
            }]
        );
        assert_eq!(plan.entries[0].status, ConfigImportStatus::Importable);
        assert!(!plan.entries[0].selected_by_default);
    }

    #[test]
    fn should_not_select_by_default_when_a_tag_starts_on_launch() {
        let mut fixture = Fixture::new();
        fixture.settings.start_on_launch_tags = vec!["boot".to_owned()];

        let plan = fixture.plan(&[&entry("nova", "local", 4000, "tags = [\"boot\"]\n")]);

        assert_eq!(
            plan.entries[0].risks,
            vec![ConfigImportRisk::StartsOnLaunch {
                tags: vec!["boot".to_owned()]
            }]
        );
        assert!(!plan.entries[0].selected_by_default);
    }

    #[test]
    fn should_not_select_by_default_when_a_remote_tunnel_targets_beyond_loopback() {
        let plan = Fixture::new().plan(&[&entry(
            "nova",
            "remote",
            9000,
            "target_host = \"db.internal\"\n",
        )]);

        assert_eq!(
            plan.entries[0].risks,
            vec![ConfigImportRisk::RemoteNonLoopbackTarget {
                target_host: "db.internal".to_owned()
            }]
        );
    }

    #[test]
    fn should_warn_and_list_the_host_when_no_host_entry_names_it() {
        let plan = Fixture::new().plan(&[
            &entry("oldbox", "local", 4000, ""),
            &entry("oldbox", "local", 4001, ""),
            &entry("beta", "local", 4002, ""),
            &entry("nova", "local", 4003, ""),
        ]);

        assert_eq!(
            plan.unknown_hosts,
            vec!["beta".to_owned(), "oldbox".to_owned()]
        );
        assert_eq!(
            plan.entries[0].warnings,
            vec![ConfigImportEntryWarning::UnknownHost {
                host: "oldbox".to_owned()
            }]
        );
        assert_eq!(plan.entries[3].warnings, Vec::new());
        assert!(plan.entries[0].selected_by_default);
    }

    #[test]
    fn should_apply_the_host_map_before_classifying() {
        let existing = tunnel(TunnelKind::Local);
        let mut fixture = Fixture::new();
        fixture.existing = vec![existing.clone()];
        let map = BTreeMap::from([("oldbox".to_owned(), "nova".to_owned())]);

        let plan = fixture.plan_with_map(&[&entry("oldbox", "local", 3333, "")], &map);

        assert_eq!(plan.entries[0].tunnel.host, "nova");
        assert_eq!(plan.entries[0].file_host, "oldbox");
        assert_eq!(
            plan.entries[0].status,
            ConfigImportStatus::AlreadyConfigured {
                existing_id: existing.id,
                ignored_tags: Vec::new()
            }
        );
        assert_eq!(plan.unknown_hosts, Vec::<String>::new());
        assert_eq!(plan.entries[0].warnings, Vec::new());
    }

    #[test]
    fn should_warn_about_the_mapped_host_when_it_is_not_a_known_alias_either() {
        let map = BTreeMap::from([("oldbox".to_owned(), "ghost".to_owned())]);

        let plan = Fixture::new().plan_with_map(&[&entry("oldbox", "local", 4000, "")], &map);

        assert_eq!(plan.unknown_hosts, Vec::<String>::new());
        assert_eq!(
            plan.entries[0].warnings,
            vec![ConfigImportEntryWarning::UnknownHost {
                host: "ghost".to_owned()
            }]
        );
    }

    #[test]
    fn should_ignore_the_host_map_keys_no_entry_uses() {
        let map = BTreeMap::from([("nobody".to_owned(), "nova".to_owned())]);

        let plan = Fixture::new().plan_with_map(&[&entry("nova", "local", 4000, "")], &map);

        assert_eq!(plan.entries[0].status, ConfigImportStatus::Importable);
    }

    #[test]
    fn should_mark_the_entry_invalid_when_the_mapped_host_breaks_the_host_rule() {
        let map = BTreeMap::from([("oldbox".to_owned(), "a;b".to_owned())]);

        let plan = Fixture::new().plan_with_map(&[&entry("oldbox", "local", 4000, "")], &map);

        assert!(matches!(
            plan.entries[0].status,
            ConfigImportStatus::Invalid { .. }
        ));
        assert_eq!(plan.entries[0].warnings, Vec::new());
    }

    #[test]
    fn should_mark_the_entry_invalid_and_not_list_its_host_when_the_host_breaks_the_rule() {
        let plan = Fixture::new().plan(&[&entry("-oProxyCommand=x", "local", 4000, "")]);

        assert!(matches!(
            plan.entries[0].status,
            ConfigImportStatus::Invalid { .. }
        ));
        assert_eq!(plan.unknown_hosts, Vec::<String>::new());
    }

    #[test]
    fn should_drop_the_opener_and_warn_when_this_machine_does_not_know_it() {
        let plan = Fixture::new().plan(&[&entry(
            "nova",
            "local",
            4000,
            "open = { with = \"ghost\", path = \"/x\" }\n",
        )]);

        assert_eq!(plan.entries[0].tunnel.open.with, None);
        assert_eq!(plan.entries[0].tunnel.open.path.as_deref(), Some("/x"));
        assert_eq!(
            plan.entries[0].warnings,
            vec![ConfigImportEntryWarning::OpenerDropped {
                opener: "ghost".to_owned()
            }]
        );
        assert!(plan.entries[0].selected_by_default);
    }

    #[test]
    fn should_keep_the_opener_when_it_is_known_on_this_machine() {
        let plan = Fixture::new().plan(&[&entry(
            "nova",
            "local",
            4000,
            "open = { with = \"viewer\" }\n",
        )]);

        assert_eq!(plan.entries[0].tunnel.open.with.as_deref(), Some("viewer"));
        assert_eq!(plan.entries[0].warnings, Vec::new());
    }

    #[test]
    fn should_number_the_entries_by_their_position_in_the_file() {
        let plan = Fixture::new().plan(&[
            &entry("nova", "local", 4000, ""),
            &entry("nova", "local", 0, ""),
            &entry("nova", "local", 4002, ""),
        ]);

        let indexes: Vec<usize> = plan.entries.iter().map(|planned| planned.index).collect();
        assert_eq!(indexes, vec![0, 1, 2]);
    }

    #[test]
    fn should_describe_the_tunnel_that_would_be_created() {
        let plan = Fixture::new().plan(&[&entry(
            "nova",
            "local",
            4000,
            "tags = [\"dev\"]\nstandby = true\n",
        )]);

        assert_eq!(
            plan.entries[0].tunnel,
            NewTunnel {
                host: "nova".to_owned(),
                kind: TunnelKind::Local,
                listen_port: 4000,
                listen_address: None,
                target_host: None,
                target_port: None,
                tags: vec!["dev".to_owned()],
                open: OpenOptions::default(),
                standby: Some(true),
            }
        );
    }

    #[test]
    fn should_ignore_the_existing_tunnels_on_another_remote_host() {
        let mut fixture = Fixture::new();
        fixture.existing = vec![tunnel_on(TunnelKind::Remote, 8080)];

        let plan = fixture.plan(&[&entry("atlas", "remote", 8080, "")]);

        assert_eq!(plan.entries[0].status, ConfigImportStatus::Importable);
    }
}
