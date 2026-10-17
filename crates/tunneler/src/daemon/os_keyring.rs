//! The OS credential store (Windows Credential Manager, macOS Keychain,
//! Secret Service on the other Unixes) as the engine's [`SecretKeyring`].

use std::path::Path;

use engine::{KeyringError, SecretKeyring};
use zeroize::Zeroizing;

const SERVICE_NAME: &str = "tunneler";
const ACCOUNT_PREFIX: &str = "passphrase:";

/// Keeps the passphrase of each identity file under the account
/// `passphrase:<canonical key path>` of the service `tunneler`.
pub(crate) struct OsKeyring;

impl SecretKeyring for OsKeyring {
    fn load(&self, key_path: &Path) -> Result<Option<Zeroizing<String>>, KeyringError> {
        match entry_of(key_path)?.get_password() {
            Ok(passphrase) => Ok(Some(Zeroizing::new(passphrase))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(keyring_error(&error)),
        }
    }

    fn store(&self, key_path: &Path, passphrase: &str) -> Result<(), KeyringError> {
        entry_of(key_path)?
            .set_password(passphrase)
            .map_err(|error| keyring_error(&error))
    }

    fn delete(&self, key_path: &Path) -> Result<(), KeyringError> {
        match entry_of(key_path)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(keyring_error(&error)),
        }
    }
}

fn entry_of(key_path: &Path) -> Result<keyring::Entry, KeyringError> {
    keyring::Entry::new(SERVICE_NAME, &account_of(key_path)).map_err(|error| keyring_error(&error))
}

fn account_of(key_path: &Path) -> String {
    format!("{ACCOUNT_PREFIX}{}", key_path.display())
}

fn keyring_error(error: &keyring::Error) -> KeyringError {
    KeyringError::new(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_the_account_after_the_key_path() {
        assert_eq!(
            account_of(Path::new("/home/me/.ssh/id_ed25519")),
            "passphrase:/home/me/.ssh/id_ed25519"
        );
    }
}
