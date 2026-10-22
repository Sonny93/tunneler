//! Carries a purge plan out through small abstractions, so the logic is
//! checked with fakes and never touches the real keyring or file system.

use std::io;
use std::path::Path;

use async_trait::async_trait;
use engine::{KeyringError, SecretKeyring};
use thiserror::Error;

use super::plan::PurgeTarget;
use crate::cli::CliError;
use crate::daemon::login_launch::{LoginLaunch, LoginLaunchError};

/// The file system operations of a purge. Removing something that is already
/// gone succeeds.
pub trait Filesystem: Send + Sync {
    /// Whether something exists at the path.
    fn exists(&self, path: &Path) -> bool;

    /// Removes a file.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Removes a directory with everything in it.
    fn remove_directory_all(&self, path: &Path) -> io::Result<()>;

    /// Removes a directory when it is empty and leaves it otherwise.
    fn remove_directory_if_empty(&self, path: &Path) -> io::Result<()>;
}

/// The daemon, as far as a purge is concerned.
#[async_trait]
pub trait DaemonControl: Send + Sync {
    /// Whether a daemon answers.
    async fn is_running(&self) -> bool;

    /// Stops the daemon and its tunnels, and waits until it is gone.
    async fn stop(&self) -> Result<(), CliError>;
}

/// Why one item could not be removed.
#[derive(Debug, Error)]
pub enum PurgeError {
    /// A file or directory could not be removed.
    #[error("cannot remove {}: {source}", path.display())]
    Io {
        /// What could not be removed.
        path: std::path::PathBuf,
        /// Why.
        source: io::Error,
    },
    /// The launch-at-login entry could not be removed.
    #[error("cannot remove the launch-at-login entry: {0}")]
    LoginLaunch(#[from] LoginLaunchError),
    /// The state file stays because it is the only record of passphrases that
    /// could not be deleted, so that a second purge can retry.
    #[error("keeping {} because some passphrases could not be deleted from the keyring", path.display())]
    StateFileKept {
        /// The state file.
        path: std::path::PathBuf,
    },
    /// A passphrase could not be deleted from the keyring.
    #[error("cannot delete the keyring passphrase of {}: {source}", key_path.display())]
    Keyring {
        /// The identity file the passphrase belongs to.
        key_path: std::path::PathBuf,
        /// Why.
        source: KeyringError,
    },
}

/// Removes the targets of a plan.
pub struct Purger<'a> {
    /// The daemon, stopped before anything is removed.
    pub daemon: &'a dyn DaemonControl,
    /// The file system.
    pub filesystem: &'a dyn Filesystem,
    /// The launch-at-login entry.
    pub login_launch: &'a dyn LoginLaunch,
    /// The OS keyring.
    pub keyring: &'a dyn SecretKeyring,
}

impl Purger<'_> {
    /// The targets of `plan` that exist, so that the user is told about what is
    /// really there. A launch-at-login entry and a keyring passphrase cannot
    /// be probed cheaply, they always stay.
    pub fn retain_present(&self, plan: Vec<PurgeTarget>) -> Vec<PurgeTarget> {
        plan.into_iter()
            .filter(|target| match target {
                PurgeTarget::Directory(path)
                | PurgeTarget::File(path)
                | PurgeTarget::StateFile(path)
                | PurgeTarget::DirectoryIfEmpty(path) => self.filesystem.exists(path),
                PurgeTarget::LoginLaunch | PurgeTarget::KeyringPassphrase(_) => true,
            })
            .collect()
    }

    /// Stops the daemon when one answers, then removes every target, going on
    /// after a failure. Returns the failures.
    ///
    /// # Errors
    ///
    /// Fails, removing nothing, when the daemon cannot be stopped: it would
    /// write its files again.
    pub async fn purge(&self, plan: &[PurgeTarget]) -> Result<Vec<PurgeError>, CliError> {
        if self.daemon.is_running().await {
            self.daemon.stop().await?;
        }
        let mut failures = Vec::new();
        for target in plan {
            if let PurgeTarget::StateFile(path) = target
                && has_keyring_failure(&failures)
            {
                failures.push(PurgeError::StateFileKept { path: path.clone() });
                continue;
            }
            if let Err(failure) = self.remove(target) {
                failures.push(failure);
            }
        }
        Ok(failures)
    }

    fn remove(&self, target: &PurgeTarget) -> Result<(), PurgeError> {
        match target {
            PurgeTarget::LoginLaunch => Ok(self.login_launch.apply(false)?),
            PurgeTarget::Directory(path) => {
                with_path(path, self.filesystem.remove_directory_all(path))
            }
            PurgeTarget::File(path) | PurgeTarget::StateFile(path) => {
                with_path(path, self.filesystem.remove_file(path))
            }
            PurgeTarget::DirectoryIfEmpty(path) => {
                with_path(path, self.filesystem.remove_directory_if_empty(path))
            }
            PurgeTarget::KeyringPassphrase(key_path) => {
                self.keyring
                    .delete(key_path)
                    .map_err(|source| PurgeError::Keyring {
                        key_path: key_path.clone(),
                        source,
                    })
            }
        }
    }
}

