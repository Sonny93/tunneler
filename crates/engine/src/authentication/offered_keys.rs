use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};

use russh::keys::{HashAlg, PublicKey};

/// The public keys already offered to the server, so that an identity held
/// both by the agent and in a file is offered once.
#[derive(Debug, Default)]
pub(crate) struct OfferedKeys {
    fingerprints: Mutex<HashSet<String>>,
}

impl OfferedKeys {
    pub(crate) fn insert(&self, key: &PublicKey) {
        self.fingerprints().insert(fingerprint_of(key));
    }

    pub(crate) fn contains(&self, key: &PublicKey) -> bool {
        self.fingerprints().contains(&fingerprint_of(key))
    }

    fn fingerprints(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.fingerprints
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

fn fingerprint_of(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

#[cfg(test)]
mod tests {
    use russh::keys::key::safe_rng;
    use russh::keys::{Algorithm, PrivateKey};

    use super::*;

    fn public_key() -> PublicKey {
        PrivateKey::random(&mut safe_rng(), Algorithm::Ed25519)
            .unwrap()
            .public_key()
            .clone()
    }

    #[test]
    fn should_contain_the_key_when_it_was_inserted() {
        let offered = OfferedKeys::default();
        let key = public_key();
        offered.insert(&key);
        assert!(offered.contains(&key));
    }

    #[test]
    fn should_not_contain_another_key_when_a_key_was_inserted() {
        let offered = OfferedKeys::default();
        offered.insert(&public_key());
        assert!(!offered.contains(&public_key()));
    }
}
