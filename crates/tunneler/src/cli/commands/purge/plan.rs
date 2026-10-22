//! What `tunneler purge` removes, as data: built from the data layout, so it
//! can be checked without touching the file system.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::daemon::notification_identity::describe_registry_key;
use crate::layout::DataLayout;

/// One thing `purge` removes.
///
/// A new kind of trace is added as a variant here, planned in [`build_plan`]
/// and removed in `Purger::remove`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurgeTarget {
    /// The entry that starts the daemon at login.
    LoginLaunch,
    /// The registry key of the Windows notification identity.
    NotificationIdentity,
    /// A directory and everything in it.
    Directory(PathBuf),
    /// A single file.
    File(PathBuf),
    /// The passphrase of an identity file kept in the OS keyring.
    KeyringPassphrase(PathBuf),
    /// The state file, the only record of the keyring passphrases: removed
    /// only when every passphrase was.
    StateFile(PathBuf),
    /// A directory removed only when nothing is left in it.
    DirectoryIfEmpty(PathBuf),
}

impl PurgeTarget {
    /// The line that tells the user what this is.
    pub fn describe(&self) -> String {
        match self {
            Self::LoginLaunch => "launch-at-login entry".to_owned(),
            Self::NotificationIdentity => {
                format!(
                    "notification identity, registry key {}",
                    describe_registry_key()
                )
            }
            Self::Directory(path) => format!("directory {}", path.display()),
            Self::File(path) | Self::StateFile(path) => format!("file {}", path.display()),
            Self::KeyringPassphrase(key_path) => {
                format!("keyring passphrase of {}", key_path.display())
            }
            Self::DirectoryIfEmpty(path) => {
                format!(
                    "directory {}, if nothing else is left in it",
                    path.display()
                )
            }
        }
    }
}