fn has_keyring_failure(failures: &[PurgeError]) -> bool {
    failures
        .iter()
        .any(|failure| matches!(failure, PurgeError::Keyring { .. }))
}

fn with_path(path: &Path, outcome: io::Result<()>) -> Result<(), PurgeError> {
    outcome.map_err(|source| PurgeError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use pretty_assertions::assert_eq;
    use zeroize::Zeroizing;

    use super::*;

    type Journal = Mutex<Vec<String>>;

    struct FakeDaemon<'a> {
        is_running: bool,
        is_failing: bool,
        journal: &'a Journal,
    }

    #[async_trait]
    impl DaemonControl for FakeDaemon<'_> {
        async fn is_running(&self) -> bool {
            self.is_running
        }

        async fn stop(&self) -> Result<(), CliError> {
            self.journal.lock().unwrap().push("stop daemon".to_owned());
            if self.is_failing {
                return Err(CliError::DaemonNotRunning);
            }
            Ok(())
        }
    }

    struct FakeFilesystem<'a> {
        existing: BTreeSet<PathBuf>,
        failing: BTreeSet<PathBuf>,
        journal: &'a Journal,
    }

    impl FakeFilesystem<'_> {
        fn record(&self, operation: &str, path: &Path) -> io::Result<()> {
            self.journal
                .lock()
                .unwrap()
                .push(format!("{operation} {}", path.display()));
            if self.failing.contains(path) {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(())
        }
    }

    impl Filesystem for FakeFilesystem<'_> {
        fn exists(&self, path: &Path) -> bool {
            self.existing.contains(path)
        }

        fn remove_file(&self, path: &Path) -> io::Result<()> {
            self.record("remove file", path)
        }

        fn remove_directory_all(&self, path: &Path) -> io::Result<()> {
            self.record("remove directory", path)
        }

        fn remove_directory_if_empty(&self, path: &Path) -> io::Result<()> {
            self.record("remove directory if empty", path)
        }
    }

    struct FakeLoginLaunch<'a> {
        journal: &'a Journal,
    }

    impl LoginLaunch for FakeLoginLaunch<'_> {
        fn apply(&self, is_enabled: bool) -> Result<(), LoginLaunchError> {
            self.journal
                .lock()
                .unwrap()
                .push(format!("login launch enabled={is_enabled}"));
            Ok(())
        }
    }

    struct FakeKeyring<'a> {
        journal: &'a Journal,
        is_failing: bool,
    }

    impl SecretKeyring for FakeKeyring<'_> {
        fn load(&self, _key_path: &Path) -> Result<Option<Zeroizing<String>>, KeyringError> {
            Err(KeyringError::new("never read during a purge"))
        }

        fn store(&self, _key_path: &Path, _passphrase: &str) -> Result<(), KeyringError> {
            Err(KeyringError::new("never written during a purge"))
        }

        fn delete(&self, key_path: &Path) -> Result<(), KeyringError> {
            self.journal
                .lock()
                .unwrap()
                .push(format!("delete passphrase {}", key_path.display()));
            if self.is_failing {
                return Err(KeyringError::new("locked"));
            }
            Ok(())
        }
    }

    struct Fixture {
        journal: Journal,
        is_daemon_running: bool,
        is_daemon_failing: bool,
        is_keyring_failing: bool,
        existing: BTreeSet<PathBuf>,
        failing: BTreeSet<PathBuf>,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                journal: Mutex::new(Vec::new()),
                is_daemon_running: false,
                is_daemon_failing: false,
                is_keyring_failing: false,
                existing: BTreeSet::new(),
                failing: BTreeSet::new(),
            }
        }

        async fn purge(&self, plan: &[PurgeTarget]) -> Result<Vec<PurgeError>, CliError> {
            let daemon = FakeDaemon {
                is_running: self.is_daemon_running,
                is_failing: self.is_daemon_failing,
                journal: &self.journal,
            };
            let filesystem = FakeFilesystem {
                existing: self.existing.clone(),
                failing: self.failing.clone(),
                journal: &self.journal,
            };
            let login_launch = FakeLoginLaunch {
                journal: &self.journal,
            };
            let keyring = FakeKeyring {
                journal: &self.journal,
                is_failing: self.is_keyring_failing,
            };
            Purger {
                daemon: &daemon,
                filesystem: &filesystem,
                login_launch: &login_launch,
                keyring: &keyring,
            }
            .purge(plan)
            .await
        }

        fn retain_present(&self, plan: Vec<PurgeTarget>) -> Vec<PurgeTarget> {
            let daemon = FakeDaemon {
                is_running: false,
                is_failing: false,
                journal: &self.journal,
            };
            let filesystem = FakeFilesystem {
                existing: self.existing.clone(),
                failing: BTreeSet::new(),
                journal: &self.journal,
            };
            let login_launch = FakeLoginLaunch {
                journal: &self.journal,
            };
            let keyring = FakeKeyring {
                journal: &self.journal,
                is_failing: self.is_keyring_failing,
            };
            Purger {
                daemon: &daemon,
                filesystem: &filesystem,
                login_launch: &login_launch,
                keyring: &keyring,
            }
            .retain_present(plan)
        }

        fn journal(&self) -> Vec<String> {
            self.journal.lock().unwrap().clone()
        }
    }

    fn full_plan() -> Vec<PurgeTarget> {
        vec![
            PurgeTarget::LoginLaunch,
            PurgeTarget::Directory("/data/logs".into()),
            PurgeTarget::File("/config/tunnels.toml".into()),
            PurgeTarget::KeyringPassphrase("/ssh/id_rsa".into()),
            PurgeTarget::StateFile("/state/state.toml".into()),
            PurgeTarget::DirectoryIfEmpty("/config".into()),
        ]
    }

    #[tokio::test]
    async fn should_remove_every_target_in_order_when_the_plan_runs() {
        let fixture = Fixture::new();

        fixture.purge(&full_plan()).await.unwrap();

        assert_eq!(
            fixture.journal(),
            vec![
                "login launch enabled=false",
                "remove directory /data/logs",
                "remove file /config/tunnels.toml",
                "delete passphrase /ssh/id_rsa",
                "remove file /state/state.toml",
                "remove directory if empty /config",
            ]
        );
    }

    #[tokio::test]
    async fn should_stop_the_daemon_before_removing_anything_when_one_runs() {
        let mut fixture = Fixture::new();
        fixture.is_daemon_running = true;

        fixture
            .purge(&[PurgeTarget::File("/config/tunnels.toml".into())])
            .await
            .unwrap();

        assert_eq!(
            fixture.journal(),
            vec!["stop daemon", "remove file /config/tunnels.toml"]
        );
    }

    #[tokio::test]
    async fn should_not_stop_anything_when_no_daemon_runs() {
        let fixture = Fixture::new();

        fixture.purge(&full_plan()).await.unwrap();

        assert!(!fixture.journal().contains(&"stop daemon".to_owned()));
    }

    #[tokio::test]
    async fn should_remove_nothing_when_the_daemon_cannot_be_stopped() {
        let mut fixture = Fixture::new();
        fixture.is_daemon_running = true;
        fixture.is_daemon_failing = true;

        let outcome = fixture.purge(&full_plan()).await;

        assert!(outcome.is_err());
        assert_eq!(fixture.journal(), vec!["stop daemon"]);
    }

    #[tokio::test]
    async fn should_go_on_with_the_other_targets_when_one_removal_fails() {
        let mut fixture = Fixture::new();
        fixture.failing.insert("/data/logs".into());

        let failures = fixture.purge(&full_plan()).await.unwrap();

        assert_eq!(failures.len(), 1);
        assert_eq!(fixture.journal().len(), 6);
    }

    #[tokio::test]
    async fn should_keep_the_state_file_when_a_passphrase_could_not_be_deleted() {
        let mut fixture = Fixture::new();
        fixture.is_keyring_failing = true;

        let failures = fixture.purge(&full_plan()).await.unwrap();

        assert!(
            !fixture
                .journal()
                .contains(&"remove file /state/state.toml".to_owned())
        );
        assert!(matches!(
            failures.last(),
            Some(PurgeError::StateFileKept { .. })
        ));
    }

    #[tokio::test]
    async fn should_name_the_path_and_the_cause_when_a_removal_fails() {
        let mut fixture = Fixture::new();
        fixture.failing.insert("/data/logs".into());

        let failures = fixture.purge(&full_plan()).await.unwrap();

        assert_eq!(
            failures[0].to_string(),
            "cannot remove /data/logs: permission denied"
        );
    }

    #[test]
    fn should_skip_the_files_and_directories_that_do_not_exist() {
        let mut fixture = Fixture::new();
        fixture.existing.insert("/config/tunnels.toml".into());

        let present = fixture.retain_present(full_plan());

        assert_eq!(
            present,
            vec![
                PurgeTarget::LoginLaunch,
                PurgeTarget::File("/config/tunnels.toml".into()),
                PurgeTarget::KeyringPassphrase("/ssh/id_rsa".into()),
            ]
        );
    }
}
