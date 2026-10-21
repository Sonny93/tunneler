use std::collections::HashSet;

use crate::store::{TunnelDefinition, TunnelId};

/// What a change of definition means for a tunnel that may be running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DefinitionChange {
    /// Nothing changed.
    Unchanged,
    /// Only labels and start-up preferences changed (tags, open options,
    /// standby): a running tunnel is not disturbed. A new standby choice
    /// applies at the next start.
    InPlace,
    /// A field that shapes the connection changed (host, kind, listen or
    /// target address): a running tunnel must restart.
    RequiresRestart,
}

pub(crate) fn classify_change(old: &TunnelDefinition, new: &TunnelDefinition) -> DefinitionChange {
    if old == new {
        return DefinitionChange::Unchanged;
    }
    let is_connection_unchanged = old.host == new.host
        && old.kind == new.kind
        && old.listen_port == new.listen_port
        && old.listen_address == new.listen_address
        && old.target_host == new.target_host
        && old.target_port == new.target_port;
    if is_connection_unchanged {
        return DefinitionChange::InPlace;
    }
    DefinitionChange::RequiresRestart
}

/// How the set of tunnels moves from one configuration to the next.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReloadPlan {
    /// Tunnels that are gone and must be stopped.
    pub(crate) removed: Vec<TunnelId>,
    /// Tunnels that are new and start stopped.
    pub(crate) added: Vec<TunnelDefinition>,
    /// Tunnels whose definition differs, with their new definition.
    pub(crate) changed: Vec<TunnelDefinition>,
}

pub(crate) fn plan_reload(current: &[TunnelDefinition], next: &[TunnelDefinition]) -> ReloadPlan {
    let next_ids: HashSet<TunnelId> = next.iter().map(|definition| definition.id).collect();
    let removed = current
        .iter()
        .map(|definition| definition.id)
        .filter(|id| !next_ids.contains(id))
        .collect();
    let mut plan = ReloadPlan {
        removed,
        ..ReloadPlan::default()
    };
    for definition in next {
        match current.iter().find(|existing| existing.id == definition.id) {
            None => plan.added.push(definition.clone()),
            Some(existing) if existing != definition => plan.changed.push(definition.clone()),
            Some(_) => {}
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use crate::store::TunnelKind;

    use super::*;

    fn definition() -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            host: "nova".to_owned(),
            kind: TunnelKind::Local,
            listen_port: 3333,
            listen_address: None,
            target_host: None,
            target_port: None,
            tags: Vec::new(),
            open: crate::store::OpenOptions::default(),
            standby: None,
        }
    }

    fn with(
        base: &TunnelDefinition,
        change: impl FnOnce(&mut TunnelDefinition),
    ) -> TunnelDefinition {
        let mut changed = base.clone();
        change(&mut changed);
        changed
    }

    #[test]
    fn should_report_no_change_when_the_definitions_are_equal() {
        let old = definition();

        assert_eq!(
            classify_change(&old, &old.clone()),
            DefinitionChange::Unchanged
        );
    }

    #[test]
    fn should_apply_in_place_when_only_the_tags_changed() {
        let old = definition();
        let new = with(&old, |new| new.tags = vec!["dev".to_owned()]);

        assert_eq!(classify_change(&old, &new), DefinitionChange::InPlace);
    }

    #[test]
    fn should_apply_in_place_when_only_the_standby_choice_changed() {
        let old = definition();
        let new = with(&old, |new| new.standby = Some(true));

        assert_eq!(classify_change(&old, &new), DefinitionChange::InPlace);
    }

    #[test]
    fn should_restart_when_the_host_changed() {
        let old = definition();
        let new = with(&old, |new| new.host = "other".to_owned());

        assert_eq!(
            classify_change(&old, &new),
            DefinitionChange::RequiresRestart
        );
    }

    #[test]
    fn should_restart_when_the_kind_changed() {
        let old = definition();
        let new = with(&old, |new| new.kind = TunnelKind::Dynamic);

        assert_eq!(
            classify_change(&old, &new),
            DefinitionChange::RequiresRestart
        );
    }

    #[test]
    fn should_restart_when_the_listen_port_changed() {
        let old = definition();
        let new = with(&old, |new| new.listen_port = 4444);

        assert_eq!(
            classify_change(&old, &new),
            DefinitionChange::RequiresRestart
        );
    }

    #[test]
    fn should_restart_when_the_listen_address_changed() {
        let old = definition();
        let new = with(&old, |new| new.listen_address = Some("0.0.0.0".to_owned()));

        assert_eq!(
            classify_change(&old, &new),
            DefinitionChange::RequiresRestart
        );
    }

    #[test]
    fn should_restart_when_the_target_host_changed() {
        let old = definition();
        let new = with(&old, |new| new.target_host = Some("db".to_owned()));

        assert_eq!(
            classify_change(&old, &new),
            DefinitionChange::RequiresRestart
        );
    }

    #[test]
    fn should_restart_when_the_target_port_changed() {
        let old = definition();
        let new = with(&old, |new| new.target_port = Some(5432));

        assert_eq!(
            classify_change(&old, &new),
            DefinitionChange::RequiresRestart
        );
    }

    #[test]
    fn should_restart_when_a_label_and_the_port_changed_together() {
        let old = definition();
        let new = with(&old, |new| {
            new.tags = vec!["dev".to_owned()];
            new.listen_port = 4444;
        });

        assert_eq!(
            classify_change(&old, &new),
            DefinitionChange::RequiresRestart
        );
    }

    #[test]
    fn should_plan_nothing_when_the_configurations_are_equal() {
        let tunnels = vec![definition()];

        assert_eq!(plan_reload(&tunnels, &tunnels), ReloadPlan::default());
    }

    #[test]
    fn should_remove_the_tunnel_when_it_is_missing_from_the_next_configuration() {
        let gone = definition();

        let plan = plan_reload(std::slice::from_ref(&gone), &[]);

        assert_eq!(plan.removed, [gone.id]);
    }

    #[test]
    fn should_add_the_tunnel_when_it_is_missing_from_the_current_configuration() {
        let fresh = definition();

        let plan = plan_reload(&[], std::slice::from_ref(&fresh));

        assert_eq!(plan.added, [fresh]);
    }

    #[test]
    fn should_report_the_new_definition_when_a_tunnel_changed() {
        let old = definition();
        let new = with(&old, |new| new.listen_port = 4444);

        let plan = plan_reload(&[old], std::slice::from_ref(&new));

        assert_eq!(plan.changed, [new]);
    }

    #[test]
    fn should_not_report_unchanged_tunnels_when_another_one_changed() {
        let steady = definition();
        let old = definition();
        let new = with(&old, |new| new.listen_port = 4444);

        let plan = plan_reload(&[steady.clone(), old], &[steady, new]);

        assert_eq!((plan.removed.len(), plan.added.len()), (0, 0));
    }
}
