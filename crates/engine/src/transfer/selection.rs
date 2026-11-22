use std::collections::BTreeSet;

use super::plan_types::{ConfigImportPlan, ConfigImportSkipped, ConfigImportStatus};
use crate::runtime::NewTunnel;

/// The entries an apply adds, and the requested ones it leaves out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportSelection {
    /// The importable requested entries: their position in the file and what
    /// to create.
    pub(crate) to_add: Vec<(usize, NewTunnel)>,
    /// The requested entries that are no longer importable.
    pub(crate) skipped: Vec<ConfigImportSkipped>,
}

/// Picks the requested entries of a fresh plan, in file order. An entry
/// that is not importable now is skipped with its current status, a risky one
/// that is importable is added since it was asked for explicitly, and a
/// position beyond the file is ignored.
pub(crate) fn select_importable(plan: ConfigImportPlan, requested: &[usize]) -> ImportSelection {
    let wanted: BTreeSet<usize> = requested.iter().copied().collect();
    let mut to_add = Vec::new();
    let mut skipped = Vec::new();
    for entry in plan
        .entries
        .into_iter()
        .filter(|entry| wanted.contains(&entry.index))
    {
        if entry.status == ConfigImportStatus::Importable {
            to_add.push((entry.index, entry.tunnel));
            continue;
        }
        skipped.push(ConfigImportSkipped {
            index: entry.index,
            status: entry.status,
        });
    }
    ImportSelection { to_add, skipped }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use pretty_assertions::assert_eq;

    use super::*;
    use crate::store::Settings;
    use crate::transfer::plan::{PlanContext, plan_import};

    const FILE: &str = "format = \"tunneler-export\"\nversion = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 4000\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 4000\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 4001\nlisten_address = \"0.0.0.0\"\n";

    fn plan() -> ConfigImportPlan {
        let settings = Settings::default();
        let known_hosts = vec!["nova".to_owned()];
        let context = PlanContext {
            existing: &[],
            settings: &settings,
            known_hosts: &known_hosts,
            opener_ids: &[],
        };
        plan_import(FILE, &BTreeMap::new(), &context).unwrap()
    }

    #[test]
    fn should_add_the_requested_importable_entries_in_file_order() {
        let selection = select_importable(plan(), &[2, 0]);

        let positions: Vec<usize> = selection.to_add.iter().map(|(index, _)| *index).collect();
        assert_eq!(positions, vec![0, 2]);
        assert_eq!(selection.skipped, Vec::new());
    }

    #[test]
    fn should_skip_with_the_current_status_when_a_requested_entry_is_not_importable() {
        let selection = select_importable(plan(), &[1]);

        assert_eq!(selection.to_add, Vec::new());
        assert_eq!(
            selection.skipped,
            vec![ConfigImportSkipped {
                index: 1,
                status: ConfigImportStatus::DuplicateInFile { earlier_index: 0 }
            }]
        );
    }

    #[test]
    fn should_not_add_an_entry_nobody_asked_for() {
        let selection = select_importable(plan(), &[0]);

        assert_eq!(selection.to_add.len(), 1);
    }

    #[test]
    fn should_ignore_a_position_beyond_the_file_and_repeated_positions() {
        let selection = select_importable(plan(), &[0, 0, 99]);

        assert_eq!(selection.to_add.len(), 1);
        assert_eq!(selection.skipped, Vec::new());
    }
}
