use std::path::Path;

use thiserror::Error;
use zeroize::Zeroizing;

/// The OS keyring refused or failed an operation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{reason}")]
pub struct KeyringError {
    reason: String,
}

impl KeyringError {
    /// An error described by `reason`.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

/// Where passphrases are kept across runs, usually the OS credential store.
///
/// The operations block, and may wait for the user to unlock the keyring;
/// the engine runs them off the executor. Entries are keyed by the canonical
/// path of the identity file.
pub trait SecretKeyring: Send + Sync {
    /// The stored passphrase of the identity file, if any.
    fn load(&self, key_path: &Path) -> Result<Option<Zeroizing<String>>, KeyringError>;

    /// Stores the passphrase, replacing a previous one.
    fn store(&self, key_path: &Path, passphrase: &str) -> Result<(), KeyringError>;

    /// Deletes the stored passphrase. Succeeds when there is none.
    fn delete(&self, key_path: &Path) -> Result<(), KeyringError>;
}
