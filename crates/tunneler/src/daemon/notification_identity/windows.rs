//! Windows: the key `HKCU\Software\Classes\AppUserModelId\<identifier>` with a
//! `DisplayName` value.
//!
//! This code is only cross-compiled in development; it has never been run.

use std::io;

use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

use super::{DISPLAY_NAME, NotificationIdentity, NotificationIdentityError};
use crate::layout::APPLICATION_IDENTIFIER;

const CLASSES_KEY_PATH: &str = r"Software\Classes\AppUserModelId";
const DISPLAY_NAME_VALUE: &str = "DisplayName";

pub(super) struct SystemNotificationIdentity;

impl NotificationIdentity for SystemNotificationIdentity {
    fn register(&self) -> Result<(), NotificationIdentityError> {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(key_path())
            .map_err(NotificationIdentityError::Registry)?;
        key.set_value(DISPLAY_NAME_VALUE, &DISPLAY_NAME)
            .map_err(NotificationIdentityError::Registry)
    }

    fn remove(&self) -> Result<(), NotificationIdentityError> {
        match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(key_path()) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => {
                Err(NotificationIdentityError::Registry(error))
            }
            _ => Ok(()),
        }
    }
}

fn key_path() -> String {
    format!(r"{CLASSES_KEY_PATH}\{APPLICATION_IDENTIFIER}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_put_the_key_under_the_current_user_classes() {
        assert_eq!(
            key_path(),
            r"Software\Classes\AppUserModelId\com.minimalstuff.tunneler"
        );
    }
}
