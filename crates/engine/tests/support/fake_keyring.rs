use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use engine::{KeyringError, SecretKeyring};
use zeroize::Zeroizing;

/// A keyring that keeps passphrases in memory and counts the calls.
#[derive(Default)]
pub struct FakeKeyring {
    entries: Mutex<HashMap<PathBuf, String>>,
    load_count: Mutex<usize>,
    store_count: Mutex<usize>,
    delete_count: Mutex<usize>,
}

impl FakeKeyring {
    pub fn with_entry(self, key_path: &Path, passphrase: &str) -> Self {
        self.entries
            .lock()
            .unwrap()
            .insert(key_path.to_owned(), passphrase.to_owned());
        self
    }

    pub fn passphrase_of(&self, key_path: &Path) -> Option<String> {
        self.entries.lock().unwrap().get(key_path).cloned()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn load_count(&self) -> usize {
        *self.load_count.lock().unwrap()
    }

    pub fn store_count(&self) -> usize {
        *self.store_count.lock().unwrap()
    }

    pub fn delete_count(&self) -> usize {
        *self.delete_count.lock().unwrap()
    }
}

impl SecretKeyring for FakeKeyring {
    fn load(&self, key_path: &Path) -> Result<Option<Zeroizing<String>>, KeyringError> {
        *self.load_count.lock().unwrap() += 1;
        Ok(self.passphrase_of(key_path).map(Zeroizing::new))
    }

    fn store(&self, key_path: &Path, passphrase: &str) -> Result<(), KeyringError> {
        *self.store_count.lock().unwrap() += 1;
        self.entries
            .lock()
            .unwrap()
            .insert(key_path.to_owned(), passphrase.to_owned());
        Ok(())
    }

    fn delete(&self, key_path: &Path) -> Result<(), KeyringError> {
        *self.delete_count.lock().unwrap() += 1;
        self.entries.lock().unwrap().remove(key_path);
        Ok(())
    }
}
