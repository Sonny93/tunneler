use std::collections::VecDeque;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use russh::keys::{
    Certificate, PrivateKey, PublicKey, decode_secret_key, load_openssh_certificate,
};
use tracing::{debug, warn};
use zeroize::Zeroizing;

use super::identity::{Identity, IdentitySource};
use super::identity_files::identity_file_public_key;
use super::offered_keys::OfferedKeys;
use crate::blocking::run_blocking;
use crate::bounded::DeadlinePause;
use crate::error::ConnectError;
use crate::passphrase_vault::PassphraseVault;
use crate::prompter::{CredentialPrompter, PassphraseRequest, PromptError};

const ENCRYPTED_PEM_MARKER: &str = "ENCRYPTED";
const PPK_ENCRYPTION_FIELD: &str = "Encryption:";
const PPK_NO_ENCRYPTION: &str = "none";

pub(crate) struct FileIdentityOptions {
    pub(crate) alias: String,
    pub(crate) key_paths: Vec<PathBuf>,
    pub(crate) certificate_paths: Vec<PathBuf>,
    pub(crate) prompter: Arc<dyn CredentialPrompter>,
    pub(crate) is_prompting_allowed: bool,
    pub(crate) passphrase_attempts: u32,
    pub(crate) offered: Arc<OfferedKeys>,
    pub(crate) passphrases: Arc<PassphraseVault>,
    /// The clock of the authentication phase, paused while the keyring is read.
    pub(crate) deadline_pause: Arc<DeadlinePause>,
}

enum KeyFile {
    Unusable,
    Plain(Box<PrivateKey>),
    Encrypted {
        contents: Arc<Zeroizing<String>>,
        public_key: Option<PublicKey>,
    },
}

/// Identities read from `IdentityFile`s, paired with their `CertificateFile`s
/// and skipping the keys the agent already offered.
pub(crate) struct FileIdentitySource {
    alias: String,
    key_paths: VecDeque<PathBuf>,
    certificate_paths: Vec<PathBuf>,
    certificates: Option<Vec<Certificate>>,
    prompter: Arc<dyn CredentialPrompter>,
    is_prompting_allowed: bool,
    passphrase_attempts: u32,
    offered: Arc<OfferedKeys>,
    passphrases: Arc<PassphraseVault>,
    deadline_pause: Arc<DeadlinePause>,
    pending: VecDeque<Identity>,
}

impl FileIdentitySource {
    pub(crate) fn new(options: FileIdentityOptions) -> Self {
        Self {
            alias: options.alias,
            key_paths: options.key_paths.into(),
            certificate_paths: options.certificate_paths,
            certificates: None,
            prompter: options.prompter,
            is_prompting_allowed: options.is_prompting_allowed,
            passphrase_attempts: options.passphrase_attempts,
            offered: options.offered,
            passphrases: options.passphrases,
            deadline_pause: options.deadline_pause,
            pending: VecDeque::new(),
        }
    }

    async fn load_key(&self, path: &Path) -> Result<Option<Arc<PrivateKey>>, PromptError> {
        let owned_path = path.to_owned();
        let key = match run_blocking(move || inspect_key_file(&owned_path)).await {
            KeyFile::Unusable => return Ok(None),
            KeyFile::Plain(key) => Arc::new(*key),
            KeyFile::Encrypted {
                contents,
                public_key,
            } => {
                if public_key.is_some_and(|public_key| self.offered.contains(&public_key)) {
                    debug!(path = %path.display(), "identity already offered by the agent");
                    return Ok(None);
                }
                match self.unlock(path, contents).await? {
                    Some(key) => Arc::new(key),
                    None => return Ok(None),
                }
            }
        };
        if self.offered.contains(key.public_key()) {
            debug!(path = %path.display(), "identity already offered by the agent");
            return Ok(None);
        }
        Ok(Some(key))
    }

    async fn unlock(
        &self,
        path: &Path,
        contents: Arc<Zeroizing<String>>,
    ) -> Result<Option<PrivateKey>, PromptError> {
        if !self.is_prompting_allowed {
            debug!(path = %path.display(), "skipping an encrypted identity file: prompts are disabled");
            return Ok(None);
        }
        if let Some(key) = self.unlock_with_remembered(path, &contents).await {
            return Ok(Some(key));
        }
        self.unlock_with_prompts(path, contents).await
    }

