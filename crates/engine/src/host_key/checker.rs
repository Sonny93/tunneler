use std::path::PathBuf;
use std::sync::Arc;

use known_hosts::{Hashing, HostKey, HostLookup, KnownHosts, PresentedKey, Verdict, append_entry};
use russh::keys::{PublicKeyBase64, PublicKeyOrCertificate};
use ssh_config::{ResolvedConfig, StrictHostKeyChecking};
use tracing::{info, warn};

use super::accepted_once::AcceptedOnceStore;
use super::policy::{UnknownHostAction, unknown_host_action};
use super::state::HandshakeState;
use super::verifier::{
    HostKeyDecision, HostKeyVerifier, RejectedHostKey, RejectionReason, UnknownHostKey,
};
use crate::blocking::run_blocking;
use crate::error::HostKeyFailure;

/// The parts of the resolved configuration that govern host key decisions.
#[derive(Debug, Clone)]
pub(crate) struct HostKeyPolicy {
    pub(crate) strict_host_key_checking: StrictHostKeyChecking,
    pub(crate) is_batch_mode: bool,
    pub(crate) hashing: Hashing,
    pub(crate) recording_file: Option<PathBuf>,
}

impl HostKeyPolicy {
    pub(crate) fn from_config(config: &ResolvedConfig) -> Self {
        let hashing = if config.hash_known_hosts {
            Hashing::Hashed
        } else {
            Hashing::Plain
        };
        Self {
            strict_host_key_checking: config.strict_host_key_checking,
            is_batch_mode: config.batch_mode,
            hashing,
            recording_file: config.user_known_hosts_files.first().cloned(),
        }
    }
}

pub(crate) struct HostKeyChecker {
    pub(crate) alias: String,
    pub(crate) lookup: HostLookup,
    pub(crate) known_hosts: KnownHosts,
    pub(crate) policy: HostKeyPolicy,
    pub(crate) verifier: Arc<dyn HostKeyVerifier>,
    pub(crate) accepted_once: Arc<AcceptedOnceStore>,
    pub(crate) state: Arc<HandshakeState>,
}

impl HostKeyChecker {
    pub(crate) fn record_failure(&self, failure: HostKeyFailure) {
        self.state.record_failure(failure);
    }

    pub(crate) async fn check(
        &self,
        presented: &PublicKeyOrCertificate,
    ) -> Result<(), HostKeyFailure> {
        let host_key = presented_host_key(presented)?;
        let presented_key = PresentedKey::Plain(host_key.clone());
        let presented_fingerprint = host_key.fingerprint_sha256();
        match self.known_hosts.check(&self.lookup, &presented_key) {
            Verdict::Trusted { .. } => Ok(()),
            Verdict::Changed { path, line, known } => Err(self
                .refuse(HostKeyFailure::Changed {
                    presented_fingerprint,
                    known_fingerprint: known.fingerprint_sha256(),
                    path,
                    line,
                })
                .await),
            Verdict::Revoked { path, line } => Err(self
                .refuse(HostKeyFailure::Revoked {
                    presented_fingerprint,
                    path,
                    line,
                })
                .await),
            Verdict::Unknown => self.admit_unknown(host_key).await,
        }
    }

    async fn admit_unknown(&self, host_key: HostKey) -> Result<(), HostKeyFailure> {
        let action = unknown_host_action(
            self.policy.strict_host_key_checking,
            self.policy.is_batch_mode,
        );
        match action {
            UnknownHostAction::Reject => Err(self
                .refuse(HostKeyFailure::UnknownRejectedByPolicy {
                    presented_fingerprint: host_key.fingerprint_sha256(),
                })
                .await),
            UnknownHostAction::AcceptAndSave => self.record(&host_key).await,
            UnknownHostAction::Ask => self.ask_user(host_key).await,
        }
    }

    async fn ask_user(&self, host_key: HostKey) -> Result<(), HostKeyFailure> {
        let lookup_name = self.lookup.lookup_name();
        if self.accepted_once.contains(&lookup_name, &host_key) {
            return Ok(());
        }
        let decision = self.ask_verifier(&lookup_name, &host_key).await;
        match decision {
            HostKeyDecision::AcceptAndSave => self.record(&host_key).await,
            HostKeyDecision::AcceptOnce => {
                self.accepted_once.remember(&lookup_name, &host_key);
                Ok(())
            }
            HostKeyDecision::Reject => Err(HostKeyFailure::RejectedByUser {
                presented_fingerprint: host_key.fingerprint_sha256(),
            }),
        }
    }

    async fn ask_verifier(&self, lookup_name: &str, host_key: &HostKey) -> HostKeyDecision {
        let request = UnknownHostKey {
            alias: self.alias.clone(),
            lookup_name: lookup_name.to_owned(),
            algorithm: host_key.algorithm.clone(),
            fingerprint_sha256: host_key.fingerprint_sha256(),
        };
        self.state.begin_awaiting_user();
        let decision = self.verifier.decide_unknown(request).await;
        self.state.end_awaiting_user();
        decision
    }

