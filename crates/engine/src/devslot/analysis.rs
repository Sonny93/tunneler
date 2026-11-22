//! Comparing a devslot listing with the tunnels of a host: which listed ports
//! are already tunneled, and which devslot tunnels no longer match what the
//! host lists.
//!
//! Everything here is pure, so that the rules are tested without a host.

use std::collections::HashMap;

use super::listing::{DevslotInstance, DevslotPortStatus};
use super::origin::DevslotOrigin;
use super::project_tag::{DEVSLOT_TAG, PROJECT_TAG_PREFIX};
use crate::store::{DevslotStale, TunnelDefinition, TunnelKind};

const LOOPBACK_TARGETS: [&str; 2] = ["127.0.0.1", "localhost"];

/// Why a devslot tunnel no longer matches the listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevslotStaleReason {
    /// No instance lists the remote port of the tunnel.
    PortGone,
    /// The port now belongs to another project or another host name.
    Reassigned,
}

/// What to do with a stale devslot tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevslotStaleAction {
    /// Remove it without asking.
    Remove,
    /// Ask the user.
    Ask,
}

/// A devslot tunnel that no longer matches the listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevslotStaleTunnel {
    /// The tunnel as it was when the listing was analysed.
    pub definition: TunnelDefinition,
    /// Whether the tunnel lives only in the daemon.
    pub is_temporary: bool,
    /// Why it is stale.
    pub reason: DevslotStaleReason,
    /// What to do with it.
    pub action: DevslotStaleAction,
}

/// A tunnel of the engine, as the analysis sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnownTunnel {
    pub(crate) definition: TunnelDefinition,
    pub(crate) is_temporary: bool,
    pub(crate) origin: Option<DevslotOrigin>,
}

impl KnownTunnel {
    fn project_tag(&self) -> Option<&str> {
        if self.is_temporary {
            return self.origin.as_ref()?.project_tag.as_deref();
        }
        let mut project_tags = self
            .definition
            .tags
            .iter()
            .filter(|tag| tag.starts_with(PROJECT_TAG_PREFIX));
        let first = project_tags.next()?;
        project_tags.next().is_none().then_some(first.as_str())
    }

    fn remote_port(&self) -> u16 {
        self.definition.effective_target_port()
    }
}

fn is_local_loopback_forward(definition: &TunnelDefinition) -> bool {
    definition.kind == TunnelKind::Local
        && definition
            .target_host
            .as_deref()
            .is_none_or(|target| LOOPBACK_TARGETS.contains(&target))
}

fn is_forward_of_host(tunnel: &KnownTunnel, host: &str) -> bool {
    tunnel.definition.host == host && is_local_loopback_forward(&tunnel.definition)
}

/// Whether `tunnel` is a devslot tunnel of `host`: a local forward to the
/// loopback of the host that is either saved with the `devslot` tag or
/// temporary with a devslot origin.
pub(crate) fn is_devslot_tunnel_of(tunnel: &KnownTunnel, host: &str) -> bool {
    if !is_forward_of_host(tunnel, host) {
        return false;
    }
    if tunnel.is_temporary {
        return tunnel.origin.is_some();
    }
    tunnel.definition.tags.iter().any(|tag| tag == DEVSLOT_TAG)
}

struct PortIndex<'listing> {
    instances: &'listing [DevslotInstance],
    owners_by_port: HashMap<u16, Vec<usize>>,
}

impl<'listing> PortIndex<'listing> {
    fn new(instances: &'listing [DevslotInstance]) -> Self {
        let mut owners_by_port: HashMap<u16, Vec<usize>> = HashMap::new();
        for (position, instance) in instances.iter().enumerate() {
            for port in &instance.ports {
                let owners = owners_by_port.entry(port.port).or_default();
                if owners.last() != Some(&position) {
                    owners.push(position);
                }
            }
        }
        Self {
            instances,
            owners_by_port,
        }
    }

    fn owners(&self, port: u16) -> &[usize] {
        self.owners_by_port.get(&port).map_or(&[], Vec::as_slice)
    }
}

