use russh::keys::{Algorithm, HashAlg};
use tracing::debug;

const RSA_SIGNATURE_PREFERENCE: [Option<HashAlg>; 3] =
    [Some(HashAlg::Sha512), Some(HashAlg::Sha256), None];

/// Puts the algorithms the host already has a key for first, in the order
/// given, then the remaining defaults, like OpenSSH `order_hostkeyalgs`.
pub(crate) fn order_host_key_algorithms(
    known_algorithm_names: &[String],
    default_algorithms: &[Algorithm],
) -> Vec<Algorithm> {
    let mut ordered = Vec::new();
    let known_algorithms = known_algorithm_names
        .iter()
        .flat_map(|name| algorithms_for_known_name(name));
    for algorithm in known_algorithms.chain(default_algorithms.iter().cloned()) {
        if !ordered.contains(&algorithm) {
            ordered.push(algorithm);
        }
    }
    ordered
}

fn algorithms_for_known_name(name: &str) -> Vec<Algorithm> {
    match Algorithm::new(name) {
        Ok(Algorithm::Rsa { .. }) => RSA_SIGNATURE_PREFERENCE
            .into_iter()
            .map(|hash| Algorithm::Rsa { hash })
            .collect(),
        Ok(algorithm) => vec![algorithm],
        Err(error) => {
            debug!(name, %error, "ignoring a known host key algorithm the client cannot use");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use russh::Preferred;
    use russh::keys::EcdsaCurve;

    use super::*;

    fn defaults() -> Vec<Algorithm> {
        Preferred::DEFAULT.key.to_vec()
    }

    #[test]
    fn should_keep_defaults_when_no_algorithm_is_known() {
        assert_eq!(order_host_key_algorithms(&[], &defaults()), defaults());
    }

    #[test]
    fn should_put_known_algorithm_first_when_it_is_not_the_default_first() {
        let ordered = order_host_key_algorithms(&["ecdsa-sha2-nistp256".to_owned()], &defaults());
        assert_eq!(
            ordered.first(),
            Some(&Algorithm::Ecdsa {
                curve: EcdsaCurve::NistP256
            })
        );
    }

    #[test]
    fn should_expand_rsa_to_the_sha2_signatures_first_when_rsa_is_known() {
        let ordered = order_host_key_algorithms(&["ssh-rsa".to_owned()], &defaults());
        assert_eq!(
            ordered[..3],
            [
                Algorithm::Rsa {
                    hash: Some(HashAlg::Sha512)
                },
                Algorithm::Rsa {
                    hash: Some(HashAlg::Sha256)
                },
                Algorithm::Rsa { hash: None },
            ]
        );
    }

    #[test]
    fn should_not_repeat_an_algorithm_when_it_is_known_and_default() {
        let ordered = order_host_key_algorithms(&["ssh-ed25519".to_owned()], &defaults());
        assert_eq!(ordered.len(), defaults().len());
    }

    #[test]
    fn should_skip_known_name_when_algorithm_is_unknown_to_the_client() {
        let ordered = order_host_key_algorithms(&["no-such-algorithm".to_owned()], &defaults());
        assert_eq!(ordered, defaults());
    }

    #[test]
    fn should_follow_known_order_when_several_algorithms_are_known() {
        let ordered = order_host_key_algorithms(
            &["ecdsa-sha2-nistp256".to_owned(), "ssh-ed25519".to_owned()],
            &defaults(),
        );
        assert_eq!(ordered[1], Algorithm::Ed25519);
    }
}