    async fn unlock_with_remembered(
        &self,
        path: &Path,
        contents: &Arc<Zeroizing<String>>,
    ) -> Option<PrivateKey> {
        let passphrase = self.passphrases.recall(path, &self.deadline_pause).await?;
        let (decrypted, _) = decrypt_off_executor(Arc::clone(contents), passphrase).await;
        if decrypted.is_none() {
            warn!(path = %path.display(), "the remembered passphrase was refused, forgetting it");
            self.passphrases.evict(path, &self.deadline_pause).await;
        }
        decrypted
    }

    async fn unlock_with_prompts(
        &self,
        path: &Path,
        contents: Arc<Zeroizing<String>>,
    ) -> Result<Option<PrivateKey>, PromptError> {
        for _ in 0..self.passphrase_attempts {
            let request = PassphraseRequest {
                alias: self.alias.clone(),
                key_path: path.to_owned(),
            };
            let Some(answer) = self.prompter.passphrase(request).await? else {
                return Ok(None);
            };
            if answer.passphrase.is_empty() {
                return Ok(None);
            }
            let (decrypted, passphrase) =
                decrypt_off_executor(Arc::clone(&contents), answer.passphrase).await;
            if decrypted.is_some() {
                self.passphrases
                    .remember(path, &passphrase, answer.retention, &self.deadline_pause)
                    .await;
                return Ok(decrypted);
            }
            debug!(path = %path.display(), "passphrase refused");
        }
        warn!(path = %path.display(), "no valid passphrase given for the identity file");
        Ok(None)
    }

    async fn ensure_certificates_loaded(&mut self) -> &[Certificate] {
        if self.certificates.is_none() {
            let paths = self.certificate_paths.clone();
            self.certificates = Some(run_blocking(move || load_certificates(&paths)).await);
        }
        self.certificates.as_deref().unwrap_or_default()
    }

    async fn queue_identities(&mut self, key: Arc<PrivateKey>) {
        let matching_certificate = self
            .ensure_certificates_loaded()
            .await
            .iter()
            .find(|certificate| certificate.public_key() == key.public_key().key_data())
            .cloned()
            .map(Box::new);
        if matching_certificate.is_some() {
            self.pending.push_back(Identity::Key {
                key: Arc::clone(&key),
                certificate: matching_certificate,
            });
        }
        self.pending.push_back(Identity::Key {
            key,
            certificate: None,
        });
    }
}

#[async_trait]
impl IdentitySource for FileIdentitySource {
    async fn next_identity(&mut self) -> Result<Option<Identity>, ConnectError> {
        loop {
            if let Some(identity) = self.pending.pop_front() {
                return Ok(Some(identity));
            }
            let Some(path) = self.key_paths.pop_front() else {
                return Ok(None);
            };
            if let Some(key) = self.load_key(&path).await? {
                self.queue_identities(key).await;
            }
        }
    }
}

fn inspect_key_file(path: &Path) -> KeyFile {
    let Some(contents) = read_key_file(path) else {
        return KeyFile::Unusable;
    };
    match decode_secret_key(&contents, None) {
        Ok(key) => KeyFile::Plain(Box::new(key)),
        Err(error) if is_encrypted(&error, &contents) => KeyFile::Encrypted {
            public_key: identity_file_public_key(path),
            contents: Arc::new(contents),
        },
        Err(error) => {
            warn!(path = %path.display(), %error, "cannot read the identity file");
            KeyFile::Unusable
        }
    }
}

async fn decrypt_off_executor(
    contents: Arc<Zeroizing<String>>,
    passphrase: Zeroizing<String>,
) -> (Option<PrivateKey>, Zeroizing<String>) {
    run_blocking(move || (decrypt_key(&contents, &passphrase), passphrase)).await
}

fn decrypt_key(contents: &Zeroizing<String>, passphrase: &Zeroizing<String>) -> Option<PrivateKey> {
    decode_secret_key(contents, Some(passphrase)).ok()
}

fn read_key_file(path: &Path) -> Option<Zeroizing<String>> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Some(Zeroizing::new(contents)),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            debug!(path = %path.display(), "identity file does not exist");
            None
        }
        Err(error) => {
            warn!(path = %path.display(), %error, "cannot read the identity file");
            None
        }
    }
}

fn load_certificates(paths: &[PathBuf]) -> Vec<Certificate> {
    paths
        .iter()
        .filter_map(|path| match load_openssh_certificate(path) {
            Ok(certificate) => Some(certificate),
            Err(error) => {
                debug!(path = %path.display(), %error, "ignoring the certificate file");
                None
            }
        })
        .collect()
}