/// The devslot tunnels of `host` that no longer match `instances`, with what
/// to do with each, in the order of `known`.
///
/// A port listed by several instances (a forced port) is never touched. When
/// the listing is empty, or when every saved devslot tunnel of the host
/// would be removed, no saved tunnel is removed without asking.
pub(crate) fn find_stale_tunnels(
    host: &str,
    instances: &[DevslotInstance],
    known: &[KnownTunnel],
    policy: DevslotStale,
) -> Vec<DevslotStaleTunnel> {
    let index = PortIndex::new(instances);
    let assessed: Vec<(&KnownTunnel, Option<DevslotStaleReason>)> = known
        .iter()
        .filter(|tunnel| is_devslot_tunnel_of(tunnel, host))
        .map(|tunnel| (tunnel, find_reason(tunnel, &index)))
        .collect();
    let is_guarded = instances.is_empty() || would_remove_every_saved(&assessed, policy);
    assessed
        .into_iter()
        .filter_map(|(tunnel, reason)| {
            let reason = reason?;
            let action = if is_guarded && !tunnel.is_temporary {
                DevslotStaleAction::Ask
            } else {
                plan_action(tunnel, reason, policy)
            };
            Some(DevslotStaleTunnel {
                definition: tunnel.definition.clone(),
                is_temporary: tunnel.is_temporary,
                reason,
                action,
            })
        })
        .collect()
}

fn find_reason(tunnel: &KnownTunnel, index: &PortIndex<'_>) -> Option<DevslotStaleReason> {
    let port = tunnel.remote_port();
    match index.owners(port) {
        [] => Some(DevslotStaleReason::PortGone),
        [only] => is_reassigned(tunnel, &index.instances[*only], port)
            .then_some(DevslotStaleReason::Reassigned),
        _ => None,
    }
}

fn is_reassigned(tunnel: &KnownTunnel, instance: &DevslotInstance, port: u16) -> bool {
    let has_other_project = matches!(
        (tunnel.project_tag(), instance.project_tag.as_deref()),
        (Some(known), Some(listed)) if known != listed
    );
    let has_other_host_name = matches!(
        (tunnel.definition.open.host_name.as_deref(), listed_host_name(instance, port)),
        (Some(known), Some(listed)) if known != listed
    );
    has_other_project || has_other_host_name
}

fn listed_host_name(instance: &DevslotInstance, port: u16) -> Option<&str> {
    instance
        .ports
        .iter()
        .filter(|listed| listed.port == port)
        .find_map(|listed| listed.host_name.as_deref())
}

fn plan_action(
    tunnel: &KnownTunnel,
    reason: DevslotStaleReason,
    policy: DevslotStale,
) -> DevslotStaleAction {
    if tunnel.is_temporary {
        return DevslotStaleAction::Remove;
    }
    match (reason, policy) {
        (DevslotStaleReason::PortGone, DevslotStale::Delete) => DevslotStaleAction::Remove,
        _ => DevslotStaleAction::Ask,
    }
}

fn would_remove_every_saved(
    assessed: &[(&KnownTunnel, Option<DevslotStaleReason>)],
    policy: DevslotStale,
) -> bool {
    let mut saved = assessed.iter().filter(|(tunnel, _)| !tunnel.is_temporary);
    let Some(first) = saved.next() else {
        return false;
    };
    std::iter::once(first).chain(saved).all(|(tunnel, reason)| {
        reason
            .is_some_and(|reason| plan_action(tunnel, reason, policy) == DevslotStaleAction::Remove)
    })
}

/// Sets the status of every listed port against the tunnels of `host`.
pub(crate) fn assign_port_statuses(
    host: &str,
    instances: Vec<DevslotInstance>,
    known: &[KnownTunnel],
) -> Vec<DevslotInstance> {
    let statuses = port_statuses(host, &instances, known);
    instances
        .into_iter()
        .map(|instance| with_statuses(instance, &statuses))
        .collect()
}

fn port_statuses(
    host: &str,
    instances: &[DevslotInstance],
    known: &[KnownTunnel],
) -> HashMap<u16, DevslotPortStatus> {
    let index = PortIndex::new(instances);
    index
        .owners_by_port
        .iter()
        .map(|(port, owners)| (*port, status_of(host, *port, owners.len(), known)))
        .collect()
}