    async fn record(&self, host_key: &HostKey) -> Result<(), HostKeyFailure> {
        let Some(path) = &self.policy.recording_file else {
            warn!(alias = %self.alias, "no user known_hosts file: the host key is accepted but not recorded");
            return Ok(());
        };
        let (entry_path, lookup, key, hashing) = (
            path.clone(),
            self.lookup.clone(),
            host_key.clone(),
            self.policy.hashing,
        );
        run_blocking(move || append_entry(&entry_path, &lookup, &key, hashing))
            .await
            .map_err(|source| HostKeyFailure::NotRecorded { source })?;
        info!(alias = %self.alias, path = %path.display(), "recorded a new host key");
        Ok(())
    }

    async fn refuse(&self, failure: HostKeyFailure) -> HostKeyFailure {
        if let Some(rejection) = rejection_report(&self.alias, &failure) {
            self.verifier.report_rejected(rejection).await;
        }
        failure
    }
}

fn presented_host_key(presented: &PublicKeyOrCertificate) -> Result<HostKey, HostKeyFailure> {
    let PublicKeyOrCertificate::PublicKey { key, .. } = presented else {
        return Err(HostKeyFailure::UnsupportedKey {
            reason: "host certificates are not supported".to_owned(),
        });
    };
    HostKey::new(key.algorithm().as_str(), key.public_key_bytes()).map_err(|error| {
        HostKeyFailure::UnsupportedKey {
            reason: error.to_string(),
        }
    })
}

fn rejection_report(alias: &str, failure: &HostKeyFailure) -> Option<RejectedHostKey> {
    let (reason, presented_fingerprint) = match failure {
        HostKeyFailure::Changed {
            presented_fingerprint,
            known_fingerprint,
            path,
            line,
        } => (
            RejectionReason::Changed {
                known_fingerprint: known_fingerprint.clone(),
                path: path.clone(),
                line: *line,
            },
            presented_fingerprint,
        ),
        HostKeyFailure::Revoked {
            presented_fingerprint,
            path,
            line,
        } => (
            RejectionReason::Revoked {
                path: path.clone(),
                line: *line,
            },
            presented_fingerprint,
        ),
        HostKeyFailure::UnknownRejectedByPolicy {
            presented_fingerprint,
        } => (
            RejectionReason::UnknownRejectedByPolicy,
            presented_fingerprint,
        ),
        HostKeyFailure::RejectedByUser { .. }
        | HostKeyFailure::UnsupportedKey { .. }
        | HostKeyFailure::NotRecorded { .. } => return None,
    };
    Some(RejectedHostKey {
        alias: alias.to_owned(),
        reason,
        presented_fingerprint: presented_fingerprint.clone(),
    })
}

#[cfg(test)]
mod tests {
    use russh::keys::{Algorithm, HashAlg, PrivateKey, key::safe_rng};

    use super::*;

    fn ed25519_public_key() -> russh::keys::PublicKey {
        PrivateKey::random(&mut safe_rng(), Algorithm::Ed25519)
            .unwrap()
            .public_key()
            .clone()
    }

    #[test]
    fn should_convert_to_ed25519_algorithm_when_key_is_ed25519() {
        let presented = PublicKeyOrCertificate::from(ed25519_public_key());
        assert_eq!(
            presented_host_key(&presented).unwrap().algorithm,
            "ssh-ed25519"
        );
    }

    #[test]
    fn should_keep_the_openssh_fingerprint_when_converting_the_key() {
        let public_key = ed25519_public_key();
        let expected = public_key.fingerprint(HashAlg::Sha256).to_string();
        let presented = PublicKeyOrCertificate::from(public_key);
        assert_eq!(
            presented_host_key(&presented).unwrap().fingerprint_sha256(),
            expected
        );
    }

    #[test]
    fn should_not_report_when_the_user_rejected_the_key() {
        let failure = HostKeyFailure::RejectedByUser {
            presented_fingerprint: "SHA256:abc".to_owned(),
        };
        assert_eq!(rejection_report("alias", &failure), None);
    }

    #[test]
    fn should_report_the_policy_when_unknown_key_is_rejected_by_policy() {
        let failure = HostKeyFailure::UnknownRejectedByPolicy {
            presented_fingerprint: "SHA256:abc".to_owned(),
        };
        assert_eq!(
            rejection_report("alias", &failure).map(|report| report.reason),
            Some(RejectionReason::UnknownRejectedByPolicy)
        );
    }

    #[test]
    fn should_report_the_recorded_key_when_the_key_changed() {
        let failure = HostKeyFailure::Changed {
            presented_fingerprint: "SHA256:new".to_owned(),
            known_fingerprint: "SHA256:old".to_owned(),
            path: PathBuf::from("known_hosts"),
            line: 3,
        };
        assert_eq!(
            rejection_report("alias", &failure).map(|report| report.reason),
            Some(RejectionReason::Changed {
                known_fingerprint: "SHA256:old".to_owned(),
                path: PathBuf::from("known_hosts"),
                line: 3,
            })
        );
    }
}
