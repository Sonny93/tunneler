use std::fmt;

use russh::MethodKind;
use ssh_config::{PubkeyAuthentication, ResolvedConfig};

/// A way of proving who the user is to the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthenticationMethod {
    /// A private key, from an identity file.
    PublicKey,
    /// Questions and answers, such as a one-time code.
    KeyboardInteractive,
    /// The account password.
    Password,
}

const DEFAULT_ORDER: [AuthenticationMethod; 3] = [
    AuthenticationMethod::PublicKey,
    AuthenticationMethod::KeyboardInteractive,
    AuthenticationMethod::Password,
];

impl AuthenticationMethod {
    fn name(self) -> &'static str {
        match self {
            Self::PublicKey => "publickey",
            Self::KeyboardInteractive => "keyboard-interactive",
            Self::Password => "password",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        DEFAULT_ORDER
            .into_iter()
            .find(|method| method.name().eq_ignore_ascii_case(name))
    }

    pub(crate) fn kind(self) -> MethodKind {
        match self {
            Self::PublicKey => MethodKind::PublicKey,
            Self::KeyboardInteractive => MethodKind::KeyboardInteractive,
            Self::Password => MethodKind::Password,
        }
    }
}

impl fmt::Display for AuthenticationMethod {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct MethodPolicy {
    pub(crate) preferred: Option<Vec<String>>,
    pub(crate) is_publickey_enabled: bool,
    pub(crate) is_keyboard_interactive_enabled: bool,
    pub(crate) is_password_enabled: bool,
    pub(crate) is_batch_mode: bool,
}

impl MethodPolicy {
    pub(crate) fn from_config(config: &ResolvedConfig) -> Self {
        Self {
            preferred: config.preferred_authentications.clone(),
            is_publickey_enabled: config.pubkey_authentication != PubkeyAuthentication::No,
            is_keyboard_interactive_enabled: config.kbd_interactive_authentication,
            is_password_enabled: config.password_authentication,
            is_batch_mode: config.batch_mode,
        }
    }

    pub(crate) fn planned_methods(&self) -> Vec<AuthenticationMethod> {
        self.ordered_methods()
            .into_iter()
            .filter(|method| self.is_allowed(*method))
            .collect()
    }

    fn ordered_methods(&self) -> Vec<AuthenticationMethod> {
        let Some(names) = &self.preferred else {
            return DEFAULT_ORDER.to_vec();
        };
        let mut ordered = Vec::new();
        for method in names
            .iter()
            .filter_map(|name| AuthenticationMethod::from_name(name))
        {
            if !ordered.contains(&method) {
                ordered.push(method);
            }
        }
        ordered
    }

    fn is_allowed(&self, method: AuthenticationMethod) -> bool {
        match method {
            AuthenticationMethod::PublicKey => self.is_publickey_enabled,
            AuthenticationMethod::KeyboardInteractive => {
                self.is_keyboard_interactive_enabled && !self.is_batch_mode
            }
            AuthenticationMethod::Password => self.is_password_enabled && !self.is_batch_mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> MethodPolicy {
        MethodPolicy {
            preferred: None,
            is_publickey_enabled: true,
            is_keyboard_interactive_enabled: true,
            is_password_enabled: true,
            is_batch_mode: false,
        }
    }

    fn preferring(names: &[&str]) -> MethodPolicy {
        MethodPolicy {
            preferred: Some(names.iter().map(|name| (*name).to_owned()).collect()),
            ..policy()
        }
    }

    #[test]
    fn should_try_publickey_then_keyboard_interactive_then_password_when_nothing_is_configured() {
        assert_eq!(policy().planned_methods(), DEFAULT_ORDER.to_vec());
    }

    #[test]
    fn should_follow_the_preferred_order_when_it_is_set() {
        assert_eq!(
            preferring(&["password", "publickey"]).planned_methods(),
            vec![
                AuthenticationMethod::Password,
                AuthenticationMethod::PublicKey
            ]
        );
    }

    #[test]
    fn should_ignore_methods_the_engine_does_not_implement_when_they_are_preferred() {
        assert_eq!(
            preferring(&["gssapi-with-mic", "hostbased", "password"]).planned_methods(),
            vec![AuthenticationMethod::Password]
        );
    }

    #[test]
    fn should_not_repeat_a_method_when_it_is_preferred_twice() {
        assert_eq!(
            preferring(&["password", "password"]).planned_methods(),
            vec![AuthenticationMethod::Password]
        );
    }

    #[test]
    fn should_drop_publickey_when_it_is_disabled() {
        let disabled = MethodPolicy {
            is_publickey_enabled: false,
            ..policy()
        };
        assert_eq!(
            disabled.planned_methods(),
            vec![
                AuthenticationMethod::KeyboardInteractive,
                AuthenticationMethod::Password
            ]
        );
    }

    #[test]
    fn should_drop_password_when_it_is_disabled() {
        let disabled = MethodPolicy {
            is_password_enabled: false,
            ..policy()
        };
        assert!(
            !disabled
                .planned_methods()
                .contains(&AuthenticationMethod::Password)
        );
    }

    #[test]
    fn should_drop_keyboard_interactive_when_it_is_disabled() {
        let disabled = MethodPolicy {
            is_keyboard_interactive_enabled: false,
            ..policy()
        };
        assert!(
            !disabled
                .planned_methods()
                .contains(&AuthenticationMethod::KeyboardInteractive)
        );
    }

    #[test]
    fn should_keep_only_publickey_when_in_batch_mode() {
        let batch = MethodPolicy {
            is_batch_mode: true,
            ..policy()
        };
        assert_eq!(
            batch.planned_methods(),
            vec![AuthenticationMethod::PublicKey]
        );
    }

    #[test]
    fn should_match_method_names_regardless_of_case_when_preferred() {
        assert_eq!(
            preferring(&["Password"]).planned_methods(),
            vec![AuthenticationMethod::Password]
        );
    }
}
