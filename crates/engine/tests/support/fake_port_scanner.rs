use std::net::SocketAddr;
use std::sync::{Mutex, MutexGuard};

use engine::{
    ListeningSocket, PortScanner, ProcessIdentity, ProcessInfo, ScanError, SystemScan,
    TerminateError,
};

const IDENTITY_STEP: u64 = 10;

/// A scanner that reports whatever the test scripted, and records the
/// termination requests. Terminating a process removes it from the scan.
#[derive(Default)]
pub struct FakePortScanner {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    scan: SystemScan,
    scan_count: usize,
    terminated: Vec<ProcessIdentity>,
    is_scan_failing: bool,
}

impl FakePortScanner {
    pub fn set_scan(&self, scan: SystemScan) {
        self.state().scan = scan;
    }

    pub fn scan_count(&self) -> usize {
        self.state().scan_count
    }

    pub fn terminated(&self) -> Vec<ProcessIdentity> {
        self.state().terminated.clone()
    }

    pub fn fail_scans(&self) {
        self.state().is_scan_failing = true;
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }
}

impl PortScanner for FakePortScanner {
    fn scan(&self) -> Result<SystemScan, ScanError> {
        let mut state = self.state();
        state.scan_count += 1;
        if state.is_scan_failing {
            return Err(ScanError::ListeningSockets {
                message: "scripted failure".to_owned(),
            });
        }
        Ok(state.scan.clone())
    }

    fn terminate(&self, process: ProcessIdentity) -> Result<(), TerminateError> {
        let mut state = self.state();
        state.terminated.push(process);
        state
            .scan
            .listeners
            .retain(|socket| socket.pid != process.pid);
        state
            .scan
            .processes
            .retain(|candidate| candidate.identity.pid != process.pid);
        Ok(())
    }
}

pub fn identity_of(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        started_at: u64::from(pid) * IDENTITY_STEP,
    }
}

pub fn listener(port: u16, pid: u32, process: &str) -> ListeningSocket {
    ListeningSocket {
        address: SocketAddr::from(([127, 0, 0, 1], port)),
        pid,
        process: process.to_owned(),
    }
}

pub fn process(pid: u32, name: &str, command_line: &str) -> ProcessInfo {
    ProcessInfo {
        identity: identity_of(pid),
        name: name.to_owned(),
        command_line: command_line.split_whitespace().map(str::to_owned).collect(),
    }
}

/// A scan with one `ssh` process listening on each given port, forwarding the
/// matching `-L` argument.
pub fn ssh_scan(pid: u32, forwards: &[(u16, String)]) -> SystemScan {
    let options: Vec<String> = forwards
        .iter()
        .map(|(_, forward)| format!("-L {forward}"))
        .collect();
    SystemScan {
        listeners: forwards
            .iter()
            .map(|(port, _)| listener(*port, pid, "ssh"))
            .collect(),
        processes: vec![process(
            pid,
            "ssh",
            &format!("ssh -N {} somehost", options.join(" ")),
        )],
    }
}

/// A scan with one non-`ssh` process listening on `port`.
pub fn foreign_scan(pid: u32, name: &str, port: u16) -> SystemScan {
    SystemScan {
        listeners: vec![listener(port, pid, name)],
        processes: vec![process(pid, name, name)],
    }
}
