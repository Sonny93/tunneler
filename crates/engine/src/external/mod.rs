//! Detection of tunnels that something else already provides: which process
//! owns the listen port of a configured tunnel, and which `ssh` processes
//! forward ports that match no configured tunnel.
//!
//! A [`PortScanner`] reads the listening sockets and the processes that
//! matter; the classification on top of its [`SystemScan`] is pure.
//!
//! # Known limits
//!
//! - On Linux, the sockets of another user's process have no PID without
//!   root, so they are invisible to the scan.
//! - `-o LocalForward=...` style options on an `ssh` command line are not
//!   interpreted.

mod classification;
mod command_line;
mod matching;
mod scanner;

pub use classification::{PortOccupation, PortOwner, UnmanagedProcess};
pub use command_line::{SshForward, SshInvocation, parse_ssh_command_line};
pub use scanner::{
    ListeningSocket, PortScanner, ProcessIdentity, ProcessInfo, ScanError, SystemPortScanner,
    SystemScan, TerminateError, is_ssh_process,
};

pub(crate) use classification::{occupation_of, unmanaged_processes};
