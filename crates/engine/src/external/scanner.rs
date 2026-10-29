use std::collections::HashSet;
use std::net::SocketAddr;

use listeners::{Protocol, SocketState};
use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, Signal, System, UpdateKind};
use thiserror::Error;

const SSH_PROGRAM_NAMES: [&str; 2] = ["ssh", "ssh.exe"];

/// Reads what listens on the machine and ends processes.
///
/// Known limit: on Linux, the sockets of another user's process have no PID
/// without root, so they are invisible to the scan.
pub trait PortScanner: Send + Sync {
    /// Blocking: reads the listening TCP sockets and the processes that matter.
    ///
    /// # Errors
    ///
    /// Returns [`ScanError`] when the operating system refuses to list the
    /// listening sockets.
    fn scan(&self) -> Result<SystemScan, ScanError>;

    /// Blocking: asks the process to terminate, after checking it is still the
    /// same process.
    ///
    /// # Errors
    ///
    /// Returns [`TerminateError`] when the process is gone, was replaced by
    /// another one with the same PID, or refuses the request.
    fn terminate(&self, process: ProcessIdentity) -> Result<(), TerminateError>;
}

/// A process that survives PID reuse: the PID and the moment it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessIdentity {
    /// The process ID.
    pub pid: u32,
    /// The start time, in seconds since the Unix epoch.
    pub started_at: u64,
}

/// A TCP socket in the listening state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListeningSocket {
    /// The bound address and port.
    pub address: SocketAddr,
    /// The ID of the process owning the socket.
    pub pid: u32,
    /// The name of the process owning the socket.
    pub process: String,
}

/// A process that owns a listener or runs `ssh`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    /// The PID and start time.
    pub identity: ProcessIdentity,
    /// The process name.
    pub name: String,
    /// The program followed by its arguments.
    pub command_line: Vec<String>,
}

/// A snapshot of the listening sockets and of the processes that matter.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemScan {
    /// The listening TCP sockets.
    pub listeners: Vec<ListeningSocket>,
    /// The processes owning a listener, and the `ssh` processes.
    pub processes: Vec<ProcessInfo>,
}

impl SystemScan {
    pub(crate) fn process(&self, pid: u32) -> Option<&ProcessInfo> {
        self.processes
            .iter()
            .find(|process| process.identity.pid == pid)
    }
}

/// The listening sockets or the processes could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScanError {
    /// The operating system refused to list the listening sockets.
    #[error("cannot list the listening sockets: {message}")]
    ListeningSockets {
        /// The message reported by the operating system.
        message: String,
    },
    /// The operating system refused to list the processes.
    #[error("cannot list the processes: {message}")]
    Processes {
        /// The message reported by the operating system.
        message: String,
    },
}

/// A process could not be terminated.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TerminateError {
    /// No process has this PID any more.
    #[error("process {0} no longer exists")]
    NotFound(u32),
    /// The PID now belongs to another process.
    #[error("process {0} is no longer the same process")]
    ProcessChanged(u32),
    /// The operating system refused the request, for instance for another
    /// user's process.
    #[error("the operating system refused to terminate process {pid}")]
    Refused {
        /// The process ID.
        pid: u32,
    },
}

/// Whether `name` is the name of an `ssh` client process.
pub fn is_ssh_process(name: &str) -> bool {
    SSH_PROGRAM_NAMES
        .iter()
        .any(|program| name.eq_ignore_ascii_case(program))
}

/// The scanner backed by the operating system.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemPortScanner;

impl PortScanner for SystemPortScanner {
    fn scan(&self) -> Result<SystemScan, ScanError> {
        let listeners = read_listening_sockets()?;
        let listener_pids: HashSet<u32> = listeners.iter().map(|socket| socket.pid).collect();
        let mut system = System::new().map_err(|error| ScanError::Processes {
            message: error.to_string(),
        })?;
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
        );
        let mut processes: Vec<ProcessInfo> = system
            .processes()
            .values()
            .map(process_info)
            .filter(|process| {
                listener_pids.contains(&process.identity.pid) || is_ssh_process(&process.name)
            })
            .collect();
        processes.sort_by_key(|process| process.identity.pid);
        Ok(SystemScan {
            listeners,
            processes,
        })
    }

    fn terminate(&self, process: ProcessIdentity) -> Result<(), TerminateError> {
        let pid = Pid::from_u32(process.pid);
        let mut system = System::new().map_err(|_| TerminateError::Refused { pid: process.pid })?;
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing(),
        );
        let running = system
            .process(pid)
            .ok_or(TerminateError::NotFound(process.pid))?;
        if running.start_time() != process.started_at {
            return Err(TerminateError::ProcessChanged(process.pid));
        }
        let is_signal_sent = running
            .kill_with(Signal::Term)
            .unwrap_or_else(|| running.kill());
        is_signal_sent
            .then_some(())
            .ok_or(TerminateError::Refused { pid: process.pid })
    }
}

fn read_listening_sockets() -> Result<Vec<ListeningSocket>, ScanError> {
    let all_listeners = listeners::get_all().map_err(|error| ScanError::ListeningSockets {
        message: error.to_string(),
    })?;
    let mut sockets: Vec<ListeningSocket> = all_listeners
        .into_iter()
        .filter(|listener| {
            listener.protocol == Protocol::TCP && listener.state == SocketState::Listen
        })
        .map(|listener| ListeningSocket {
            address: listener.socket,
            pid: listener.process.pid,
            process: listener.process.name,
        })
        .collect();
    sockets.sort_by_key(|socket| (socket.pid, socket.address));
    Ok(sockets)
}

fn process_info(process: &Process) -> ProcessInfo {
    ProcessInfo {
        identity: ProcessIdentity {
            pid: process.pid().as_u32(),
            started_at: process.start_time(),
        },
        name: process.name().to_string_lossy().into_owned(),
        command_line: process
            .cmd()
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    #[test]
    fn should_recognise_ssh_when_name_is_ssh() {
        assert!(is_ssh_process("ssh"));
    }

    #[test]
    fn should_recognise_ssh_when_name_is_a_windows_executable_in_any_case() {
        assert!(is_ssh_process("SSH.EXE"));
    }

    #[test]
    fn should_not_recognise_ssh_when_name_is_another_program() {
        assert!(!is_ssh_process("sshd"));
    }

    #[test]
    fn should_report_own_listener_when_scanning_the_system() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let bound_address = listener.local_addr().unwrap();

        let scan = SystemPortScanner.scan().unwrap();

        assert!(
            scan.listeners.iter().any(|socket| {
                socket.address == bound_address && socket.pid == std::process::id()
            })
        );
    }

    #[test]
    fn should_report_own_process_when_it_owns_a_listener() {
        let _listener = TcpListener::bind("127.0.0.1:0").unwrap();

        let scan = SystemPortScanner.scan().unwrap();

        assert!(scan.process(std::process::id()).is_some());
    }

    #[test]
    fn should_refuse_termination_when_start_time_differs() {
        let own_pid = std::process::id();
        let identity = ProcessIdentity {
            pid: own_pid,
            started_at: 0,
        };

        assert_eq!(
            SystemPortScanner.terminate(identity),
            Err(TerminateError::ProcessChanged(own_pid))
        );
    }

    #[test]
    fn should_report_missing_process_when_pid_does_not_exist() {
        let identity = ProcessIdentity {
            pid: u32::MAX - 1,
            started_at: 0,
        };

        assert_eq!(
            SystemPortScanner.terminate(identity),
            Err(TerminateError::NotFound(u32::MAX - 1))
        );
    }
}
