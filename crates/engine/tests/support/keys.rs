use std::path::{Path, PathBuf};

use russh::keys::ssh_key::{Cipher, Kdf, LineEnding};
use russh::keys::{Algorithm, HashAlg, PrivateKey, PublicKey, key::safe_rng};

const FAST_BCRYPT_ROUNDS: u32 = 2;
const BCRYPT_SALT_SIZE: usize = 16;
const CHECK_INTEGER: u32 = 0x1234_5678;

pub fn generate_key(algorithm: Algorithm) -> PrivateKey {
    PrivateKey::random(&mut safe_rng(), algorithm).unwrap()
}

/// Writes `key` in OpenSSH format, encrypted when a passphrase is given.
pub fn write_private_key(
    directory: &Path,
    name: &str,
    key: &PrivateKey,
    passphrase: Option<&str>,
) -> PathBuf {
    let stored = match passphrase {
        Some(passphrase) => encrypt(key, passphrase),
        None => key.clone(),
    };
    let path = directory.join(name);
    std::fs::write(&path, stored.to_openssh(LineEnding::LF).unwrap().as_bytes()).unwrap();
    path
}

/// A key committed under `tests/fixtures`, for algorithms too slow to
/// generate in debug builds (RSA).
pub fn fixture_key(name: &str) -> PrivateKey {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    PrivateKey::from_openssh(std::fs::read(path).unwrap()).unwrap()
}

pub fn fingerprint_of(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

fn encrypt(key: &PrivateKey, passphrase: &str) -> PrivateKey {
    let kdf = Kdf::Bcrypt {
        salt: vec![7; BCRYPT_SALT_SIZE],
        rounds: FAST_BCRYPT_ROUNDS,
    };
    key.encrypt_with(Cipher::Aes256Ctr, kdf, CHECK_INTEGER, passphrase)
        .unwrap()
}
