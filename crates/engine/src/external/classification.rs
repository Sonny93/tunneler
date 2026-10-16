use std::collections::HashSet;

use super::command_line::{SshForward, parse_ssh_command_line};
use super::matching::{address_overlaps_listener, forward_matches};
use super::scanner::{ListeningSocket, ProcessIdentity, SystemScan, is_ssh_process};
use crate::store::{Settings, TunnelDefinition, TunnelKind};

/// The process holding a port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortOwner {
    /// The process ID.
    pub pid: u32,
    /// The process name.
    pub process: String,
}

/// Who holds the listen port of a configured tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortOccupation {
    /// An `ssh` process already provides the tunnel.
    External(PortOwner),
    /// Something else holds the port.
    Conflict(Option<PortOwner>),
}

/// An `ssh` process with forwards that match no configured tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmanagedProcess {
    /// The process ID.
    pub pid: u32,
    /// The process name.
    pub process: String,
    /// The host the process connects to, when the command line has one.
    pub destination: Option<String>,
    /// The forwards that match no configured tunnel.
    pub forwards: Vec<SshForward>,
}

pub(crate) fn occupation_of(
    definition: &TunnelDefinition,
    settings: &Settings,
    scan: &SystemScan,
    ignored: &HashSet<ProcessIdentity>,
    own_pid: u32,
) -> Option<PortOccupation> {
    if definition.kind == TunnelKind::Remote {
        return None;
    }
    let configured_address = definition.effective_listen_address(settings);
    let (is_external, listener) = scan
        .listeners
        .iter()
        .filter(|listener| listener.address.port() == definition.listen_port)
        .filter(|listener| address_overlaps_listener(configured_address, listener.address.ip()))
        .filter(|listener| is_candidate(scan, ignored, own_pid, listener.pid))
        .map(|listener| {
            (
                provides_tunnel(definition, settings, scan, listener),
                listener,
            )
        })
        .min_by_key(|(is_external, listener)| (!is_external, listener.pid))?;
    let owner = PortOwner {
        pid: listener.pid,
        process: listener.process.clone(),
    };
    Some(if is_external {
        PortOccupation::External(owner)
    } else {
        PortOccupation::Conflict(Some(owner))
    })
}

pub(crate) fn unmanaged_processes(
    definitions: &[TunnelDefinition],
    settings: &Settings,
    scan: &SystemScan,
    ignored: &HashSet<ProcessIdentity>,
    own_pid: u32,
) -> Vec<UnmanagedProcess> {
    let mut unmanaged: Vec<UnmanagedProcess> = scan
        .processes
        .iter()
        .filter(|process| is_ssh_process(&process.name))
        .filter(|process| is_candidate(scan, ignored, own_pid, process.identity.pid))
        .filter_map(|process| {
            let invocation = parse_ssh_command_line(&process.command_line);
            let forwards: Vec<SshForward> = invocation
                .forwards
                .into_iter()
                .filter(|forward| {
                    !definitions
                        .iter()
                        .any(|definition| forward_matches(definition, settings, forward))
                })
                .collect();
            (!forwards.is_empty()).then(|| UnmanagedProcess {
                pid: process.identity.pid,
                process: process.name.clone(),
                destination: invocation.destination,
                forwards,
            })
        })
        .collect();
    unmanaged.sort_by_key(|process| process.pid);
    unmanaged
}

fn is_candidate(
    scan: &SystemScan,
    ignored: &HashSet<ProcessIdentity>,
    own_pid: u32,
    pid: u32,
) -> bool {
    let is_ignored = scan
        .process(pid)
        .is_some_and(|process| ignored.contains(&process.identity));
    pid != own_pid && !is_ignored
}