/// The targets in the order they are removed: the traces outside the
/// configuration (the notification identity only when the platform has one,
/// `has_notification_identity`), then the configuration unless `keep_config`, then the
/// directories that may have been left empty.
pub fn build_plan(
    layout: &DataLayout,
    keep_config: bool,
    keyring_entries: &BTreeSet<PathBuf>,
    has_notification_identity: bool,
) -> Vec<PurgeTarget> {
    let mut plan = vec![
        PurgeTarget::LoginLaunch,
        PurgeTarget::Directory(layout.log_directory.clone()),
        PurgeTarget::Directory(layout.webview_data_directory.clone()),
    ];
    if has_notification_identity {
        plan.push(PurgeTarget::NotificationIdentity);
    }
    if !keep_config {
        plan.extend(
            keyring_entries
                .iter()
                .cloned()
                .map(PurgeTarget::KeyringPassphrase),
        );
        plan.push(PurgeTarget::File(layout.tunnels_file.clone()));
        plan.push(PurgeTarget::StateFile(layout.state_file.clone()));
    }
    plan.extend(
        layout
            .removable_when_empty
            .iter()
            .cloned()
            .map(PurgeTarget::DirectoryIfEmpty),
    );
    plan
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ipc::portable_layout;
    use pretty_assertions::assert_eq;

    use super::*;

    fn user_layout() -> DataLayout {
        DataLayout {
            tunnels_file: PathBuf::from("/home/me/.config/tunneler/tunnels.toml"),
            state_file: PathBuf::from("/home/me/.local/state/tunneler/state.toml"),
            log_directory: PathBuf::from("/home/me/.local/share/tunneler/logs"),
            webview_data_directory: PathBuf::from(
                "/home/me/.local/share/com.minimalstuff.tunneler",
            ),
            removable_when_empty: vec![
                PathBuf::from("/home/me/.config/tunneler"),
                PathBuf::from("/home/me/.local/state/tunneler"),
                PathBuf::from("/home/me/.local/share/tunneler"),
            ],
        }
    }

    fn entries() -> BTreeSet<PathBuf> {
        BTreeSet::from([
            PathBuf::from("/home/me/.ssh/id_ed25519"),
            PathBuf::from("/home/me/.ssh/id_rsa"),
        ])
    }

    #[test]
    fn should_remove_everything_in_order_when_the_configuration_is_not_kept() {
        assert_eq!(
            build_plan(&user_layout(), false, &entries(), false),
            vec![
                PurgeTarget::LoginLaunch,
                PurgeTarget::Directory("/home/me/.local/share/tunneler/logs".into()),
                PurgeTarget::Directory("/home/me/.local/share/com.minimalstuff.tunneler".into()),
                PurgeTarget::KeyringPassphrase("/home/me/.ssh/id_ed25519".into()),
                PurgeTarget::KeyringPassphrase("/home/me/.ssh/id_rsa".into()),
                PurgeTarget::File("/home/me/.config/tunneler/tunnels.toml".into()),
                PurgeTarget::StateFile("/home/me/.local/state/tunneler/state.toml".into()),
                PurgeTarget::DirectoryIfEmpty("/home/me/.config/tunneler".into()),
                PurgeTarget::DirectoryIfEmpty("/home/me/.local/state/tunneler".into()),
                PurgeTarget::DirectoryIfEmpty("/home/me/.local/share/tunneler".into()),
            ]
        );
    }

    #[test]
    fn should_keep_the_tunnels_the_state_and_the_passphrases_when_the_configuration_is_kept() {
        let plan = build_plan(&user_layout(), true, &entries(), false);

        assert!(!plan.iter().any(|target| matches!(
            target,
            PurgeTarget::File(_) | PurgeTarget::StateFile(_) | PurgeTarget::KeyringPassphrase(_)
        )));
    }

    #[test]
    fn should_still_remove_the_login_entry_the_logs_and_the_webview_when_the_configuration_is_kept()
    {
        let plan = build_plan(&user_layout(), true, &entries(), false);

        assert_eq!(
            plan[..3],
            [
                PurgeTarget::LoginLaunch,
                PurgeTarget::Directory("/home/me/.local/share/tunneler/logs".into()),
                PurgeTarget::Directory("/home/me/.local/share/com.minimalstuff.tunneler".into()),
            ]
        );
    }

    #[test]
    fn should_target_the_files_next_to_the_executable_when_portable() {
        let layout =
            DataLayout::portable(&portable_layout(Path::new("/opt/tunneler"), true).unwrap());

        assert_eq!(
            build_plan(&layout, false, &BTreeSet::new(), false),
            vec![
                PurgeTarget::LoginLaunch,
                PurgeTarget::Directory("/opt/tunneler/data/logs".into()),
                PurgeTarget::Directory("/opt/tunneler/data/webview".into()),
                PurgeTarget::File("/opt/tunneler/tunnels.toml".into()),
                PurgeTarget::StateFile("/opt/tunneler/data/state.toml".into()),
                PurgeTarget::DirectoryIfEmpty("/opt/tunneler/data".into()),
            ]
        );
    }

    #[test]
    fn should_never_target_the_ssh_directory_the_executables_or_the_marker() {
        let layout =
            DataLayout::portable(&portable_layout(Path::new("/opt/tunneler"), true).unwrap());

        let described: Vec<String> = build_plan(&layout, false, &BTreeSet::new(), false)
            .iter()
            .map(PurgeTarget::describe)
            .collect();

        assert!(described.iter().all(|line| {
            !line.contains(".ssh")
                && !line.ends_with("/opt/tunneler")
                && !line.contains("portable")
                && !line.contains("tunneler-ui")
        }));
    }

    #[test]
    fn should_remove_the_notification_identity_after_the_webview_when_the_platform_has_one() {
        let plan = build_plan(&user_layout(), false, &entries(), true);

        assert_eq!(plan[3], PurgeTarget::NotificationIdentity);
    }

    #[test]
    fn should_remove_the_notification_identity_even_when_the_configuration_is_kept() {
        let plan = build_plan(&user_layout(), true, &entries(), true);

        assert!(plan.contains(&PurgeTarget::NotificationIdentity));
    }

    #[test]
    fn should_not_plan_a_notification_identity_when_the_platform_has_none() {
        let plan = build_plan(&user_layout(), false, &entries(), false);

        assert!(!plan.contains(&PurgeTarget::NotificationIdentity));
    }

    #[test]
    fn should_describe_a_keyring_passphrase_by_the_key_path() {
        assert_eq!(
            PurgeTarget::KeyringPassphrase("/home/me/.ssh/id_rsa".into()).describe(),
            "keyring passphrase of /home/me/.ssh/id_rsa"
        );
    }
}
