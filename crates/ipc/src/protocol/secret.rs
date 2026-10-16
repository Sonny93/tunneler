use std::fmt;

use serde::{Deserialize, Serialize, Serializer};
use zeroize::Zeroizing;

/// A secret typed by the user, such as a passphrase, a password or a
/// keyboard-interactive answer.
///
/// It travels as a plain JSON string, which is unavoidable. On this side it is
/// wiped from memory when dropped, and its `Debug` output is redacted.
///
/// The buffers the JSON parser fills on the way in are not wiped: the
/// guarantee only covers the value once deserialized.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    /// Wraps a secret.
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    /// The secret in clear. Never log it.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Hands the secret over to a consumer that wipes it on its own.
    pub fn into_zeroizing(self) -> Zeroizing<String> {
        self.0
    }
}

impl From<Zeroizing<String>> for Secret {
    fn from(value: Zeroizing<String>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(<redacted>)")
    }
}

impl Serialize for Secret {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET_TEXT: &str = "correct horse battery staple";

    #[test]
    fn should_redact_the_value_when_formatted_with_debug() {
        let secret = Secret::new(SECRET_TEXT.to_owned());

        assert_eq!(format!("{secret:?}"), "Secret(<redacted>)");
    }

    #[test]
    fn should_not_leak_the_value_when_pretty_formatted_with_debug() {
        let secret = Secret::new(SECRET_TEXT.to_owned());

        assert!(!format!("{secret:#?}").contains(SECRET_TEXT));
    }

    #[test]
    fn should_deserialize_from_a_plain_string() {
        let secret: Secret = serde_json::from_str("\"hunter2\"").unwrap();

        assert_eq!(secret.expose(), "hunter2");
    }

    #[test]
    fn should_serialize_as_a_plain_string() {
        let secret = Secret::new("hunter2".to_owned());

        assert_eq!(serde_json::to_string(&secret).unwrap(), "\"hunter2\"");
    }
}
