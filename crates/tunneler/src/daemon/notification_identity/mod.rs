//! The identity Windows notifications are shown under.
//!
//! A toast belongs to an application user model ID (AUMID). Without one of its
//! own, the daemon borrows the PowerShell one and its toasts read "Windows
//! PowerShell". [`register_for_toasts`] registers the daemon's AUMID, an
//! idempotent registry key holding a display name, when the daemon starts;
//! `tunneler purge` removes the key. Other platforms have no such identity and
//! do nothing.
//!
//! The key carries no icon: an `IconUri` needs an image file on disk, which
//! would be one more trace to clean.
#![cfg_attr(
    not(windows),
    allow(dead_code, reason = "only the Windows notifier registers an identity")
)]

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
use unsupported::SystemNotificationIdentity;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::SystemNotificationIdentity;

use std::io;
use std::sync::Arc;

use thiserror::Error;

use crate::layout::APPLICATION_IDENTIFIER;

/// The name Windows shows above the toasts.
const DISPLAY_NAME: &str = "Tunneler";

/// Why the identity could not be registered or removed.
#[derive(Debug, Error)]
pub(crate) enum NotificationIdentityError {
    #[error("cannot update the registry: {0}")]
    Registry(io::Error),
}

/// The operating system record of the daemon's notification identity.
pub(crate) trait NotificationIdentity: Send + Sync {
    /// Makes the identity exist. Does nothing when it already does.
    fn register(&self) -> Result<(), NotificationIdentityError>;

    /// Removes the identity. Succeeds when there is none.
    fn remove(&self) -> Result<(), NotificationIdentityError>;
}

/// The identity of the platform the daemon runs on.
pub(crate) fn system_notification_identity() -> Arc<dyn NotificationIdentity> {
    Arc::new(SystemNotificationIdentity)
}

/// The application identity a toast is shown under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToastIdentity {
    /// The daemon's own, registered.
    Tunneler,
    /// The one of PowerShell, when registering failed.
    PowerShell,
}

/// Registers the identity and says which one toasts must use: the daemon's
/// own when it is registered, else PowerShell's, with one warning.
pub(crate) fn register_for_toasts(identity: &dyn NotificationIdentity) -> ToastIdentity {
    match identity.register() {
        Ok(()) => ToastIdentity::Tunneler,
        Err(error) => {
            tracing::warn!(%error, "notifications will appear under the PowerShell identity");
            ToastIdentity::PowerShell
        }
    }
}

/// The registry key of the identity, as shown to the user.
pub(crate) fn describe_registry_key() -> String {
    format!(r"HKCU\Software\Classes\AppUserModelId\{APPLICATION_IDENTIFIER}")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeIdentity {
        is_failing: bool,
    }

    impl NotificationIdentity for FakeIdentity {
        fn register(&self) -> Result<(), NotificationIdentityError> {
            if self.is_failing {
                return Err(NotificationIdentityError::Registry(io::Error::other(
                    "denied",
                )));
            }
            Ok(())
        }

        fn remove(&self) -> Result<(), NotificationIdentityError> {
            Ok(())
        }
    }

    #[test]
    fn should_use_the_identity_of_tunneler_when_the_registration_succeeds() {
        let identity = FakeIdentity { is_failing: false };

        assert_eq!(register_for_toasts(&identity), ToastIdentity::Tunneler);
    }

    #[test]
    fn should_fall_back_to_the_powershell_identity_when_the_registration_fails() {
        let identity = FakeIdentity { is_failing: true };

        assert_eq!(register_for_toasts(&identity), ToastIdentity::PowerShell);
    }

    #[test]
    fn should_describe_the_key_under_the_current_user_classes() {
        assert_eq!(
            describe_registry_key(),
            r"HKCU\Software\Classes\AppUserModelId\com.minimalstuff.tunneler"
        );
    }
}
