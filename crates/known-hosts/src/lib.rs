//! Reading and writing of OpenSSH `known_hosts` files.
//!
//! The behavior reference is OpenSSH 10.0 (`hostfile.c`, `match.c` and
//! `check_host_key` in `sshconnect.c`). [`KnownHosts::load`] reads the user
//! and global files, [`KnownHosts::check`] answers whether a presented host
//! key is trusted, changed, revoked or unknown, and [`append_entry`] records a
//! new key.
//!
//! The crate works on plain paths and key bytes: mapping from `ssh_config`
//! (`UserKnownHostsFile`, `HostKeyAlias`, `HashKnownHosts`) is the caller's
//! job.
//!
//! # Known gaps
//!
//! - Only the host name is looked up: OpenSSH's optional `CheckHostIP` lookup
//!   by address is not performed, nor is `KnownHostsCommand`.
//! - A revoked certificate is only detected through its signing authority, not
//!   through the certificate key itself.
//! - File ownership and permission checks are not performed on read.

#![warn(missing_docs)]

mod atomic_file;
mod check;
mod encoding;
mod entry;
mod error;
mod hashing;
mod key;
mod known_hosts;
mod lookup;
mod parser;
mod pattern;
mod verdict;
mod writer;

pub use error::{Error, Warning, WarningKind};
pub use key::{HostKey, HostKeyError};
pub use known_hosts::{KnownHosts, Loaded};
pub use lookup::{HostLookup, PresentedKey};
pub use verdict::Verdict;
pub use writer::{Hashing, append_entry};