fn provides_tunnel(
    definition: &TunnelDefinition,
    settings: &Settings,
    scan: &SystemScan,
    listener: &ListeningSocket,
) -> bool {
    if !is_ssh_process(&listener.process) {
        return false;
    }
    let Some(process) = scan.process(listener.pid) else {
        return false;
    };
    parse_ssh_command_line(&process.command_line)
        .forwards
        .iter()
        .any(|forward| forward_matches(definition, settings, forward))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::scanner::ProcessInfo;
    use crate::store::test_support::tunnel;
    use pretty_assertions::assert_eq;

    const OWN_PID: u32 = 1;

    fn listening(address: &str, pid: u32, process: &str) -> ListeningSocket {
        ListeningSocket {
            address: address.parse().unwrap(),
            pid,
            process: process.to_owned(),
        }
    }

    fn identity(pid: u32) -> ProcessIdentity {
        ProcessIdentity {
            pid,
            started_at: u64::from(pid) * 10,
        }
    }

    fn process(pid: u32, name: &str, command_line: &str) -> ProcessInfo {
        ProcessInfo {
            identity: identity(pid),
            name: name.to_owned(),
            command_line: command_line.split_whitespace().map(str::to_owned).collect(),
        }
    }

    fn ssh_process(pid: u32, command_line: &str) -> ProcessInfo {
        process(pid, "ssh", command_line)
    }

    fn ssh_scan(pid: u32, address: &str, command_line: &str) -> SystemScan {
        SystemScan {
            listeners: vec![listening(address, pid, "ssh")],
            processes: vec![ssh_process(pid, command_line)],
        }
    }

    fn occupation(definition: &TunnelDefinition, scan: &SystemScan) -> Option<PortOccupation> {
        occupation_of(
            definition,
            &Settings::default(),
            scan,
            &HashSet::new(),
            OWN_PID,
        )
    }

    fn owner(pid: u32, process: &str) -> PortOwner {
        PortOwner {
            pid,
            process: process.to_owned(),
        }
    }

    #[test]
    fn should_report_external_when_ssh_holds_the_port_with_a_matching_forward() {
        let scan = ssh_scan(50, "127.0.0.1:3333", "ssh -N -L 3333:127.0.0.1:3333 nova");

        assert_eq!(
            occupation(&tunnel(TunnelKind::Local), &scan),
            Some(PortOccupation::External(owner(50, "ssh")))
        );
    }

    #[test]
    fn should_report_conflict_when_ssh_forward_targets_something_else() {
        let scan = ssh_scan(50, "127.0.0.1:3333", "ssh -N -L 3333:db:5432 nova");

        assert_eq!(
            occupation(&tunnel(TunnelKind::Local), &scan),
            Some(PortOccupation::Conflict(Some(owner(50, "ssh"))))
        );
    }

    #[test]
    fn should_report_conflict_when_another_program_holds_the_port() {
        let scan = SystemScan {
            listeners: vec![listening("127.0.0.1:3333", 60, "nginx")],
            processes: vec![process(60, "nginx", "nginx -g daemon")],
        };

        assert_eq!(
            occupation(&tunnel(TunnelKind::Local), &scan),
            Some(PortOccupation::Conflict(Some(owner(60, "nginx"))))
        );
    }

    #[test]
    fn should_report_external_when_dynamic_tunnel_matches_a_d_option() {
        let scan = ssh_scan(50, "127.0.0.1:3333", "ssh -N -D 3333 nova");

        assert_eq!(
            occupation(&tunnel(TunnelKind::Dynamic), &scan),
            Some(PortOccupation::External(owner(50, "ssh")))
        );
    }

    #[test]
    fn should_report_nothing_when_no_listener_uses_the_port() {
        let scan = ssh_scan(50, "127.0.0.1:4444", "ssh -N -L 4444:127.0.0.1:4444 nova");

        assert_eq!(occupation(&tunnel(TunnelKind::Local), &scan), None);
    }

    #[test]
    fn should_report_nothing_when_listener_is_bound_to_another_address() {
        let scan = ssh_scan(50, "192.0.2.20:3333", "ssh -N -L 3333:127.0.0.1:3333 nova");

        assert_eq!(occupation(&tunnel(TunnelKind::Local), &scan), None);
    }

    #[test]
    fn should_report_nothing_when_tunnel_is_remote() {
        let scan = ssh_scan(50, "127.0.0.1:3333", "ssh -N -L 3333:127.0.0.1:3333 nova");

        assert_eq!(occupation(&tunnel(TunnelKind::Remote), &scan), None);
    }

    #[test]
    fn should_ignore_the_daemon_itself_when_it_holds_the_port() {
        let scan = SystemScan {
            listeners: vec![listening("127.0.0.1:3333", OWN_PID, "tunneler")],
            processes: vec![process(OWN_PID, "tunneler", "tunneler")],
        };

        assert_eq!(occupation(&tunnel(TunnelKind::Local), &scan), None);
    }

    #[test]
    fn should_ignore_process_when_its_identity_is_ignored() {
        let scan = ssh_scan(50, "127.0.0.1:3333", "ssh -N -L 3333:127.0.0.1:3333 nova");
        let ignored = HashSet::from([identity(50)]);

        assert_eq!(
            occupation_of(
                &tunnel(TunnelKind::Local),
                &Settings::default(),
                &scan,
                &ignored,
                OWN_PID
            ),
            None
        );
    }

    #[test]
    fn should_not_ignore_process_when_only_its_pid_matches_an_ignored_identity() {
        let scan = ssh_scan(50, "127.0.0.1:3333", "ssh -N -L 3333:127.0.0.1:3333 nova");
        let ignored = HashSet::from([ProcessIdentity {
            pid: 50,
            started_at: 1,
        }]);

        assert!(
            occupation_of(
                &tunnel(TunnelKind::Local),
                &Settings::default(),
                &scan,
                &ignored,
                OWN_PID
            )
            .is_some()
        );
    }

    #[test]
    fn should_prefer_matching_ssh_when_several_processes_hold_the_port() {
        let scan = SystemScan {
            listeners: vec![
                listening("0.0.0.0:3333", 10, "nginx"),
                listening("127.0.0.1:3333", 90, "ssh"),
            ],
            processes: vec![
                process(10, "nginx", "nginx"),
                ssh_process(90, "ssh -N -L 3333:127.0.0.1:3333 nova"),
            ],
        };

        assert_eq!(
            occupation(&tunnel(TunnelKind::Local), &scan),
            Some(PortOccupation::External(owner(90, "ssh")))
        );
    }

    #[test]
    fn should_pick_lowest_pid_when_no_holder_matches() {
        let scan = SystemScan {
            listeners: vec![
                listening("127.0.0.1:3333", 70, "node"),
                listening("0.0.0.0:3333", 20, "python"),
            ],
            processes: Vec::new(),
        };

        assert_eq!(
            occupation(&tunnel(TunnelKind::Local), &scan),
            Some(PortOccupation::Conflict(Some(owner(20, "python"))))
        );
    }

    fn unmanaged(definitions: &[TunnelDefinition], scan: &SystemScan) -> Vec<UnmanagedProcess> {
        unmanaged_processes(
            definitions,
            &Settings::default(),
            scan,
            &HashSet::new(),
            OWN_PID,
        )
    }

    fn scan_of(processes: Vec<ProcessInfo>) -> SystemScan {
        SystemScan {
            listeners: Vec::new(),
            processes,
        }
    }

    #[test]
    fn should_list_ssh_process_when_its_forward_matches_no_tunnel() {
        let scan = scan_of(vec![ssh_process(50, "ssh -N -L 8080:web:80 alice@nova")]);

        assert_eq!(
            unmanaged(&[], &scan),
            vec![UnmanagedProcess {
                pid: 50,
                process: "ssh".to_owned(),
                destination: Some("nova".to_owned()),
                forwards: parse_ssh_command_line(&[
                    "ssh".to_owned(),
                    "-L".to_owned(),
                    "8080:web:80".to_owned()
                ])
                .forwards,
            }]
        );
    }

    #[test]
    fn should_omit_ssh_process_when_all_its_forwards_match_tunnels() {
        let scan = scan_of(vec![ssh_process(50, "ssh -N -L 3333:127.0.0.1:3333 nova")]);

        assert_eq!(unmanaged(&[tunnel(TunnelKind::Local)], &scan), Vec::new());
    }

    #[test]
    fn should_keep_only_unmatched_forwards_when_some_match_a_tunnel() {
        let scan = scan_of(vec![ssh_process(
            50,
            "ssh -N -L 3333:127.0.0.1:3333 -L 8080:web:80 nova",
        )]);

        let listed = unmanaged(&[tunnel(TunnelKind::Local)], &scan);

        assert_eq!(listed[0].forwards.len(), 1);
    }

    #[test]
    fn should_omit_process_when_it_is_not_ssh() {
        let scan = scan_of(vec![process(50, "sshd", "sshd -L 8080:web:80")]);

        assert_eq!(unmanaged(&[], &scan), Vec::new());
    }

    #[test]
    fn should_omit_ssh_process_when_it_has_no_forward() {
        let scan = scan_of(vec![ssh_process(50, "ssh nova")]);

        assert_eq!(unmanaged(&[], &scan), Vec::new());
    }

    #[test]
    fn should_omit_the_daemon_and_ignored_processes() {
        let scan = scan_of(vec![
            ssh_process(OWN_PID, "ssh -D 1080 nova"),
            ssh_process(50, "ssh -D 1081 nova"),
        ]);
        let ignored = HashSet::from([identity(50)]);

        assert_eq!(
            unmanaged_processes(&[], &Settings::default(), &scan, &ignored, OWN_PID),
            Vec::new()
        );
    }

    #[test]
    fn should_sort_unmanaged_processes_by_pid() {
        let scan = scan_of(vec![
            ssh_process(80, "ssh -D 1080 nova"),
            ssh_process(30, "ssh -D 1081 nova"),
        ]);

        let pids: Vec<u32> = unmanaged(&[], &scan)
            .iter()
            .map(|entry| entry.pid)
            .collect();

        assert_eq!(pids, vec![30, 80]);
    }
}
