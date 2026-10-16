use std::path::PathBuf;

use crate::key::HostKey;

/// The answer to "may this host key be trusted?".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The key (or its signing authority) is recorded for this host.
    Trusted {
        /// The file holding the matching line.
        path: PathBuf,
        /// The 1-based line number.
        line: usize,
    },
    /// The host has no applicable entry at all (for a certificate: no
    /// `@cert-authority` line for the signing authority).
    Unknown,
    /// The host has plain entries and none equals the presented key, whatever
    /// its algorithm. This is a hard failure, as in OpenSSH.
    Changed {
        /// The file holding the offending line.
        path: PathBuf,
        /// The 1-based line number.
        line: usize,
        /// The key recorded on that line.
        known: HostKey,
    },
    /// A `@revoked` line matches the key (or its signing authority).
    Revoked {
        /// The file holding the `@revoked` line.
        path: PathBuf,
        /// The 1-based line number.
        line: usize,
    },
}
