//! Platforms whose notifications need no identity: nothing to register.

use super::{NotificationIdentity, NotificationIdentityError};

pub(super) struct SystemNotificationIdentity;

impl NotificationIdentity for SystemNotificationIdentity {
    fn register(&self) -> Result<(), NotificationIdentityError> {
        Ok(())
    }

    fn remove(&self) -> Result<(), NotificationIdentityError> {
        Ok(())
    }
}
