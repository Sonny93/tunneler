use std::collections::HashMap;

use crate::runtime::NewTunnel;
use crate::store::{
    ListenEndpoint, OpenOptions, Settings, TunnelDefinition, TunnelId, TunnelKind, effective_target,
};

/// A route a client wants to import, host already chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRoute {
    /// The `ssh_config` alias the route goes through.
    pub host: String,
    /// The direction of the forward.
    pub kind: TunnelKind,
    /// The port to listen on.
    pub listen_port: u16,
    /// The bind address; the default of the kind when absent.
    pub listen_address: Option<String>,
    /// The host the traffic is forwarded to; `127.0.0.1` when absent.
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to; the listen port when absent.
    pub target_port: Option<u16>,
}

/// What stops a route from being imported as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteConflict {
    /// An existing tunnel has the same host, kind, listen address and port,
    /// and target.
    AlreadyConfigured {
        /// The existing tunnel.
        existing_id: TunnelId,
    },
    /// An existing tunnel holds the same listen address and port: local and
    /// dynamic tunnels share the ports of this machine whatever their host,
    /// remote ones the ports of their host.
    ListenConflict {
        /// The tunnel that holds it.
        holder_id: TunnelId,
    },
    /// An earlier candidate holds the same listen address and port.
    DuplicateInImport {
        /// The position of the earlier candidate.
        earlier_index: usize,
    },
}

/// For each candidate, what stops it from being imported as it is: an
/// identical existing tunnel, an existing tunnel on the same listen address
/// and port, or an earlier candidate doing so.
///
/// It compares with the normalization of the validation of the store, so a
/// candidate it lets through never breaks `validate` once added, as long as
/// the earlier ones that were let through are added too.
pub fn classify_routes(
    existing: &[TunnelDefinition],
    settings: &Settings,
    candidates: &[ImportRoute],
) -> Vec<Option<RouteConflict>> {
    let definitions: Vec<TunnelDefinition> = candidates.iter().map(route_definition).collect();
    classify_definitions(existing, settings, &definitions)
}

/// [`classify_routes`] for candidates already shaped like tunnels.
pub(crate) fn classify_definitions(
    existing: &[TunnelDefinition],
    settings: &Settings,
    candidates: &[TunnelDefinition],
) -> Vec<Option<RouteConflict>> {
    let mut holders: HashMap<ListenEndpoint, &TunnelDefinition> = HashMap::new();
    for tunnel in existing {
        holders
            .entry(ListenEndpoint::of(tunnel, settings))
            .or_insert(tunnel);
    }
    let mut earlier: HashMap<ListenEndpoint, usize> = HashMap::new();
    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let endpoint = ListenEndpoint::of(candidate, settings);
            let conflict = holders
                .get(&endpoint)
                .map(|holder| existing_conflict(holder, candidate))
                .or_else(|| {
                    earlier
                        .get(&endpoint)
                        .map(|&earlier_index| RouteConflict::DuplicateInImport { earlier_index })
                });
            earlier.entry(endpoint).or_insert(index);
            conflict
        })
        .collect()
}

fn existing_conflict(holder: &TunnelDefinition, candidate: &TunnelDefinition) -> RouteConflict {
    let is_identical = holder.host == candidate.host
        && holder.kind == candidate.kind
        && effective_target(holder) == effective_target(candidate);
    if is_identical {
        return RouteConflict::AlreadyConfigured {
            existing_id: holder.id,
        };
    }
    RouteConflict::ListenConflict {
        holder_id: holder.id,
    }
}

