use std::fmt::Write;

use super::error::X11Error;

/// The only authentication protocol X11 forwarding speaks.
pub(crate) const MIT_MAGIC_COOKIE_PROTOCOL: &str = "MIT-MAGIC-COOKIE-1";

/// The protocol name as it appears in `Xauthority` files and setup packets.
pub(crate) const MIT_MAGIC_COOKIE_NAME: &[u8] = MIT_MAGIC_COOKIE_PROTOCOL.as_bytes();

/// The length of the cookie given to the server.
pub(crate) const FAKE_COOKIE_LENGTH: usize = 16;

/// The random cookie of one session: the server hands it to X clients, which
/// present it in the setup packet, where it is swapped for the real one.
pub(crate) struct FakeCookie([u8; FAKE_COOKIE_LENGTH]);

impl FakeCookie {
    pub(crate) fn generate() -> Result<Self, X11Error> {
        let mut bytes = [0_u8; FAKE_COOKIE_LENGTH];
        getrandom::fill(&mut bytes).map_err(X11Error::Entropy)?;
        Ok(Self(bytes))
    }

    #[cfg(test)]
    pub(crate) fn from_bytes(bytes: [u8; FAKE_COOKIE_LENGTH]) -> Self {
        Self(bytes)
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn to_hex(&self) -> String {
        self.0.iter().fold(String::new(), |mut hexadecimal, byte| {
            let _ = write!(hexadecimal, "{byte:02x}");
            hexadecimal
        })
    }
}

/// Compares two byte strings in a time that depends on their length only.
pub(crate) fn constant_time_equals(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let difference = left
        .iter()
        .zip(right)
        .fold(0_u8, |accumulated, (left_byte, right_byte)| {
            accumulated | (left_byte ^ right_byte)
        });
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_encode_the_cookie_as_lowercase_hexadecimal_when_asked() {
        let cookie = FakeCookie::from_bytes([
            0x00, 0x01, 0x0a, 0xff, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0,
            0xb0, 0xc0,
        ]);

        assert_eq!(cookie.to_hex(), "00010aff102030405060708090a0b0c0");
    }

    #[test]
    fn should_generate_a_thirty_two_character_hexadecimal_cookie() {
        let hexadecimal = FakeCookie::generate().unwrap().to_hex();

        assert_eq!(hexadecimal.len(), FAKE_COOKIE_LENGTH * 2);
    }

    #[test]
    fn should_generate_different_cookies_when_called_twice() {
        let first = FakeCookie::generate().unwrap();
        let second = FakeCookie::generate().unwrap();

        assert_ne!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn should_report_equal_when_the_bytes_are_identical() {
        assert!(constant_time_equals(b"cookie", b"cookie"));
    }

    #[test]
    fn should_report_different_when_one_byte_differs() {
        assert!(!constant_time_equals(b"cookie", b"cookia"));
    }

    #[test]
    fn should_report_different_when_the_lengths_differ() {
        assert!(!constant_time_equals(b"cookie", b"cookies"));
    }
}
