//! What opening one devslot instance starts, reuses and leaves alone.
//!
//! Pure, so that the rules are tested without a host. It is the Rust twin of
//! the spec building of the import tab, for the web ports of one instance.

use std::collections::HashSet;

use super::analysis::{DevslotStaleTunnel, KnownTunnel};
use super::listing::{DevslotInstance, DevslotPort, DevslotPortStatus};
use super::origin::DevslotOrigin;
use super::up_outcome::{DevslotSkipReason, DevslotSkippedPort, DevslotUpPort};
use crate::runtime::NewTunnel;
use crate::store::{OpenOptions, TunnelId, TunnelKind};

const WEB_PORT_KIND: &str = "web";
const PRIVILEGED_PORT_LIMIT: u16 = 1024;
const LISTEN_ADDRESS: &str = "127.0.0.1";

/// What the plan needs to know.
pub(crate) struct UpPlanInput<'input> {
    /// The host alias.
    pub(crate) host: &'input str,
    /// The instance to open, analysed.
    pub(crate) instance: &'input DevslotInstance,
    /// Every instance of the listing, analysed: a port that several of them
    /// list is shared.
    pub(crate) all_instances: &'input [DevslotInstance],
    /// Whether new tunnels are saved rather than temporary.
    pub(crate) is_persisted: bool,
    /// The stale devslot tunnels that are still there.
    pub(crate) stale: &'input [DevslotStaleTunnel],
    /// The tunnels of the engine.
    pub(crate) known: &'input [KnownTunnel],
}

/// How a new tunnel lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UpTunnelKind {
    /// It lives in the daemon only, marked with its devslot origin.
    Temporary(DevslotOrigin),
    /// It is saved to the store, with its tags.
    Saved,
}

/// A tunnel to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedTunnel {
    pub(crate) remote_port: u16,
    pub(crate) tunnel: NewTunnel,
    pub(crate) kind: UpTunnelKind,
}

/// What to do for the web ports of one instance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DevslotUpPlan {
    /// The tunnels to create, in port order of the listing.
    pub(crate) to_start: Vec<PlannedTunnel>,
    /// The ports that already have a tunnel.
    pub(crate) existing: Vec<DevslotUpPort>,
    /// The ports left alone.
    pub(crate) skipped: Vec<DevslotSkippedPort>,
    /// The remote port of the application, when it is tunneled.
    pub(crate) app_port: Option<u16>,
    /// Whether another instance lists the port of the application.
    pub(crate) is_app_port_shared: bool,
}

enum Decision {
    Start(PlannedTunnel),
    Reuse(DevslotUpPort),
    Skip(DevslotSkippedPort),
}

/// Plans the web ports of `input.instance`: those that are new or ambiguous
/// are started, those that have a tunnel are reused unless a stale tunnel
/// holds them, and the others are left alone.
pub(crate) fn plan_devslot_up(input: &UpPlanInput<'_>) -> DevslotUpPlan {
    let mut seen = HashSet::new();
    let mut plan = DevslotUpPlan::default();
    for port in input
        .instance
        .ports
        .iter()
        .filter(|port| port.kind == WEB_PORT_KIND)
        .filter(|port| seen.insert(port.port))
    {
        match decide(port, input) {
            Decision::Start(planned) => plan.to_start.push(planned),
            Decision::Reuse(existing) => plan.existing.push(existing),
            Decision::Skip(skipped) => plan.skipped.push(skipped),
        }
    }
    plan.app_port = find_app_port(input, &plan);
    plan.is_app_port_shared = plan
        .app_port
        .is_some_and(|port| is_port_shared(input.all_instances, port));
    plan
}

fn decide(port: &DevslotPort, input: &UpPlanInput<'_>) -> Decision {
    if let DevslotPortStatus::AlreadyTunneled { tunnel_id } = port.status
        && let Some(decision) = reuse_or_review(port, tunnel_id, input)
    {
        return decision;
    }
    start_or_skip(port, input)
}

