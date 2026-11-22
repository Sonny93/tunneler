//! Deciding which of the tunnels a client asks to remove may be removed.

use std::collections::HashSet;

use super::analysis::{KnownTunnel, is_devslot_tunnel_of};
use super::discovery::DevslotResyncTunnel;
use crate::store::TunnelId;

/// What a resync does, decided against the tunnels that exist now.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ResyncPlan {
    /// The tunnels that are gone once the plan is applied, in the order they
    /// were asked: those to remove and those already absent.
    pub(crate) removed: Vec<TunnelId>,
    /// The temporary tunnels to remove from the catalog.
    pub(crate) temporaries: Vec<TunnelId>,
    /// The saved tunnels to delete.
    pub(crate) saved: Vec<TunnelId>,
    /// The tunnels that changed since the discovery and are left alone.
    pub(crate) skipped: Vec<TunnelId>,
}

/// A tunnel is removed only when it still has exactly the definition the
/// client was given, the same temporary status, and is still a devslot
/// tunnel of `host`. One that no longer exists counts as removed.
pub(crate) fn plan_resync(
    host: &str,
    requested: &[DevslotResyncTunnel],
    known: &[KnownTunnel],
) -> ResyncPlan {
    let mut plan = ResyncPlan::default();
    let mut seen = HashSet::new();
    for request in requested {
        let id = request.definition.id;
        if !seen.insert(id) {
            continue;
        }
        let Some(current) = known.iter().find(|tunnel| tunnel.definition.id == id) else {
            plan.removed.push(id);
            continue;
        };
        if !is_unchanged(request, current, host) {
            plan.skipped.push(id);
            continue;
        }
        plan.removed.push(id);
        if current.is_temporary {
            plan.temporaries.push(id);
        } else {
            plan.saved.push(id);
        }
    }
    plan
}

fn is_unchanged(request: &DevslotResyncTunnel, current: &KnownTunnel, host: &str) -> bool {
    request.is_temporary == current.is_temporary
        && request.definition == current.definition
        && is_devslot_tunnel_of(current, host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devslot::DevslotOrigin;
    use crate::store::{OpenOptions, TunnelDefinition, TunnelKind};
    use pretty_assertions::assert_eq;

    const HOST: &str = "vega";

    fn definition(tags: &[&str]) -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            host: HOST.to_owned(),
            kind: TunnelKind::Local,
            listen_port: 20050,
            listen_address: Some("127.0.0.1".to_owned()),
            target_host: None,
            target_port: None,
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            open: OpenOptions::default(),
            standby: None,
        }
    }

    fn saved_devslot() -> KnownTunnel {
        KnownTunnel {
            definition: definition(&["devslot"]),
            is_temporary: false,
            origin: None,
        }
    }

    fn temporary_devslot() -> KnownTunnel {
        KnownTunnel {
            definition: definition(&[]),
            is_temporary: true,
            origin: Some(DevslotOrigin::default()),
        }
    }

    fn request(known: &KnownTunnel) -> DevslotResyncTunnel {
        DevslotResyncTunnel {
            definition: known.definition.clone(),
            is_temporary: known.is_temporary,
        }
    }

    #[test]
    fn should_remove_a_saved_tunnel_when_nothing_changed() {
        let tunnel = saved_devslot();

        let plan = plan_resync(HOST, &[request(&tunnel)], std::slice::from_ref(&tunnel));

        assert_eq!(
            (plan.saved, plan.temporaries, plan.skipped),
            (vec![tunnel.definition.id], vec![], vec![])
        );
    }

    #[test]
    fn should_remove_a_temporary_tunnel_when_nothing_changed() {
        let tunnel = temporary_devslot();

        let plan = plan_resync(HOST, &[request(&tunnel)], std::slice::from_ref(&tunnel));

        assert_eq!(
            (plan.saved, plan.temporaries, plan.skipped),
            (vec![], vec![tunnel.definition.id], vec![])
        );
    }

    #[test]
    fn should_count_a_tunnel_as_removed_when_it_is_already_gone() {
        let tunnel = saved_devslot();

        let plan = plan_resync(HOST, &[request(&tunnel)], &[]);

        assert_eq!(
            (plan.removed, plan.saved, plan.temporaries),
            (vec![tunnel.definition.id], vec![], vec![])
        );
    }

    #[test]
    fn should_skip_the_tunnel_when_its_definition_changed() {
        let tunnel = saved_devslot();
        let mut changed = tunnel.clone();
        changed.definition.listen_port = 20051;

        let plan = plan_resync(HOST, &[request(&tunnel)], &[changed]);

        assert_eq!(
            (plan.skipped, plan.removed),
            (vec![tunnel.definition.id], vec![])
        );
    }

    #[test]
    fn should_skip_the_tunnel_when_it_was_saved_since_the_discovery() {
        let tunnel = temporary_devslot();
        let mut saved = tunnel.clone();
        saved.is_temporary = false;
        saved.origin = None;
        saved.definition.tags = vec!["devslot".to_owned()];

        let plan = plan_resync(HOST, &[request(&tunnel)], &[saved]);

        assert_eq!(plan.skipped, vec![tunnel.definition.id]);
    }

    #[test]
    fn should_skip_the_tunnel_when_it_is_no_longer_a_devslot_tunnel() {
        let tunnel = saved_devslot();
        let mut untagged = tunnel.clone();
        untagged.definition.tags = Vec::new();
        let asked = DevslotResyncTunnel {
            definition: untagged.definition.clone(),
            is_temporary: false,
        };

        let plan = plan_resync(HOST, &[asked], &[untagged]);

        assert_eq!(plan.skipped, vec![tunnel.definition.id]);
    }

    #[test]
    fn should_skip_the_tunnel_when_it_belongs_to_another_host() {
        let tunnel = saved_devslot();

        let plan = plan_resync(
            "elsewhere",
            &[request(&tunnel)],
            std::slice::from_ref(&tunnel),
        );

        assert_eq!(plan.skipped, vec![tunnel.definition.id]);
    }

    #[test]
    fn should_handle_a_tunnel_once_when_it_is_asked_twice() {
        let tunnel = saved_devslot();

        let plan = plan_resync(
            HOST,
            &[request(&tunnel), request(&tunnel)],
            std::slice::from_ref(&tunnel),
        );

        assert_eq!(plan.removed, vec![tunnel.definition.id]);
    }

    #[test]
    fn should_keep_the_order_of_the_request_when_several_tunnels_are_removed() {
        let first = saved_devslot();
        let second = temporary_devslot();

        let plan = plan_resync(
            HOST,
            &[request(&second), request(&first)],
            &[first.clone(), second.clone()],
        );

        assert_eq!(
            plan.removed,
            vec![second.definition.id, first.definition.id]
        );
    }
}
