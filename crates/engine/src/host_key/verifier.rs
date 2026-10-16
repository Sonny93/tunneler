use std::path::PathBuf;

use async_trait::async_trait;

/// An unknown host key awaiting the user's decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownHostKey {
    /// The host alias being connected to.
    pub alias: String,
    /// The name the key would be recorded under (`host`, `[host]:port` or the
    /// `HostKeyAlias`).
    pub lookup_name: String,
    /// The key algorithm, such as `ssh-ed25519`.
    pub algorithm: String,
    /// The OpenSSH SHA-256 fingerprint, `SHA256:` and unpadded base64.
    pub fingerprint_sha256: String,
}

/// What to do with an unknown host key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKeyDecision {
    /// Trust the key and record it in the first user `known_hosts` file.
    AcceptAndSave,
    /// Trust the key until the process ends, without recording it.
    AcceptOnce,
    /// Refuse the key.
    Reject,
}

/// Why a host key was refused without asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    /// The host has a different key on record.
    Changed {
        /// The SHA-256 fingerprint on record.
        known_fingerprint: String,
        /// The `known_hosts` file holding the recorded key.
        path: PathBuf,
        /// The 1-based line of the recorded key.
        line: usize,
    },
    /// A `@revoked` line matches the key.
    Revoked {
        /// The `known_hosts` file holding the `@revoked` line.
        path: PathBuf,
        /// The 1-based line of the `@revoked` line.
        line: usize,
    },
    /// The host is unknown and the configuration forbids asking.
    UnknownRejectedByPolicy,
}

/// A host key that was refused without asking the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedHostKey {
    /// The host alias being connected to.
    pub alias: String,
    /// Why the key was refused.
    pub reason: RejectionReason,
    /// The SHA-256 fingerprint the server presented.
    pub presented_fingerprint: String,
}

/// Where the engine gets decisions about host keys from.
///
/// A changed or revoked key is never submitted for a decision: it is refused
/// and only reported.
#[async_trait]
pub trait HostKeyVerifier: Send + Sync {
    /// Whether to trust a key the host has no record of.
    async fn decide_unknown(&self, host_key: UnknownHostKey) -> HostKeyDecision;

    /// Notification that a key was refused, so the user can be alerted.
    async fn report_rejected(&self, rejection: RejectedHostKey);
}