fn is_encrypted(error: &russh::keys::Error, contents: &str) -> bool {
    matches!(error, russh::keys::Error::KeyIsEncrypted)
        || contents.contains(ENCRYPTED_PEM_MARKER)
        || is_encrypted_ppk(contents)
}

fn is_encrypted_ppk(contents: &str) -> bool {
    contents
        .lines()
        .filter_map(|line| line.strip_prefix(PPK_ENCRYPTION_FIELD))
        .any(|algorithm| algorithm.trim() != PPK_NO_ENCRYPTION)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use russh::keys::ssh_key::LineEnding;
    use russh::keys::{Algorithm, key::safe_rng};
    use tempfile::TempDir;
    use zeroize::Zeroizing;

    use super::*;
    use crate::prompter::{
        KeyboardInteractiveRequest, PassphraseAnswer, PasswordRequest, Retention,
    };
    use crate::store::Settings;

    const PASSPHRASE: &str = "correct horse";

    #[derive(Default)]
    struct StubPrompter {
        passphrases: Mutex<VecDeque<Option<String>>>,
        retention: Retention,
    }

    impl StubPrompter {
        fn answering(answers: &[Option<&str>]) -> Arc<Self> {
            Self::answering_with(Retention::Never, answers)
        }

        fn answering_with(retention: Retention, answers: &[Option<&str>]) -> Arc<Self> {
            let prompter = Self {
                retention,
                ..Self::default()
            };
            *prompter.passphrases.lock().unwrap() = answers
                .iter()
                .map(|answer| answer.map(str::to_owned))
                .collect();
            Arc::new(prompter)
        }
    }

    #[async_trait]
    impl CredentialPrompter for StubPrompter {
        async fn passphrase(
            &self,
            _request: PassphraseRequest,
        ) -> Result<Option<PassphraseAnswer>, PromptError> {
            Ok(self
                .passphrases
                .lock()
                .unwrap()
                .pop_front()
                .flatten()
                .map(|passphrase| PassphraseAnswer {
                    passphrase: Zeroizing::new(passphrase),
                    retention: self.retention,
                }))
        }

        async fn password(
            &self,
            _request: PasswordRequest,
        ) -> Result<Option<Zeroizing<String>>, PromptError> {
            Ok(None)
        }

        async fn keyboard_interactive(
            &self,
            _request: KeyboardInteractiveRequest,
        ) -> Result<Option<Vec<Zeroizing<String>>>, PromptError> {
            Ok(None)
        }
    }

    fn write_key(directory: &TempDir, name: &str, passphrase: Option<&str>) -> PathBuf {
        let key = PrivateKey::random(&mut safe_rng(), Algorithm::Ed25519).unwrap();
        let key = match passphrase {
            Some(passphrase) => key.encrypt(&mut safe_rng(), passphrase).unwrap(),
            None => key,
        };
        let path = directory.path().join(name);
        std::fs::write(&path, key.to_openssh(LineEnding::LF).unwrap().as_bytes()).unwrap();
        path
    }

    fn new_vault() -> Arc<PassphraseVault> {
        let (_, settings) = tokio::sync::watch::channel(Settings::default());
        Arc::new(PassphraseVault::in_memory(settings))
    }

    fn source(
        key_paths: Vec<PathBuf>,
        prompter: Arc<StubPrompter>,
        is_prompting_allowed: bool,
    ) -> FileIdentitySource {
        source_with_vault(key_paths, prompter, is_prompting_allowed, new_vault())
    }

    fn source_with_vault(
        key_paths: Vec<PathBuf>,
        prompter: Arc<StubPrompter>,
        is_prompting_allowed: bool,
        passphrases: Arc<PassphraseVault>,
    ) -> FileIdentitySource {
        FileIdentitySource::new(FileIdentityOptions {
            alias: "host".to_owned(),
            key_paths,
            certificate_paths: Vec::new(),
            prompter,
            is_prompting_allowed,
            passphrase_attempts: 3,
            offered: Arc::new(OfferedKeys::default()),
            passphrases,
            deadline_pause: Arc::new(DeadlinePause::default()),
        })
    }

    async fn count_identities(mut source: FileIdentitySource) -> usize {
        let mut count = 0;
        while source.next_identity().await.unwrap().is_some() {
            count += 1;
        }
        count
    }

    #[tokio::test]
    async fn should_yield_the_key_when_the_identity_file_is_not_encrypted() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_key(&directory, "plain", None);
        let identities = source(vec![path], StubPrompter::answering(&[]), true);
        assert_eq!(count_identities(identities).await, 1);
    }

    #[tokio::test]
    async fn should_skip_the_file_when_it_does_not_exist() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing");
        let identities = source(vec![missing], StubPrompter::answering(&[]), true);
        assert_eq!(count_identities(identities).await, 0);
    }

    #[tokio::test]
    async fn should_skip_the_file_when_it_is_not_a_key() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("garbage");
        std::fs::write(&path, "not a key").unwrap();
        let identities = source(vec![path], StubPrompter::answering(&[]), true);
        assert_eq!(count_identities(identities).await, 0);
    }

    #[tokio::test]
    async fn should_not_prompt_when_the_key_is_encrypted_and_prompts_are_disabled() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_key(&directory, "locked", Some(PASSPHRASE));
        let prompter = StubPrompter::answering(&[Some(PASSPHRASE)]);
        let identities = source(vec![path], Arc::clone(&prompter), false);
        assert_eq!(count_identities(identities).await, 0);
        assert_eq!(prompter.passphrases.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn should_unlock_the_key_when_the_second_passphrase_is_right() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_key(&directory, "locked", Some(PASSPHRASE));
        let prompter = StubPrompter::answering(&[Some("wrong"), Some(PASSPHRASE)]);
        let identities = source(vec![path], prompter, true);
        assert_eq!(count_identities(identities).await, 1);
    }

    #[tokio::test]
    async fn should_skip_the_key_when_the_passphrase_prompt_is_cancelled() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_key(&directory, "locked", Some(PASSPHRASE));
        let identities = source(vec![path], StubPrompter::answering(&[None]), true);
        assert_eq!(count_identities(identities).await, 0);
    }

    #[tokio::test]
    async fn should_skip_the_key_when_the_passphrase_is_empty() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_key(&directory, "locked", Some(PASSPHRASE));
        let identities = source(vec![path], StubPrompter::answering(&[Some("")]), true);
        assert_eq!(count_identities(identities).await, 0);
    }

    #[tokio::test]
    async fn should_not_prompt_again_when_the_passphrase_is_kept_for_the_session() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_key(&directory, "locked", Some(PASSPHRASE));
        let vault = new_vault();
        let first_prompter = StubPrompter::answering_with(Retention::Session, &[Some(PASSPHRASE)]);
        count_identities(source_with_vault(
            vec![path.clone()],
            first_prompter,
            true,
            Arc::clone(&vault),
        ))
        .await;
        let second_prompter = StubPrompter::answering(&[]);

        let identities = source_with_vault(vec![path], Arc::clone(&second_prompter), true, vault);

        assert_eq!(count_identities(identities).await, 1);
    }

    #[tokio::test]
    async fn should_prompt_again_when_the_passphrase_was_not_kept() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_key(&directory, "locked", Some(PASSPHRASE));
        let vault = new_vault();
        let first_prompter = StubPrompter::answering(&[Some(PASSPHRASE)]);
        count_identities(source_with_vault(
            vec![path.clone()],
            first_prompter,
            true,
            Arc::clone(&vault),
        ))
        .await;
        let second_prompter = StubPrompter::answering(&[Some(PASSPHRASE)]);

        let identities = source_with_vault(vec![path], Arc::clone(&second_prompter), true, vault);
        count_identities(identities).await;

        assert_eq!(second_prompter.passphrases.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn should_prompt_and_forget_when_the_kept_passphrase_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let path = write_key(&directory, "locked", Some(PASSPHRASE));
        let vault = new_vault();
        vault
            .remember(
                &path,
                &Zeroizing::new("stale".to_owned()),
                Retention::Session,
                &DeadlinePause::default(),
            )
            .await;
        let prompter = StubPrompter::answering(&[Some(PASSPHRASE)]);

        let identities = source_with_vault(
            vec![path.clone()],
            Arc::clone(&prompter),
            true,
            Arc::clone(&vault),
        );

        assert_eq!(count_identities(identities).await, 1);
        assert!(
            vault
                .recall(&path, &DeadlinePause::default())
                .await
                .is_none()
        );
    }

    #[test]
    fn should_detect_an_encrypted_ppk_when_its_encryption_is_not_none() {
        assert!(is_encrypted_ppk(
            "PuTTY-User-Key-File-3: ssh-ed25519\nEncryption: aes256-cbc\n"
        ));
    }

    #[test]
    fn should_not_detect_an_encrypted_ppk_when_its_encryption_is_none() {
        assert!(!is_encrypted_ppk(
            "PuTTY-User-Key-File-3: ssh-ed25519\nEncryption: none\n"
        ));
    }
}