fn status_of(
    host: &str,
    port: u16,
    owner_count: usize,
    known: &[KnownTunnel],
) -> DevslotPortStatus {
    let tunneled = known
        .iter()
        .filter(|tunnel| is_forward_of_host(tunnel, host))
        .find(|tunnel| tunnel.remote_port() == port);
    if let Some(tunnel) = tunneled {
        return DevslotPortStatus::AlreadyTunneled {
            tunnel_id: tunnel.definition.id,
        };
    }
    if owner_count > 1 {
        return DevslotPortStatus::Ambiguous;
    }
    DevslotPortStatus::New
}

fn with_statuses(
    instance: DevslotInstance,
    statuses: &HashMap<u16, DevslotPortStatus>,
) -> DevslotInstance {
    let ports = instance
        .ports
        .into_iter()
        .map(|port| {
            let status = statuses.get(&port.port).copied().unwrap_or(port.status);
            super::listing::DevslotPort { status, ..port }
        })
        .collect();
    DevslotInstance { ports, ..instance }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devslot::listing::DevslotPort;
    use crate::store::{OpenOptions, TunnelId};
    use pretty_assertions::assert_eq;

    const HOST: &str = "vega";

    fn port(number: u16, host_name: Option<&str>) -> DevslotPort {
        DevslotPort {
            var: "PORT".to_owned(),
            port: number,
            kind: "web".to_owned(),
            url: None,
            host_name: host_name.map(str::to_owned),
            service: None,
            target: None,
            is_forced: false,
            status: DevslotPortStatus::New,
            is_local_port_busy: false,
            suggested_local_port: None,
        }
    }

    fn instance(name: &str, project_tag: Option<&str>, ports: Vec<DevslotPort>) -> DevslotInstance {
        DevslotInstance {
            name: name.to_owned(),
            project: None,
            project_tag: project_tag.map(str::to_owned),
            path: "/work".to_owned(),
            is_worktree: None,
            slot: 1,
            app_url: None,
            services_running: None,
            ports,
        }
    }

    fn definition(host: &str, target_port: u16, tags: &[&str]) -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            host: host.to_owned(),
            kind: TunnelKind::Local,
            listen_port: target_port,
            listen_address: Some("127.0.0.1".to_owned()),
            target_host: None,
            target_port: None,
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            open: OpenOptions::default(),
            standby: None,
        }
    }

    fn saved(port: u16, tags: &[&str]) -> KnownTunnel {
        KnownTunnel {
            definition: definition(HOST, port, tags),
            is_temporary: false,
            origin: None,
        }
    }

    fn saved_devslot(port: u16, project_tag: &str) -> KnownTunnel {
        saved(port, &["devslot", project_tag])
    }

    fn temporary(port: u16, project_tag: Option<&str>) -> KnownTunnel {
        KnownTunnel {
            definition: definition(HOST, port, &[]),
            is_temporary: true,
            origin: Some(DevslotOrigin {
                project_tag: project_tag.map(str::to_owned),
            }),
        }
    }

    fn actions(
        instances: &[DevslotInstance],
        known: &[KnownTunnel],
        policy: DevslotStale,
    ) -> Vec<(u16, DevslotStaleReason, DevslotStaleAction)> {
        find_stale_tunnels(HOST, instances, known, policy)
            .iter()
            .map(|stale| (stale.definition.listen_port, stale.reason, stale.action))
            .collect()
    }

    fn listing() -> Vec<DevslotInstance> {
        vec![instance(
            "links-a",
            Some("ds-links"),
            vec![port(20050, Some("links-a.localhost"))],
        )]
    }

    #[test]
    fn should_remove_a_temporary_tunnel_when_its_port_is_no_longer_listed() {
        let known = [temporary(20999, Some("ds-links"))];

        assert_eq!(
            actions(&listing(), &known, DevslotStale::Ask),
            [(
                20999,
                DevslotStaleReason::PortGone,
                DevslotStaleAction::Remove
            )]
        );
    }

    #[test]
    fn should_remove_a_temporary_tunnel_when_its_port_now_belongs_to_another_project() {
        let known = [temporary(20050, Some("ds-other"))];

        assert_eq!(
            actions(&listing(), &known, DevslotStale::Ask),
            [(
                20050,
                DevslotStaleReason::Reassigned,
                DevslotStaleAction::Remove
            )]
        );
    }

    #[test]
    fn should_ask_when_a_saved_tunnel_port_now_belongs_to_another_project() {
        let known = [
            saved_devslot(20050, "ds-other"),
            saved_devslot(20051, "ds-links"),
        ];
        let instances = vec![instance(
            "links-a",
            Some("ds-links"),
            vec![port(20050, None), port(20051, None)],
        )];

        assert_eq!(
            actions(&instances, &known, DevslotStale::Delete),
            [(
                20050,
                DevslotStaleReason::Reassigned,
                DevslotStaleAction::Ask
            )]
        );
    }

    #[test]
    fn should_ask_when_a_saved_tunnel_host_name_differs_from_the_listed_one() {
        let mut tunnel = saved_devslot(20050, "ds-links");
        tunnel.definition.open.host_name = Some("old-name.localhost".to_owned());
        let other = saved_devslot(20051, "ds-links");
        let instances = vec![instance(
            "links-a",
            Some("ds-links"),
            vec![port(20050, Some("links-a.localhost")), port(20051, None)],
        )];

        assert_eq!(
            actions(&instances, &[tunnel, other], DevslotStale::Ask),
            [(
                20050,
                DevslotStaleReason::Reassigned,
                DevslotStaleAction::Ask
            )]
        );
    }

    #[test]
    fn should_treat_the_tunnel_as_in_sync_when_the_project_is_unknown_on_one_side() {
        let known = [temporary(20050, None), saved(20050, &["devslot"])];
        let instances = vec![instance("a", None, vec![port(20050, None)])];

        assert_eq!(actions(&instances, &known, DevslotStale::Ask), []);
    }

    #[test]
    fn should_treat_the_tunnel_as_in_sync_when_the_host_name_is_unknown_on_one_side() {
        let mut tunnel = saved_devslot(20050, "ds-links");
        tunnel.definition.open.host_name = Some("links-a.localhost".to_owned());
        let instances = vec![instance("a", Some("ds-links"), vec![port(20050, None)])];

        assert_eq!(actions(&instances, &[tunnel], DevslotStale::Ask), []);
    }

    #[test]
    fn should_treat_the_tunnel_as_in_sync_when_the_project_and_host_name_match() {
        let mut tunnel = saved_devslot(20050, "ds-links");
        tunnel.definition.open.host_name = Some("links-a.localhost".to_owned());

        assert_eq!(actions(&listing(), &[tunnel], DevslotStale::Ask), []);
    }

    #[test]
    fn should_ask_when_a_saved_tunnel_port_is_gone_and_the_setting_is_ask() {
        let known = [
            saved_devslot(20999, "ds-links"),
            saved_devslot(20050, "ds-links"),
        ];

        assert_eq!(
            actions(&listing(), &known, DevslotStale::Ask),
            [(20999, DevslotStaleReason::PortGone, DevslotStaleAction::Ask)]
        );
    }

    #[test]
    fn should_remove_when_a_saved_tunnel_port_is_gone_and_the_setting_is_delete() {
        let known = [
            saved_devslot(20999, "ds-links"),
            saved_devslot(20050, "ds-links"),
        ];

        assert_eq!(
            actions(&listing(), &known, DevslotStale::Delete),
            [(
                20999,
                DevslotStaleReason::PortGone,
                DevslotStaleAction::Remove
            )]
        );
    }

    #[test]
    fn should_ask_for_every_saved_tunnel_when_the_listing_has_no_instance() {
        let known = [
            saved_devslot(20999, "ds-links"),
            saved_devslot(20998, "ds-links"),
        ];

        assert_eq!(
            actions(&[], &known, DevslotStale::Delete),
            [
                (20999, DevslotStaleReason::PortGone, DevslotStaleAction::Ask),
                (20998, DevslotStaleReason::PortGone, DevslotStaleAction::Ask)
            ]
        );
    }

    #[test]
    fn should_still_remove_a_temporary_tunnel_when_the_listing_has_no_instance() {
        let known = [temporary(20999, Some("ds-links"))];

        assert_eq!(
            actions(&[], &known, DevslotStale::Delete),
            [(
                20999,
                DevslotStaleReason::PortGone,
                DevslotStaleAction::Remove
            )]
        );
    }

    #[test]
    fn should_ask_for_every_saved_tunnel_when_all_of_them_would_be_removed() {
        let known = [
            saved_devslot(20999, "ds-links"),
            saved_devslot(20998, "ds-links"),
        ];

        assert_eq!(
            actions(&listing(), &known, DevslotStale::Delete),
            [
                (20999, DevslotStaleReason::PortGone, DevslotStaleAction::Ask),
                (20998, DevslotStaleReason::PortGone, DevslotStaleAction::Ask)
            ]
        );
    }

    #[test]
    fn should_keep_removing_a_temporary_tunnel_when_the_saved_ones_are_guarded() {
        let known = [saved_devslot(20999, "ds-links"), temporary(20998, None)];

        assert_eq!(
            actions(&listing(), &known, DevslotStale::Delete),
            [
                (20999, DevslotStaleReason::PortGone, DevslotStaleAction::Ask),
                (
                    20998,
                    DevslotStaleReason::PortGone,
                    DevslotStaleAction::Remove
                )
            ]
        );
    }

    #[test]
    fn should_never_touch_a_tunnel_when_several_instances_list_its_port() {
        let instances = vec![
            instance("a", Some("ds-a"), vec![port(3000, None)]),
            instance("b", Some("ds-b"), vec![port(3000, None)]),
        ];
        let known = [
            saved_devslot(3000, "ds-other"),
            temporary(3000, Some("ds-other")),
        ];

        assert_eq!(actions(&instances, &known, DevslotStale::Delete), []);
    }

    #[test]
    fn should_count_one_instance_when_it_lists_the_same_port_twice() {
        let instances = vec![instance(
            "a",
            Some("ds-links"),
            vec![port(3000, None), port(3000, None)],
        )];
        let known = [temporary(3000, Some("ds-other"))];

        assert_eq!(
            actions(&instances, &known, DevslotStale::Ask),
            [(
                3000,
                DevslotStaleReason::Reassigned,
                DevslotStaleAction::Remove
            )]
        );
    }

    #[test]
    fn should_never_consider_a_tunnel_of_another_host() {
        let mut other_host = saved_devslot(20999, "ds-links");
        other_host.definition.host = "elsewhere".to_owned();

        assert_eq!(actions(&listing(), &[other_host], DevslotStale::Delete), []);
    }

    #[test]
    fn should_never_consider_a_saved_tunnel_without_the_devslot_tag() {
        let known = [saved(20999, &["ds-links"]), saved(20998, &[])];

        assert_eq!(actions(&listing(), &known, DevslotStale::Delete), []);
    }

    #[test]
    fn should_never_consider_a_temporary_tunnel_without_an_origin() {
        let mut plain = temporary(20999, None);
        plain.origin = None;

        assert_eq!(actions(&listing(), &[plain], DevslotStale::Delete), []);
    }

    #[test]
    fn should_never_consider_a_tunnel_that_is_not_a_local_forward_to_loopback() {
        let mut remote = saved_devslot(20999, "ds-links");
        remote.definition.kind = TunnelKind::Remote;
        let mut elsewhere = saved_devslot(20998, "ds-links");
        elsewhere.definition.target_host = Some("db.internal".to_owned());

        assert_eq!(
            actions(&listing(), &[remote, elsewhere], DevslotStale::Delete),
            []
        );
    }

    #[test]
    fn should_consider_a_tunnel_that_targets_localhost() {
        let mut tunnel = temporary(20999, None);
        tunnel.definition.target_host = Some("localhost".to_owned());

        assert_eq!(actions(&listing(), &[tunnel], DevslotStale::Ask).len(), 1);
    }

    #[test]
    fn should_use_the_target_port_as_the_remote_port_when_it_differs_from_the_listen_port() {
        let mut tunnel = saved_devslot(30000, "ds-links");
        tunnel.definition.target_port = Some(20050);
        let other = saved_devslot(20999, "ds-links");

        assert_eq!(
            actions(&listing(), &[tunnel, other], DevslotStale::Ask),
            [(20999, DevslotStaleReason::PortGone, DevslotStaleAction::Ask)]
        );
    }

    #[test]
    fn should_read_the_project_of_a_saved_tunnel_from_its_only_ds_tag() {
        let mut tunnel = saved(20050, &["devslot", "mine", "ds-other"]);
        tunnel.definition.listen_port = 20050;
        let sibling = saved_devslot(20051, "ds-links");
        let instances = vec![instance(
            "a",
            Some("ds-links"),
            vec![port(20050, None), port(20051, None)],
        )];

        assert_eq!(
            actions(&instances, &[tunnel, sibling], DevslotStale::Ask)
                .iter()
                .map(|(_, reason, _)| *reason)
                .collect::<Vec<_>>(),
            [DevslotStaleReason::Reassigned]
        );
    }

    #[test]
    fn should_ignore_the_project_tag_of_a_saved_tunnel_when_it_has_several() {
        let tunnel = saved(20050, &["devslot", "ds-one", "ds-two"]);
        let sibling = saved_devslot(20051, "ds-links");
        let instances = vec![instance(
            "a",
            Some("ds-links"),
            vec![port(20050, None), port(20051, None)],
        )];

        assert_eq!(
            actions(&instances, &[tunnel, sibling], DevslotStale::Ask),
            []
        );
    }

    fn status_of_port(
        instances: Vec<DevslotInstance>,
        known: &[KnownTunnel],
        wanted: u16,
    ) -> DevslotPortStatus {
        assign_port_statuses(HOST, instances, known)
            .iter()
            .flat_map(|instance| &instance.ports)
            .find(|listed| listed.port == wanted)
            .map(|listed| listed.status)
            .unwrap()
    }

    #[test]
    fn should_report_a_port_as_new_when_nothing_forwards_to_it() {
        assert_eq!(
            status_of_port(listing(), &[], 20050),
            DevslotPortStatus::New
        );
    }

    #[test]
    fn should_report_the_tunnel_when_a_devslot_tunnel_already_forwards_to_the_port() {
        let tunnel = saved_devslot(20050, "ds-links");
        let id = tunnel.definition.id;

        assert_eq!(
            status_of_port(listing(), &[tunnel], 20050),
            DevslotPortStatus::AlreadyTunneled { tunnel_id: id }
        );
    }

    #[test]
    fn should_report_the_tunnel_when_a_tunnel_without_the_devslot_tag_forwards_to_the_port() {
        let tunnel = saved(20050, &[]);
        let id = tunnel.definition.id;

        assert_eq!(
            status_of_port(listing(), &[tunnel], 20050),
            DevslotPortStatus::AlreadyTunneled { tunnel_id: id }
        );
    }

    #[test]
    fn should_report_the_tunnel_when_a_temporary_tunnel_without_origin_forwards_to_the_port() {
        let mut tunnel = temporary(20050, None);
        tunnel.origin = None;
        let id = tunnel.definition.id;

        assert_eq!(
            status_of_port(listing(), &[tunnel], 20050),
            DevslotPortStatus::AlreadyTunneled { tunnel_id: id }
        );
    }

    #[test]
    fn should_report_a_port_as_new_when_the_tunnel_belongs_to_another_host() {
        let mut tunnel = saved_devslot(20050, "ds-links");
        tunnel.definition.host = "elsewhere".to_owned();

        assert_eq!(
            status_of_port(listing(), &[tunnel], 20050),
            DevslotPortStatus::New
        );
    }

    #[test]
    fn should_report_a_port_as_new_when_the_tunnel_does_not_target_the_loopback() {
        let mut tunnel = saved(20050, &[]);
        tunnel.definition.target_host = Some("db.internal".to_owned());

        assert_eq!(
            status_of_port(listing(), &[tunnel], 20050),
            DevslotPortStatus::New
        );
    }

    #[test]
    fn should_report_a_port_as_ambiguous_when_several_instances_list_it() {
        let instances = vec![
            instance("a", None, vec![port(3000, None)]),
            instance("b", None, vec![port(3000, None)]),
        ];

        assert_eq!(
            status_of_port(instances, &[], 3000),
            DevslotPortStatus::Ambiguous
        );
    }

    #[test]
    fn should_report_the_tunnel_rather_than_ambiguous_when_a_tunnel_forwards_to_the_port() {
        let instances = vec![
            instance("a", None, vec![port(3000, None)]),
            instance("b", None, vec![port(3000, None)]),
        ];
        let tunnel = saved(3000, &[]);
        let id = tunnel.definition.id;

        assert_eq!(
            status_of_port(instances, &[tunnel], 3000),
            DevslotPortStatus::AlreadyTunneled { tunnel_id: id }
        );
    }

    #[test]
    fn should_never_mark_a_tunnel_without_the_devslot_tag_stale_when_it_shares_a_listed_port() {
        let tunnel = saved(20999, &[]);

        assert_eq!(actions(&listing(), &[tunnel], DevslotStale::Delete), []);
    }
}
