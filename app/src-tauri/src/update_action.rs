//! What the UI can do about a newer version, by how the program is installed.

use serde::Serialize;

/// The operating systems that differ for updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Windows,
    Other,
}

impl OperatingSystem {
    /// The operating system this build runs on.
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

/// Where the program keeps its files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installation {
    Installed,
    Portable,
}

impl Installation {
    /// The installation of the running program, from its portable marker.
    pub fn detect() -> Self {
        if ipc::detect_portable_layout().is_some() {
            Self::Portable
        } else {
            Self::Installed
        }
    }
}

/// What the banner offers for a newer version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateAction {
    /// Download, verify and run the installer, then relaunch.
    InstallAndRestart,
    /// Only point to the release page.
    ReleaseLinkOnly,
}

/// Only the installed Windows program updates itself. Every other form is
/// replaced by hand: the Tauri updater can only replace an `AppImage`, which
/// does not contain the daemon, so UI and daemon would end up mismatched.
pub const fn decide_update_action(
    operating_system: OperatingSystem,
    installation: Installation,
) -> UpdateAction {
    match (operating_system, installation) {
        (OperatingSystem::Windows, Installation::Installed) => UpdateAction::InstallAndRestart,
        _ => UpdateAction::ReleaseLinkOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_install_and_restart_when_windows_is_installed() {
        assert_eq!(
            decide_update_action(OperatingSystem::Windows, Installation::Installed),
            UpdateAction::InstallAndRestart
        );
    }

    #[test]
    fn should_only_link_the_release_when_windows_is_portable() {
        assert_eq!(
            decide_update_action(OperatingSystem::Windows, Installation::Portable),
            UpdateAction::ReleaseLinkOnly
        );
    }

    #[test]
    fn should_only_link_the_release_when_the_system_is_not_windows() {
        assert_eq!(
            decide_update_action(OperatingSystem::Other, Installation::Installed),
            UpdateAction::ReleaseLinkOnly
        );
    }

    #[test]
    fn should_only_link_the_release_when_the_system_is_not_windows_and_portable() {
        assert_eq!(
            decide_update_action(OperatingSystem::Other, Installation::Portable),
            UpdateAction::ReleaseLinkOnly
        );
    }

    #[test]
    fn should_serialize_the_actions_in_camel_case() {
        assert_eq!(
            serde_json::to_value([
                UpdateAction::InstallAndRestart,
                UpdateAction::ReleaseLinkOnly
            ])
            .unwrap(),
            serde_json::json!(["installAndRestart", "releaseLinkOnly"])
        );
    }
}
