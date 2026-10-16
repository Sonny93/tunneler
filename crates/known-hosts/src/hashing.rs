use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;

use crate::encoding::LENIENT_BASE64;
use crate::error::Error;

const HASH_MAGIC: &str = "|1|";
const HASH_DELIMITER: char = '|';
const SHA1_OUTPUT_SIZE: usize = 20;
const SALT_SIZE: usize = SHA1_OUTPUT_SIZE;
const MAX_ENCODED_SALT_LENGTH: usize = 1024;

type Salt = [u8; SALT_SIZE];
type Digest = [u8; SHA1_OUTPUT_SIZE];

/// A `|1|salt|hash` host entry: HMAC-SHA1 of the host name keyed by the salt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HashedHost {
    salt: Salt,
    hash: Digest,
}

impl HashedHost {
    pub(crate) fn parse(field: &str) -> Option<Self> {
        let remainder = field.strip_prefix(HASH_MAGIC)?;
        let (encoded_salt, encoded_hash) = remainder.split_once(HASH_DELIMITER)?;
        if encoded_salt.is_empty() || encoded_salt.len() > MAX_ENCODED_SALT_LENGTH {
            return None;
        }
        Some(Self {
            salt: decode_fixed(encoded_salt)?,
            hash: decode_fixed(encoded_hash)?,
        })
    }

    pub(crate) fn with_salt(name: &str, salt: Salt) -> Self {
        Self {
            salt,
            hash: hash_name(&salt, name),
        }
    }

    pub(crate) fn with_random_salt(name: &str) -> Result<Self, Error> {
        let mut salt = [0; SALT_SIZE];
        getrandom::fill(&mut salt).map_err(|source| Error::Entropy { source })?;
        Ok(Self::with_salt(name, salt))
    }

    pub(crate) fn matches(&self, name: &str) -> bool {
        hash_name(&self.salt, name) == self.hash
    }
}

impl fmt::Display for HashedHost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{HASH_MAGIC}{}{HASH_DELIMITER}{}",
            STANDARD.encode(self.salt),
            STANDARD.encode(self.hash)
        )
    }
}

fn decode_fixed<const SIZE: usize>(encoded: &str) -> Option<[u8; SIZE]> {
    let decoded = LENIENT_BASE64.decode(encoded).ok()?;
    decoded.try_into().ok()
}

fn hash_name(salt: &Salt, name: &str) -> Digest {
    let mut mac = <Hmac<Sha1> as KeyInit>::new_from_slice(salt)
        .unwrap_or_else(|_| unreachable!("HMAC accepts keys of any length"));
    mac.update(name.as_bytes());
    mac.finalize().into_bytes().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const EXAMPLE_COM_ENTRY: &str = "|1|gVOLzquvWAX1fO3BhkWWnrpX3Ng=|H5T3S2F+VmdLdwmR2SjFcXfkbnY=";

    #[test]
    fn should_match_name_when_entry_was_generated_by_ssh_keygen() {
        let hashed = HashedHost::parse(EXAMPLE_COM_ENTRY).unwrap();

        assert!(hashed.matches("example.com"));
    }

    #[test]
    fn should_not_match_name_when_entry_was_generated_for_another_name() {
        let hashed = HashedHost::parse(EXAMPLE_COM_ENTRY).unwrap();

        assert!(!hashed.matches("example.org"));
    }

    #[test]
    fn should_round_trip_through_display_when_entry_is_valid() {
        let hashed = HashedHost::parse(EXAMPLE_COM_ENTRY).unwrap();

        assert_eq!(hashed.to_string(), EXAMPLE_COM_ENTRY);
    }

    #[test]
    fn should_match_own_name_when_salt_is_chosen_by_caller() {
        let hashed = HashedHost::with_salt("[host]:2222", [7; SALT_SIZE]);

        assert!(hashed.matches("[host]:2222"));
    }

    #[test]
    fn should_use_different_salts_when_generated_twice() {
        let first = HashedHost::with_random_salt("host").unwrap();
        let second = HashedHost::with_random_salt("host").unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn should_reject_entry_when_magic_is_wrong() {
        assert_eq!(HashedHost::parse("|2|AAAA|AAAA"), None);
    }

    #[test]
    fn should_reject_entry_when_hash_part_is_missing() {
        assert_eq!(HashedHost::parse("|1|gVOLzquvWAX1fO3BhkWWnrpX3Ng="), None);
    }

    #[test]
    fn should_reject_entry_when_salt_is_not_twenty_bytes() {
        assert_eq!(
            HashedHost::parse("|1|AAAA|H5T3S2F+VmdLdwmR2SjFcXfkbnY="),
            None
        );
    }

    #[test]
    fn should_reject_entry_when_hash_is_not_base64() {
        assert_eq!(
            HashedHost::parse("|1|gVOLzquvWAX1fO3BhkWWnrpX3Ng=|!!!"),
            None
        );
    }
}