fn route_definition(route: &ImportRoute) -> TunnelDefinition {
    NewTunnel {
        host: route.host.clone(),
        kind: route.kind,
        listen_port: route.listen_port,
        listen_address: route.listen_address.clone(),
        target_host: route.target_host.clone(),
        target_port: route.target_port,
        tags: Vec::new(),
        open: OpenOptions::default(),
        standby: None,
    }
    .into_definition(TunnelId::generate())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::store::test_support::{tunnel, tunnel_on};
    use crate::store::validate;

    fn route(kind: TunnelKind, listen_port: u16) -> ImportRoute {
        ImportRoute {
            host: "nova".to_owned(),
            kind,
            listen_port,
            listen_address: None,
            target_host: None,
            target_port: None,
        }
    }

    fn classify_one(
        candidate: ImportRoute,
        existing: &[TunnelDefinition],
    ) -> Option<RouteConflict> {
        classify_routes(existing, &Settings::default(), &[candidate])
            .into_iter()
            .next()
            .flatten()
    }

    #[test]
    fn should_report_nothing_when_there_is_no_existing_tunnel() {
        assert_eq!(classify_one(route(TunnelKind::Local, 3333), &[]), None);
    }

    #[test]
    fn should_flag_an_identical_tunnel_as_already_configured() {
        let existing = tunnel(TunnelKind::Local);

        assert_eq!(
            classify_one(
                route(TunnelKind::Local, 3333),
                std::slice::from_ref(&existing)
            ),
            Some(RouteConflict::AlreadyConfigured {
                existing_id: existing.id
            })
        );
    }

    #[test]
    fn should_compare_the_effective_target_when_the_candidate_spells_the_defaults() {
        let existing = tunnel(TunnelKind::Local);
        let candidate = ImportRoute {
            target_host: Some("127.0.0.1".to_owned()),
            target_port: Some(3333),
            ..route(TunnelKind::Local, 3333)
        };

        assert_eq!(
            classify_one(candidate, std::slice::from_ref(&existing)),
            Some(RouteConflict::AlreadyConfigured {
                existing_id: existing.id
            })
        );
    }

    #[test]
    fn should_ignore_case_and_spaces_when_comparing_addresses_and_targets() {
        let existing = TunnelDefinition {
            listen_address: Some("LocalHost".to_owned()),
            target_host: Some("DB.internal".to_owned()),
            ..tunnel(TunnelKind::Local)
        };
        let candidate = ImportRoute {
            listen_address: Some(" localhost ".to_owned()),
            target_host: Some("db.INTERNAL ".to_owned()),
            ..route(TunnelKind::Local, 3333)
        };

        assert_eq!(
            classify_one(candidate, std::slice::from_ref(&existing)),
            Some(RouteConflict::AlreadyConfigured {
                existing_id: existing.id
            })
        );
    }

    #[test]
    fn should_flag_the_same_port_with_another_target_as_a_listen_conflict() {
        let existing = tunnel(TunnelKind::Local);
        let candidate = ImportRoute {
            target_port: Some(80),
            ..route(TunnelKind::Local, 3333)
        };

        assert_eq!(
            classify_one(candidate, std::slice::from_ref(&existing)),
            Some(RouteConflict::ListenConflict {
                holder_id: existing.id
            })
        );
    }

    #[test]
    fn should_flag_a_listen_conflict_whatever_the_ssh_host() {
        let existing = tunnel(TunnelKind::Local);
        let candidate = ImportRoute {
            host: "other".to_owned(),
            ..route(TunnelKind::Local, 3333)
        };

        assert_eq!(
            classify_one(candidate, std::slice::from_ref(&existing)),
            Some(RouteConflict::ListenConflict {
                holder_id: existing.id
            })
        );
    }

    #[test]
    fn should_treat_local_and_dynamic_forwards_as_sharing_the_local_ports() {
        let existing = tunnel(TunnelKind::Local);

        assert_eq!(
            classify_one(
                route(TunnelKind::Dynamic, 3333),
                std::slice::from_ref(&existing)
            ),
            Some(RouteConflict::ListenConflict {
                holder_id: existing.id
            })
        );
    }

    #[test]
    fn should_not_flag_another_listen_address() {
        let candidate = ImportRoute {
            listen_address: Some("127.0.0.2".to_owned()),
            ..route(TunnelKind::Local, 3333)
        };

        assert_eq!(classify_one(candidate, &[tunnel(TunnelKind::Local)]), None);
    }

    #[test]
    fn should_not_conflict_a_remote_forward_with_a_local_one() {
        assert_eq!(
            classify_one(
                route(TunnelKind::Remote, 3333),
                &[tunnel(TunnelKind::Local)]
            ),
            None
        );
    }

    #[test]
    fn should_flag_a_remote_forward_on_the_same_host_address_and_port() {
        let existing = tunnel_on(TunnelKind::Remote, 8080);
        let candidate = ImportRoute {
            target_port: Some(80),
            ..route(TunnelKind::Remote, 8080)
        };

        assert_eq!(
            classify_one(candidate, std::slice::from_ref(&existing)),
            Some(RouteConflict::ListenConflict {
                holder_id: existing.id
            })
        );
    }

    #[test]
    fn should_not_flag_a_remote_forward_on_another_host() {
        let candidate = ImportRoute {
            host: "other".to_owned(),
            ..route(TunnelKind::Remote, 8080)
        };

        assert_eq!(
            classify_one(candidate, &[tunnel_on(TunnelKind::Remote, 8080)]),
            None
        );
    }

    #[test]
    fn should_flag_a_later_candidate_repeating_an_earlier_listen_identity() {
        let candidates = [
            route(TunnelKind::Local, 4000),
            route(TunnelKind::Local, 4000),
        ];

        assert_eq!(
            classify_routes(&[], &Settings::default(), &candidates),
            vec![
                None,
                Some(RouteConflict::DuplicateInImport { earlier_index: 0 })
            ]
        );
    }

    #[test]
    fn should_point_at_the_first_candidate_when_three_share_a_listen_identity() {
        let candidates = [
            route(TunnelKind::Local, 4000),
            route(TunnelKind::Local, 4001),
            route(TunnelKind::Dynamic, 4000),
            route(TunnelKind::Local, 4000),
        ];

        let conflicts = classify_routes(&[], &Settings::default(), &candidates);

        assert_eq!(
            conflicts,
            vec![
                None,
                None,
                Some(RouteConflict::DuplicateInImport { earlier_index: 0 }),
                Some(RouteConflict::DuplicateInImport { earlier_index: 0 }),
            ]
        );
    }

    #[test]
    fn should_prefer_the_existing_tunnel_over_an_earlier_candidate() {
        let existing = tunnel(TunnelKind::Local);
        let candidates = [
            route(TunnelKind::Local, 3333),
            route(TunnelKind::Local, 3333),
        ];

        let conflicts = classify_routes(
            std::slice::from_ref(&existing),
            &Settings::default(),
            &candidates,
        );

        assert_eq!(
            conflicts[1],
            Some(RouteConflict::AlreadyConfigured {
                existing_id: existing.id
            })
        );
    }

    #[test]
    fn should_use_the_default_remote_address_of_the_settings_for_remote_routes() {
        let settings = Settings {
            default_remote_bind_address: "0.0.0.0".to_owned(),
            ..Settings::default()
        };
        let existing = TunnelDefinition {
            listen_address: Some("0.0.0.0".to_owned()),
            ..tunnel_on(TunnelKind::Remote, 8080)
        };

        let conflicts = classify_routes(
            std::slice::from_ref(&existing),
            &settings,
            &[route(TunnelKind::Remote, 8080)],
        );

        assert_eq!(
            conflicts,
            vec![Some(RouteConflict::AlreadyConfigured {
                existing_id: existing.id
            })]
        );
    }

    #[test]
    fn should_never_let_through_a_candidate_the_store_validation_refuses() {
        let settings = Settings::default();
        let existing = vec![
            tunnel_on(TunnelKind::Local, 3333),
            tunnel_on(TunnelKind::Remote, 8080),
        ];
        let candidates = [
            route(TunnelKind::Local, 3333),
            route(TunnelKind::Dynamic, 3333),
            route(TunnelKind::Local, 3334),
            route(TunnelKind::Local, 3334),
            route(TunnelKind::Remote, 8080),
            route(TunnelKind::Remote, 9090),
        ];

        let conflicts = classify_routes(&existing, &settings, &candidates);

        let accepted: Vec<TunnelDefinition> = candidates
            .iter()
            .zip(&conflicts)
            .filter(|(_, conflict)| conflict.is_none())
            .map(|(candidate, _)| route_definition(candidate))
            .collect();
        let document = crate::store::StoreDocument {
            tunnels: existing.into_iter().chain(accepted).collect(),
            ..crate::store::StoreDocument::default()
        };
        assert_eq!(validate(&document), Ok(()));
    }
}
