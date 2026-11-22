//! Finding the listed ports that something else already holds on this
//! machine, and a free local port to use instead.

use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, TcpListener};

use super::analysis::KnownTunnel;
use super::listing::{DevslotInstance, DevslotPort, DevslotPortStatus};

/// How far above the remote port the first suggestion is.
pub(crate) const SUGGESTED_PORT_OFFSET: u32 = 10_000;

/// How many ports are tried, from the first suggestion upward, before giving
/// up.
pub(crate) const SUGGESTION_ATTEMPTS: u32 = 64;

const HIGHEST_PORT: u32 = 65_535;

/// Tells whether a port of the loopback of this machine is free.
pub(crate) trait LocalPortProbe {
    fn is_free(&self, port: u16) -> bool;
}

/// Probes by binding a listener, which is dropped at once. Nothing connects
/// and nothing leaves the machine.
pub(crate) struct BindProbe;

impl LocalPortProbe for BindProbe {
    fn is_free(&self, port: u16) -> bool {
        TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok()
    }
}

/// Marks the ports that are busy locally and suggests another local port for
/// each. Ports already tunneled are never probed.
pub(crate) fn assign_local_ports(
    instances: Vec<DevslotInstance>,
    known: &[KnownTunnel],
    probe: &dyn LocalPortProbe,
) -> Vec<DevslotInstance> {
    let mut finder = SuggestionFinder::new(&instances, known, probe);
    instances
        .into_iter()
        .map(|instance| {
            let ports = instance
                .ports
                .into_iter()
                .map(|port| with_local_port(port, &mut finder))
                .collect();
            DevslotInstance { ports, ..instance }
        })
        .collect()
}

fn with_local_port(port: DevslotPort, finder: &mut SuggestionFinder<'_>) -> DevslotPort {
    if matches!(port.status, DevslotPortStatus::AlreadyTunneled { .. }) {
        return port;
    }
    match finder.check(port.port) {
        LocalPort::Free => port,
        LocalPort::Busy { suggestion } => DevslotPort {
            is_local_port_busy: true,
            suggested_local_port: suggestion,
            ..port
        },
    }
}

#[derive(Clone, Copy)]
enum LocalPort {
    Free,
    Busy { suggestion: Option<u16> },
}

struct SuggestionFinder<'context> {
    probe: &'context dyn LocalPortProbe,
    listed: HashSet<u16>,
    tunnel_listen_ports: HashSet<u16>,
    suggested: HashSet<u16>,
    checked: HashMap<u16, LocalPort>,
}

impl<'context> SuggestionFinder<'context> {
    fn new(
        instances: &[DevslotInstance],
        known: &[KnownTunnel],
        probe: &'context dyn LocalPortProbe,
    ) -> Self {
        Self {
            probe,
            listed: instances
                .iter()
                .flat_map(|instance| instance.ports.iter().map(|port| port.port))
                .collect(),
            tunnel_listen_ports: known
                .iter()
                .map(|tunnel| tunnel.definition.listen_port)
                .collect(),
            suggested: HashSet::new(),
            checked: HashMap::new(),
        }
    }

    fn check(&mut self, port: u16) -> LocalPort {
        if let Some(known) = self.checked.get(&port) {
            return *known;
        }
        let outcome = if self.probe.is_free(port) {
            LocalPort::Free
        } else {
            LocalPort::Busy {
                suggestion: self.suggest(port),
            }
        };
        self.checked.insert(port, outcome);
        outcome
    }

    fn suggest(&mut self, port: u16) -> Option<u16> {
        let first = u32::from(port) + SUGGESTED_PORT_OFFSET;
        let candidate = (first..first + SUGGESTION_ATTEMPTS)
            .take_while(|candidate| *candidate <= HIGHEST_PORT)
            .filter_map(|candidate| u16::try_from(candidate).ok())
            .find(|candidate| self.is_usable(*candidate))?;
        self.suggested.insert(candidate);
        Some(candidate)
    }

