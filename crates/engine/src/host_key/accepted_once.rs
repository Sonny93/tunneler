use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};

use known_hosts::HostKey;

#[derive(Debug, Default)]
pub(crate) struct AcceptedOnceStore {
    entries: Mutex<HashSet<(String, HostKey)>>,
}

impl AcceptedOnceStore {
    pub(crate) fn remember(&self, lookup_name: &str, host_key: &HostKey) {
        self.entries()
            .insert((lookup_name.to_owned(), host_key.clone()));
    }

    pub(crate) fn contains(&self, lookup_name: &str, host_key: &HostKey) -> bool {
        self.entries()
            .contains(&(lookup_name.to_owned(), host_key.clone()))
    }

    fn entries(&self) -> std::sync::MutexGuard<'_, HashSet<(String, HostKey)>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(payload: &[u8]) -> HostKey {
        let mut blob = Vec::new();
        blob.extend_from_slice(&11_u32.to_be_bytes());
        blob.extend_from_slice(b"ssh-ed25519");
        blob.extend_from_slice(payload);
        HostKey::new("ssh-ed25519", blob).unwrap()
    }

    #[test]
    fn should_contain_key_when_it_was_remembered_for_the_same_name() {
        let store = AcceptedOnceStore::default();
        store.remember("host", &key(b"a"));
        assert!(store.contains("host", &key(b"a")));
    }

    #[test]
    fn should_not_contain_key_when_name_differs() {
        let store = AcceptedOnceStore::default();
        store.remember("host", &key(b"a"));
        assert!(!store.contains("other", &key(b"a")));
    }

    #[test]
    fn should_not_contain_key_when_key_differs() {
        let store = AcceptedOnceStore::default();
        store.remember("host", &key(b"a"));
        assert!(!store.contains("host", &key(b"b")));
    }
}
