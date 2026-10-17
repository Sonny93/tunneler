use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::watch;
use tracing::{debug, warn};
use zeroize::Zeroizing;

use crate::blocking::run_blocking;
use crate::bounded::DeadlinePause;
use crate::keyring::SecretKeyring;
use crate::prompter::Retention;
use crate::state_tracker::StateTracker;
use crate::store::Settings;

/// Remembers the passphrases of identity files: in memory for the life of the
/// vault, and in the OS keyring when the user asked for it and the setting
/// `use_os_keyring` is on.
///
/// The memory copies are zeroized when dropped. Keyring calls block and may
/// show an unlock dialog, so they run off the executor while the deadline of
/// the authentication is paused.
pub(crate) struct PassphraseVault {
    cache: Mutex<HashMap<PathBuf, Zeroizing<String>>>,
    keyring: Option<Arc<dyn SecretKeyring>>,
    settings: watch::Receiver<Settings>,
    state: Arc<StateTracker>,
}

impl PassphraseVault {
    pub(crate) fn new(
        keyring: Option<Arc<dyn SecretKeyring>>,
        settings: watch::Receiver<Settings>,
        state: Arc<StateTracker>,
    ) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            keyring,
            settings,
            state,
        }
    }

    /// A vault with no keyring that records nothing.
    pub(crate) fn in_memory(settings: watch::Receiver<Settings>) -> Self {
        Self::new(None, settings, StateTracker::detached())
    }

    /// The remembered passphrase of the identity file: from memory, else from
    /// the keyring when it is enabled. A keyring failure counts as not found.
    pub(crate) async fn recall(
        &self,
        path: &Path,
        pause: &DeadlinePause,
    ) -> Option<Zeroizing<String>> {
        let key = canonical_key(path).await;
        if let Some(passphrase) = self.cached(&key) {
            return Some(passphrase);
        }
        let keyring = self.enabled_keyring()?;
        let passphrase = {
            let _pause = pause.pause();
            let lookup_key = key.clone();
            run_blocking(move || keyring.load(&lookup_key)).await
        };
        match passphrase {
            Ok(Some(passphrase)) => {
                self.cache(key, passphrase.clone());
                Some(passphrase)
            }
            Ok(None) => None,
            Err(error) => {
                warn!(path = %key.display(), %error, "cannot read the passphrase from the keyring");
                None
            }
        }
    }

    /// Remembers a passphrase that unlocked the identity file, as far as
    /// `retention` asks for.
    pub(crate) async fn remember(
        &self,
        path: &Path,
        passphrase: &Zeroizing<String>,
        retention: Retention,
        pause: &DeadlinePause,
    ) {
        if retention == Retention::Never {
            return;
        }
        let key = canonical_key(path).await;
        self.cache(key.clone(), passphrase.clone());
        if retention == Retention::Keyring {
            self.store_in_keyring(key, passphrase, pause).await;
        }
    }

    /// Forgets a passphrase that no longer unlocks the identity file.
    pub(crate) async fn evict(&self, path: &Path, pause: &DeadlinePause) {
        let key = canonical_key(path).await;
        lock(&self.cache).remove(&key);
        let Some(keyring) = self.enabled_keyring() else {
            return;
        };
        let _pause = pause.pause();
        self.delete_from_keyring(keyring, key).await;
    }

    /// Forgets every passphrase, in memory and in the keyring, and returns
    /// how many identity files had one.
    pub(crate) async fn forget_all(&self) -> usize {
        let mut forgotten: BTreeSet<PathBuf> =
            lock(&self.cache).drain().map(|(key, _)| key).collect();
        let entries = self.state.snapshot().keyring_entries;
        forgotten.extend(entries.iter().cloned());
        match self.keyring.clone() {
            Some(keyring) => {
                for entry in entries {
                    self.delete_from_keyring(Arc::clone(&keyring), entry).await;
                }
            }
            None => self.state.update(|state| state.keyring_entries.clear()),
        }
        debug!(count = forgotten.len(), "forgot the remembered passphrases");
        forgotten.len()
    }

    async fn store_in_keyring(
        &self,
        key: PathBuf,
        passphrase: &Zeroizing<String>,
        pause: &DeadlinePause,
    ) {
        let Some(keyring) = self.enabled_keyring() else {
            debug!(path = %key.display(), "the OS keyring is off, keeping the passphrase in memory only");
            return;
        };
        let stored = {
            let _pause = pause.pause();
            let (entry, secret) = (key.clone(), passphrase.clone());
            run_blocking(move || keyring.store(&entry, &secret)).await
        };
        match stored {
            Ok(()) => self.state.update(|state| {
                state.keyring_entries.insert(key);
            }),
            Err(error) => {
                warn!(path = %key.display(), %error, "cannot store the passphrase in the keyring");
            }
        }
    }

    async fn delete_from_keyring(&self, keyring: Arc<dyn SecretKeyring>, key: PathBuf) {
        let entry = key.clone();
        match run_blocking(move || keyring.delete(&entry)).await {
            Ok(()) => self.state.update(|state| {
                state.keyring_entries.remove(&key);
            }),
            Err(error) => {
                warn!(path = %key.display(), %error, "cannot delete the passphrase from the keyring");
            }
        }
    }

    fn enabled_keyring(&self) -> Option<Arc<dyn SecretKeyring>> {
        if !self.settings.borrow().use_os_keyring {
            return None;
        }
        self.keyring.clone()
    }

    fn cached(&self, key: &Path) -> Option<Zeroizing<String>> {
        lock(&self.cache).get(key).cloned()
    }

    fn cache(&self, key: PathBuf, passphrase: Zeroizing<String>) {
        lock(&self.cache).insert(key, passphrase);
    }
}

async fn canonical_key(path: &Path) -> PathBuf {
    let owned = path.to_owned();
    run_blocking(move || std::fs::canonicalize(&owned).unwrap_or(owned)).await
}

fn lock<Value>(mutex: &Mutex<Value>) -> MutexGuard<'_, Value> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