fn reuse_or_review(
    port: &DevslotPort,
    tunnel_id: TunnelId,
    input: &UpPlanInput<'_>,
) -> Option<Decision> {
    if input
        .stale
        .iter()
        .any(|stale| stale.definition.id == tunnel_id)
    {
        return Some(Decision::Skip(skip(port, DevslotSkipReason::NeedsReview)));
    }
    let tunnel = input
        .known
        .iter()
        .find(|tunnel| tunnel.definition.id == tunnel_id)?;
    Some(Decision::Reuse(DevslotUpPort {
        remote_port: port.port,
        listen_port: tunnel.definition.listen_port,
        tunnel_id,
    }))
}

fn start_or_skip(port: &DevslotPort, input: &UpPlanInput<'_>) -> Decision {
    if port.port < PRIVILEGED_PORT_LIMIT {
        return Decision::Skip(skip(port, DevslotSkipReason::PrivilegedPort));
    }
    let Some(listen_port) = choose_listen_port(port) else {
        return Decision::Skip(skip(port, DevslotSkipReason::LocalPortBusy));
    };
    Decision::Start(plan_tunnel(port, listen_port, input))
}

fn skip(port: &DevslotPort, reason: DevslotSkipReason) -> DevslotSkippedPort {
    DevslotSkippedPort {
        remote_port: port.port,
        reason,
    }
}

fn choose_listen_port(port: &DevslotPort) -> Option<u16> {
    if port.is_local_port_busy {
        return port.suggested_local_port;
    }
    Some(port.port)
}

fn plan_tunnel(port: &DevslotPort, listen_port: u16, input: &UpPlanInput<'_>) -> PlannedTunnel {
    let origin = DevslotOrigin {
        project_tag: input.instance.project_tag.clone(),
    };
    let (tags, kind) = if input.is_persisted {
        (origin.tags(), UpTunnelKind::Saved)
    } else {
        (Vec::new(), UpTunnelKind::Temporary(origin))
    };
    PlannedTunnel {
        remote_port: port.port,
        tunnel: NewTunnel {
            host: input.host.to_owned(),
            kind: TunnelKind::Local,
            listen_port,
            listen_address: Some(LISTEN_ADDRESS.to_owned()),
            target_host: None,
            target_port: (port.port != listen_port).then_some(port.port),
            tags,
            open: OpenOptions {
                host_name: port.host_name.clone(),
                ..OpenOptions::default()
            },
            standby: None,
        },
        kind,
    }
}

fn find_app_port(input: &UpPlanInput<'_>, plan: &DevslotUpPlan) -> Option<u16> {
    let tunneled: Vec<u16> = plan
        .to_start
        .iter()
        .map(|planned| planned.remote_port)
        .chain(plan.existing.iter().map(|existing| existing.remote_port))
        .collect();
    let from_url = input
        .instance
        .app_url
        .as_deref()
        .and_then(find_url_port)
        .filter(|port| tunneled.contains(port));
    from_url.or_else(|| find_only_web_port(input.instance, &tunneled))
}

fn find_only_web_port(instance: &DevslotInstance, tunneled: &[u16]) -> Option<u16> {
    let web_ports: HashSet<u16> = instance
        .ports
        .iter()
        .filter(|port| port.kind == WEB_PORT_KIND)
        .map(|port| port.port)
        .collect();
    let mut ports = web_ports.into_iter();
    let only = ports.next()?;
    (ports.next().is_none() && tunneled.contains(&only)).then_some(only)
}

fn find_url_port(url: &str) -> Option<u16> {
    let without_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = without_scheme.split(['/', '?', '#']).next()?;
    let (_, port) = authority.rsplit_once(':')?;
    port.parse().ok()
}

