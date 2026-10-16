use std::path::Path;

use russh::keys::{PrivateKey, PublicKey, load_public_key};

const PUBLIC_KEY_SUFFIX: &str = ".pub";

/// The public key of an identity file without decrypting it: from its `.pub`
/// sibling, or from the clear-text part of an OpenSSH-format private key.
pub(crate) fn identity_file_public_key(path: &Path) -> Option<PublicKey> {
    let mut sibling = path.as_os_str().to_owned();
    sibling.push(PUBLIC_KEY_SUFFIX);
    if let Ok(public_key) = load_public_key(&sibling) {
        return Some(public_key);
    }
    let contents = std::fs::read(path).ok()?;
    let private_key = PrivateKey::from_openssh(contents).ok()?;
    Some(private_key.public_key().clone())
}

#[cfg(test)]
mod tests {
    use russh::keys::Algorithm;
    use russh::keys::key::safe_rng;
    use russh::keys::ssh_key::LineEnding;

    use super::*;

    fn write_private_key(directory: &Path) -> (std::path::PathBuf, PublicKey) {
        let key = PrivateKey::random(&mut safe_rng(), Algorithm::Ed25519).unwrap();
        let path = directory.join("id");
        std::fs::write(&path, key.to_openssh(LineEnding::LF).unwrap().as_bytes()).unwrap();
        (path, key.public_key().clone())
    }

    #[test]
    fn should_read_the_public_part_when_the_private_key_is_in_openssh_format() {
        let directory = tempfile::tempdir().unwrap();
        let (path, public_key) = write_private_key(directory.path());
        assert_eq!(identity_file_public_key(&path), Some(public_key));
    }

    #[test]
    fn should_prefer_the_pub_sibling_when_it_exists() {
        let directory = tempfile::tempdir().unwrap();
        let (path, _) = write_private_key(directory.path());
        let other = PrivateKey::random(&mut safe_rng(), Algorithm::Ed25519).unwrap();
        let pub_path = directory.path().join("id.pub");
        std::fs::write(&pub_path, other.public_key().to_openssh().unwrap()).unwrap();
        assert_eq!(
            identity_file_public_key(&path),
            Some(other.public_key().clone())
        );
    }

    #[test]
    fn should_find_nothing_when_the_file_is_missing() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            identity_file_public_key(&directory.path().join("absent")),
            None
        );
    }
}
