use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::encoding::LENIENT_BASE64;

const LENGTH_PREFIX_SIZE: usize = 4;
const FINGERPRINT_PREFIX: &str = "SHA256:";

/// Why a host key cannot be built.
#[allow(
    missing_docs,
    reason = "the error messages describe the variants and their fields"
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HostKeyError {
    #[error("key is not valid base64")]
    InvalidBase64,
    #[error("key blob is truncated")]
    TruncatedBlob,
    #[error("key type \"{declared}\" does not match the key blob algorithm \"{embedded}\"")]
    AlgorithmMismatch { declared: String, embedded: String },
}

/// A public host key: its algorithm name and its SSH wire-format blob.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostKey {
    /// The algorithm name, such as `ssh-ed25519` or `ecdsa-sha2-nistp256`.
    pub algorithm: String,
    /// The public key in SSH wire format (what the base64 field of a
    /// `known_hosts` line decodes to).
    pub blob: Vec<u8>,
}

impl HostKey {
    /// A key from its algorithm name and blob.
    ///
    /// Fails when the blob does not start with the same algorithm name.
    pub fn new(algorithm: impl Into<String>, blob: Vec<u8>) -> Result<Self, HostKeyError> {
        let algorithm = algorithm.into();
        let embedded = embedded_algorithm(&blob)?;
        if embedded != algorithm.as_bytes() {
            return Err(HostKeyError::AlgorithmMismatch {
                declared: algorithm,
                embedded: String::from_utf8_lossy(embedded).into_owned(),
            });
        }
        Ok(Self { algorithm, blob })
    }

    /// A key from its algorithm name and the base64 text of its blob.
    pub fn from_base64(algorithm: &str, encoded: &str) -> Result<Self, HostKeyError> {
        let blob = LENIENT_BASE64
            .decode(encoded)
            .map_err(|_| HostKeyError::InvalidBase64)?;
        Self::new(algorithm, blob)
    }

    /// The blob as padded base64, as written in a `known_hosts` line.
    pub fn to_base64(&self) -> String {
        STANDARD.encode(&self.blob)
    }

    /// The OpenSSH SHA-256 fingerprint: `SHA256:` and unpadded base64.
    pub fn fingerprint_sha256(&self) -> String {
        let digest = Sha256::digest(&self.blob);
        format!("{FINGERPRINT_PREFIX}{}", STANDARD_NO_PAD.encode(digest))
    }
}

fn embedded_algorithm(blob: &[u8]) -> Result<&[u8], HostKeyError> {
    let (length_bytes, remainder) = blob
        .split_first_chunk::<LENGTH_PREFIX_SIZE>()
        .ok_or(HostKeyError::TruncatedBlob)?;
    let length = usize::try_from(u32::from_be_bytes(*length_bytes))
        .map_err(|_| HostKeyError::TruncatedBlob)?;
    remainder.get(..length).ok_or(HostKeyError::TruncatedBlob)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    pub(crate) fn blob_for(algorithm: &str, payload: &[u8]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&u32::try_from(algorithm.len()).unwrap().to_be_bytes());
        blob.extend_from_slice(algorithm.as_bytes());
        blob.extend_from_slice(payload);
        blob
    }

    pub(crate) fn key_for(algorithm: &str, payload: &[u8]) -> HostKey {
        HostKey::new(algorithm, blob_for(algorithm, payload)).unwrap()
    }

    #[test]
    fn should_build_key_when_blob_starts_with_declared_algorithm() {
        let blob = blob_for("ssh-ed25519", b"payload");

        assert!(HostKey::new("ssh-ed25519", blob).is_ok());
    }

    #[test]
    fn should_reject_key_when_blob_algorithm_differs() {
        let blob = blob_for("ssh-rsa", b"payload");

        assert_eq!(
            HostKey::new("ssh-ed25519", blob),
            Err(HostKeyError::AlgorithmMismatch {
                declared: "ssh-ed25519".to_owned(),
                embedded: "ssh-rsa".to_owned(),
            })
        );
    }

    #[test]
    fn should_reject_key_when_blob_is_shorter_than_length_prefix() {
        assert_eq!(
            HostKey::new("ssh-ed25519", vec![0, 0]),
            Err(HostKeyError::TruncatedBlob)
        );
    }

    #[test]
    fn should_reject_key_when_algorithm_name_is_truncated() {
        let mut blob = blob_for("ssh-ed25519", b"");
        blob.truncate(8);

        assert_eq!(
            HostKey::new("ssh-ed25519", blob),
            Err(HostKeyError::TruncatedBlob)
        );
    }

    #[test]
    fn should_reject_key_when_base64_is_invalid() {
        assert_eq!(
            HostKey::from_base64("ssh-ed25519", "not base64!"),
            Err(HostKeyError::InvalidBase64)
        );
    }

    #[test]
    fn should_round_trip_base64_when_key_is_valid() {
        let key = key_for("ssh-ed25519", b"payload");

        assert_eq!(
            HostKey::from_base64("ssh-ed25519", &key.to_base64()),
            Ok(key)
        );
    }

    #[test]
    fn should_format_fingerprint_like_openssh_when_key_is_valid() {
        let key = HostKey::from_base64(
            "ssh-ed25519",
            "AAAAC3NzaC1lZDI1NTE5AAAAIIQ9/YDGdoYkoifwHjSv26LdePTYBpLXBvfuDG9Qeg5I",
        )
        .unwrap();

        assert_eq!(
            key.fingerprint_sha256(),
            "SHA256:UWsbrv1PEWMQZg80SGAUVPP5WpL58SxQMr5KoY3vFtI"
        );
    }
}