fn is_port_shared(instances: &[DevslotInstance], port: u16) -> bool {
    instances
        .iter()
        .filter(|instance| instance.ports.iter().any(|listed| listed.port == port))
        .count()
        > 1
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::devslot::analysis::{DevslotStaleAction, DevslotStaleReason};
    use crate::store::TunnelDefinition;

    const HOST: &str = "vega";
    const APP_PORT: u16 = 20050;

    fn web_port(number: u16) -> DevslotPort {
        DevslotPort {
            var: "PORT".to_owned(),
            port: number,
            kind: "web".to_owned(),
            url: Some(format!("http://web-shop-098a.localhost:{number}")),
            host_name: Some("web-shop-098a.localhost".to_owned()),
            service: None,
            target: None,
            is_forced: false,
            status: DevslotPortStatus::New,
            is_local_port_busy: false,
            suggested_local_port: None,
        }
    }

    fn service_port(number: u16) -> DevslotPort {
        DevslotPort {
            kind: "service".to_owned(),
            url: None,
            host_name: None,
            service: Some("postgres".to_owned()),
            target: Some(5432),
            ..web_port(number)
        }
    }

    fn busy(port: DevslotPort, suggestion: Option<u16>) -> DevslotPort {
        DevslotPort {
            is_local_port_busy: true,
            suggested_local_port: suggestion,
            ..port
        }
    }

    fn with_status(port: DevslotPort, status: DevslotPortStatus) -> DevslotPort {
        DevslotPort { status, ..port }
    }

    fn instance(ports: Vec<DevslotPort>) -> DevslotInstance {
        DevslotInstance {
            name: "web-shop-098a".to_owned(),
            project: Some("web-shop".to_owned()),
            project_tag: Some("ds-web-shop".to_owned()),
            path: "/work/web-shop".to_owned(),
            is_worktree: Some(false),
            slot: 1,
            app_url: Some(format!("http://web-shop-098a.localhost:{APP_PORT}")),
            services_running: Some(true),
            ports,
        }
    }

    fn known_tunnel(listen_port: u16, remote_port: u16) -> KnownTunnel {
        KnownTunnel {
            definition: TunnelDefinition {
                id: TunnelId::generate(),
                host: HOST.to_owned(),
                kind: TunnelKind::Local,
                listen_port,
                listen_address: Some(LISTEN_ADDRESS.to_owned()),
                target_host: None,
                target_port: Some(remote_port),
                tags: vec!["devslot".to_owned()],
                open: OpenOptions::default(),
                standby: None,
            },
            is_temporary: false,
            origin: None,
        }
    }

    fn stale_for(tunnel: &KnownTunnel) -> DevslotStaleTunnel {
        DevslotStaleTunnel {
            definition: tunnel.definition.clone(),
            is_temporary: false,
            reason: DevslotStaleReason::Reassigned,
            action: DevslotStaleAction::Ask,
        }
    }

    fn plan_with(
        instances: &[DevslotInstance],
        is_persisted: bool,
        stale: &[DevslotStaleTunnel],
        known: &[KnownTunnel],
    ) -> DevslotUpPlan {
        plan_devslot_up(&UpPlanInput {
            host: HOST,
            instance: &instances[0],
            all_instances: instances,
            is_persisted,
            stale,
            known,
        })
    }

    fn plan_of(ports: Vec<DevslotPort>) -> DevslotUpPlan {
        plan_with(&[instance(ports)], false, &[], &[])
    }

    fn started_ports(plan: &DevslotUpPlan) -> Vec<(u16, u16)> {
        plan.to_start
            .iter()
            .map(|planned| (planned.remote_port, planned.tunnel.listen_port))
            .collect()
    }

    #[test]
    fn should_start_the_web_ports_and_leave_the_service_ports_alone() {
        let plan = plan_of(vec![web_port(APP_PORT), service_port(20052)]);

        assert_eq!(
            (started_ports(&plan), plan.skipped),
            (vec![(APP_PORT, APP_PORT)], Vec::new())
        );
    }

    #[test]
    fn should_start_an_ambiguous_port() {
        let ambiguous = with_status(web_port(APP_PORT), DevslotPortStatus::Ambiguous);

        assert_eq!(
            started_ports(&plan_of(vec![ambiguous])),
            vec![(APP_PORT, APP_PORT)]
        );
    }

    #[test]
    fn should_skip_a_port_below_1024_as_privileged() {
        let plan = plan_of(vec![web_port(80), web_port(APP_PORT)]);

        assert_eq!(
            (started_ports(&plan), plan.skipped),
            (
                vec![(APP_PORT, APP_PORT)],
                vec![DevslotSkippedPort {
                    remote_port: 80,
                    reason: DevslotSkipReason::PrivilegedPort
                }]
            )
        );
    }

    #[test]
    fn should_start_a_port_of_1024_exactly() {
        assert_eq!(
            started_ports(&plan_of(vec![web_port(1024)])),
            vec![(1024, 1024)]
        );
    }

    #[test]
    fn should_listen_on_the_suggestion_and_target_the_remote_port_when_the_port_is_busy() {
        let plan = plan_of(vec![busy(web_port(APP_PORT), Some(30050))]);

        let planned = &plan.to_start[0];

        assert_eq!(
            (planned.tunnel.listen_port, planned.tunnel.target_port),
            (30050, Some(APP_PORT))
        );
    }

    #[test]
    fn should_leave_the_target_port_out_when_it_equals_the_listen_port() {
        let plan = plan_of(vec![web_port(APP_PORT)]);

        assert_eq!(plan.to_start[0].tunnel.target_port, None);
    }

    #[test]
    fn should_skip_a_busy_port_without_suggestion() {
        let plan = plan_of(vec![busy(web_port(APP_PORT), None)]);

        assert_eq!(
            (plan.to_start.len(), plan.skipped),
            (
                0,
                vec![DevslotSkippedPort {
                    remote_port: APP_PORT,
                    reason: DevslotSkipReason::LocalPortBusy
                }]
            )
        );
    }

    #[test]
    fn should_describe_the_tunnel_like_the_import_tab_does() {
        let plan = plan_of(vec![web_port(APP_PORT)]);

        let tunnel = &plan.to_start[0].tunnel;

        assert_eq!(
            (
                tunnel.host.as_str(),
                tunnel.kind,
                tunnel.listen_address.as_deref(),
                tunnel.target_host.as_deref(),
                tunnel.open.host_name.as_deref(),
                tunnel.standby
            ),
            (
                HOST,
                TunnelKind::Local,
                Some("127.0.0.1"),
                None,
                Some("web-shop-098a.localhost"),
                None
            )
        );
    }

    #[test]
    fn should_make_a_temporary_tunnel_without_tags_and_with_the_origin_by_default() {
        let planned = &plan_of(vec![web_port(APP_PORT)]).to_start[0];

        assert_eq!(
            (planned.tunnel.tags.clone(), planned.kind.clone()),
            (
                Vec::new(),
                UpTunnelKind::Temporary(DevslotOrigin {
                    project_tag: Some("ds-web-shop".to_owned())
                })
            )
        );
    }

    #[test]
    fn should_make_a_saved_tunnel_with_the_devslot_and_project_tags_when_persisting() {
        let plan = plan_with(&[instance(vec![web_port(APP_PORT)])], true, &[], &[]);

        let planned = &plan.to_start[0];

        assert_eq!(
            (planned.tunnel.tags.clone(), planned.kind.clone()),
            (
                vec!["devslot".to_owned(), "ds-web-shop".to_owned()],
                UpTunnelKind::Saved
            )
        );
    }

    #[test]
    fn should_make_a_saved_tunnel_with_the_devslot_tag_alone_when_the_instance_has_no_project() {
        let mut listed = instance(vec![web_port(APP_PORT)]);
        listed.project_tag = None;

        let plan = plan_with(&[listed], true, &[], &[]);

        assert_eq!(plan.to_start[0].tunnel.tags, ["devslot"]);
    }

    #[test]
    fn should_reuse_a_port_that_already_has_a_tunnel() {
        let existing = known_tunnel(31000, APP_PORT);
        let tunneled = with_status(
            web_port(APP_PORT),
            DevslotPortStatus::AlreadyTunneled {
                tunnel_id: existing.definition.id,
            },
        );

        let plan = plan_with(
            &[instance(vec![tunneled])],
            false,
            &[],
            std::slice::from_ref(&existing),
        );

        assert_eq!(
            (plan.to_start.len(), plan.existing),
            (
                0,
                vec![DevslotUpPort {
                    remote_port: APP_PORT,
                    listen_port: 31000,
                    tunnel_id: existing.definition.id
                }]
            )
        );
    }

    #[test]
    fn should_skip_a_port_whose_tunnel_is_stale_and_needs_review() {
        let existing = known_tunnel(APP_PORT, APP_PORT);
        let tunneled = with_status(
            web_port(APP_PORT),
            DevslotPortStatus::AlreadyTunneled {
                tunnel_id: existing.definition.id,
            },
        );
        let stale = stale_for(&existing);

        let plan = plan_with(&[instance(vec![tunneled])], false, &[stale], &[existing]);

        assert_eq!(
            (
                plan.to_start.len(),
                plan.existing.len(),
                plan.skipped,
                plan.app_port
            ),
            (
                0,
                0,
                vec![DevslotSkippedPort {
                    remote_port: APP_PORT,
                    reason: DevslotSkipReason::NeedsReview
                }],
                None
            )
        );
    }

    #[test]
    fn should_pick_the_port_of_the_app_url_as_the_app_port() {
        let plan = plan_of(vec![web_port(20051), web_port(APP_PORT)]);

        assert_eq!(plan.app_port, Some(APP_PORT));
    }

    #[test]
    fn should_pick_the_only_web_port_when_the_app_url_names_no_listed_port() {
        let mut listed = instance(vec![web_port(20051), service_port(20052)]);
        listed.app_url = Some("http://web-shop-098a.localhost:9".to_owned());

        let plan = plan_with(&[listed], false, &[], &[]);

        assert_eq!(plan.app_port, Some(20051));
    }

    #[test]
    fn should_pick_no_app_port_when_the_app_url_names_none_and_there_are_several_web_ports() {
        let mut listed = instance(vec![web_port(20051), web_port(20053)]);
        listed.app_url = None;

        let plan = plan_with(&[listed], false, &[], &[]);

        assert_eq!(plan.app_port, None);
    }

    #[test]
    fn should_pick_no_app_port_when_the_instance_has_no_web_port() {
        assert_eq!(plan_of(vec![service_port(20052)]).app_port, None);
    }

    #[test]
    fn should_pick_no_app_port_when_the_port_of_the_app_url_is_skipped() {
        let plan = plan_of(vec![busy(web_port(APP_PORT), None), web_port(20051)]);

        assert_eq!(plan.app_port, None);
    }

    #[test]
    fn should_read_the_port_of_urls_with_a_path_a_query_or_no_scheme() {
        assert_eq!(
            [
                find_url_port("http://a.localhost:20050/"),
                find_url_port("http://a.localhost:20050/x?y=1:2"),
                find_url_port("a.localhost:20050"),
                find_url_port("http://a.localhost"),
                find_url_port("http://[::1]:8080/")
            ],
            [Some(20050), Some(20050), Some(20050), None, Some(8080)]
        );
    }

    #[test]
    fn should_flag_the_app_port_as_shared_when_another_instance_lists_it() {
        let mut other = instance(vec![web_port(APP_PORT)]);
        other.name = "web-shop-feat-b-8238".to_owned();
        let target = instance(vec![web_port(APP_PORT)]);

        let plan = plan_with(&[target, other], false, &[], &[]);

        assert!(plan.is_app_port_shared);
    }

    #[test]
    fn should_not_flag_the_app_port_as_shared_when_no_other_instance_lists_it() {
        let target = instance(vec![web_port(APP_PORT)]);
        let mut other = instance(vec![web_port(20060)]);
        other.name = "web-shop-feat-b-8238".to_owned();

        let plan = plan_with(&[target, other], false, &[], &[]);

        assert!(!plan.is_app_port_shared);
    }

    #[test]
    fn should_plan_a_port_listed_twice_once() {
        let plan = plan_of(vec![web_port(APP_PORT), web_port(APP_PORT)]);

        assert_eq!(started_ports(&plan), vec![(APP_PORT, APP_PORT)]);
    }
}