    fn is_usable(&self, candidate: u16) -> bool {
        !self.listed.contains(&candidate)
            && !self.tunnel_listen_ports.contains(&candidate)
            && !self.suggested.contains(&candidate)
            && self.probe.is_free(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{OpenOptions, TunnelDefinition, TunnelId, TunnelKind};
    use pretty_assertions::assert_eq;
    use std::cell::RefCell;

    struct FakeProbe {
        busy: HashSet<u16>,
        probed: RefCell<Vec<u16>>,
    }

    impl FakeProbe {
        fn with_busy(busy: &[u16]) -> Self {
            Self {
                busy: busy.iter().copied().collect(),
                probed: RefCell::new(Vec::new()),
            }
        }
    }

    impl LocalPortProbe for FakeProbe {
        fn is_free(&self, port: u16) -> bool {
            self.probed.borrow_mut().push(port);
            !self.busy.contains(&port)
        }
    }

    fn port(number: u16, status: DevslotPortStatus) -> DevslotPort {
        DevslotPort {
            var: "PORT".to_owned(),
            port: number,
            kind: "web".to_owned(),
            url: None,
            host_name: None,
            service: None,
            target: None,
            is_forced: false,
            status,
            is_local_port_busy: false,
            suggested_local_port: None,
        }
    }

    fn instance(ports: Vec<DevslotPort>) -> DevslotInstance {
        DevslotInstance {
            name: "a".to_owned(),
            project: None,
            project_tag: None,
            path: "/a".to_owned(),
            is_worktree: None,
            slot: 1,
            app_url: None,
            services_running: None,
            ports,
        }
    }

    fn tunnel_listening_on(listen_port: u16) -> KnownTunnel {
        KnownTunnel {
            definition: TunnelDefinition {
                id: TunnelId::generate(),
                host: "other".to_owned(),
                kind: TunnelKind::Local,
                listen_port,
                listen_address: None,
                target_host: None,
                target_port: Some(1),
                tags: Vec::new(),
                open: OpenOptions::default(),
                standby: None,
            },
            is_temporary: false,
            origin: None,
        }
    }

    fn outcome(
        ports: Vec<DevslotPort>,
        known: &[KnownTunnel],
        probe: &FakeProbe,
    ) -> Vec<(u16, bool, Option<u16>)> {
        assign_local_ports(vec![instance(ports)], known, probe)[0]
            .ports
            .iter()
            .map(|listed| {
                (
                    listed.port,
                    listed.is_local_port_busy,
                    listed.suggested_local_port,
                )
            })
            .collect()
    }

    fn new_port(number: u16) -> DevslotPort {
        port(number, DevslotPortStatus::New)
    }

    #[test]
    fn should_leave_a_free_port_without_busy_flag_or_suggestion() {
        assert_eq!(
            outcome(vec![new_port(20050)], &[], &FakeProbe::with_busy(&[])),
            [(20050, false, None)]
        );
    }

    #[test]
    fn should_suggest_the_port_ten_thousand_above_when_the_port_is_busy() {
        assert_eq!(
            outcome(vec![new_port(20050)], &[], &FakeProbe::with_busy(&[20050])),
            [(20050, true, Some(30050))]
        );
    }

    #[test]
    fn should_treat_an_ambiguous_port_like_a_new_one() {
        let ambiguous = port(20050, DevslotPortStatus::Ambiguous);

        assert_eq!(
            outcome(vec![ambiguous], &[], &FakeProbe::with_busy(&[20050])),
            [(20050, true, Some(30050))]
        );
    }

    #[test]
    fn should_skip_a_suggestion_that_is_busy() {
        assert_eq!(
            outcome(
                vec![new_port(20050)],
                &[],
                &FakeProbe::with_busy(&[20050, 30050])
            ),
            [(20050, true, Some(30051))]
        );
    }

    #[test]
    fn should_skip_a_suggestion_that_an_existing_tunnel_listens_on() {
        assert_eq!(
            outcome(
                vec![new_port(20050)],
                &[tunnel_listening_on(30050)],
                &FakeProbe::with_busy(&[20050])
            ),
            [(20050, true, Some(30051))]
        );
    }

    #[test]
    fn should_skip_a_suggestion_that_is_another_listed_remote_port() {
        assert_eq!(
            outcome(
                vec![new_port(20050), new_port(30050)],
                &[],
                &FakeProbe::with_busy(&[20050])
            ),
            [(20050, true, Some(30051)), (30050, false, None)]
        );
    }

    #[test]
    fn should_skip_a_suggestion_already_given_to_another_port() {
        assert_eq!(
            outcome(
                vec![new_port(20050), new_port(20051)],
                &[],
                &FakeProbe::with_busy(&[20050, 30050, 20051])
            ),
            [(20050, true, Some(30051)), (20051, true, Some(30052))]
        );
    }

    #[test]
    fn should_give_the_same_suggestion_when_two_instances_list_the_same_busy_port() {
        let probe = FakeProbe::with_busy(&[3000]);

        let instances = assign_local_ports(
            vec![
                instance(vec![port(3000, DevslotPortStatus::Ambiguous)]),
                instance(vec![port(3000, DevslotPortStatus::Ambiguous)]),
            ],
            &[],
            &probe,
        );

        assert_eq!(
            instances
                .iter()
                .map(|listed| listed.ports[0].suggested_local_port)
                .collect::<Vec<_>>(),
            [Some(13000), Some(13000)]
        );
    }

    #[test]
    fn should_give_no_suggestion_when_the_offset_goes_past_the_last_port() {
        assert_eq!(
            outcome(vec![new_port(60000)], &[], &FakeProbe::with_busy(&[60000])),
            [(60000, true, None)]
        );
    }

    #[test]
    fn should_give_no_suggestion_when_every_attempt_is_busy() {
        let first = 20050 + SUGGESTED_PORT_OFFSET;
        let busy: Vec<u16> = std::iter::once(20050)
            .chain((0..SUGGESTION_ATTEMPTS).map(|step| u16::try_from(first + step).unwrap()))
            .collect();

        assert_eq!(
            outcome(vec![new_port(20050)], &[], &FakeProbe::with_busy(&busy)),
            [(20050, true, None)]
        );
    }

    #[test]
    fn should_suggest_the_last_attempt_when_only_it_is_free() {
        let first = 20050 + SUGGESTED_PORT_OFFSET;
        let busy: Vec<u16> = std::iter::once(20050)
            .chain((0..SUGGESTION_ATTEMPTS - 1).map(|step| u16::try_from(first + step).unwrap()))
            .collect();

        assert_eq!(
            outcome(vec![new_port(20050)], &[], &FakeProbe::with_busy(&busy)),
            [(
                20050,
                true,
                Some(u16::try_from(first + SUGGESTION_ATTEMPTS - 1).unwrap())
            )]
        );
    }

    #[test]
    fn should_never_probe_a_port_that_is_already_tunneled() {
        let probe = FakeProbe::with_busy(&[20050]);
        let tunneled = port(
            20050,
            DevslotPortStatus::AlreadyTunneled {
                tunnel_id: TunnelId::generate(),
            },
        );

        let result = outcome(vec![tunneled], &[], &probe);

        assert_eq!(
            (result, probe.probed.borrow().clone()),
            (vec![(20050, false, None)], Vec::new())
        );
    }

    #[test]
    fn should_report_a_port_held_by_a_listener_as_not_free_when_binding() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let held = listener.local_addr().unwrap().port();

        assert!(!BindProbe.is_free(held));
    }

    #[test]
    fn should_report_a_port_as_free_when_nothing_holds_it() {
        let released = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        assert!(BindProbe.is_free(released));
    }
}
