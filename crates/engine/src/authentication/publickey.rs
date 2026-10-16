use std::sync::Arc;

use russh::client::{AuthResult, Handle};
use russh::keys::{HashAlg, PrivateKey, PrivateKeyWithHashAlg};

use super::flow::AuthenticationContext;
use super::identity::{Identity, IdentitySource};
use super::outcome::MethodOutcome;
use crate::error::ConnectError;
use crate::host_key::ClientHandler;

const FALLBACK_RSA_HASH: HashAlg = HashAlg::Sha256;

enum RsaHashCache {
    Unresolved,
    Resolved(Option<HashAlg>),
}

pub(crate) async fn attempt(
    handle: &mut Handle<ClientHandler>,
    context: &AuthenticationContext,
    sources: &mut [Box<dyn IdentitySource>],
) -> Result<MethodOutcome, ConnectError> {
    let mut rsa_hash = RsaHashCache::Unresolved;
    let mut last_rejection = None;
    for source in sources {
        while let Some(identity) = source.next_identity().await? {
            let result = try_identity(handle, context, identity, &mut rsa_hash).await?;
            match result {
                AuthResult::Success => return Ok(MethodOutcome::Authenticated),
                AuthResult::Failure {
                    remaining_methods,
                    partial_success: true,
                } => {
                    return Ok(MethodOutcome::Rejected {
                        remaining_methods,
                        partial_success: true,
                    });
                }
                AuthResult::Failure {
                    remaining_methods, ..
                } => last_rejection = Some(remaining_methods),
            }
        }
    }
    Ok(match last_rejection {
        Some(remaining_methods) => MethodOutcome::Rejected {
            remaining_methods,
            partial_success: false,
        },
        None => MethodOutcome::NotAttempted,
    })
}

async fn try_identity(
    handle: &mut Handle<ClientHandler>,
    context: &AuthenticationContext,
    identity: Identity,
    rsa_hash: &mut RsaHashCache,
) -> Result<AuthResult, ConnectError> {
    let Identity::Key { key, certificate } = identity;
    let user = context.user.as_str();
    if let Some(certificate) = certificate {
        return Ok(handle
            .authenticate_openssh_cert(user, key, certificate)
            .await?);
    }
    let hash = signature_hash(handle, &key, rsa_hash).await?;
    let key_with_hash = PrivateKeyWithHashAlg::new(key, hash);
    Ok(handle.authenticate_publickey(user, key_with_hash).await?)
}

async fn signature_hash(
    handle: &Handle<ClientHandler>,
    key: &Arc<PrivateKey>,
    cache: &mut RsaHashCache,
) -> Result<Option<HashAlg>, ConnectError> {
    if !key.algorithm().is_rsa() {
        return Ok(None);
    }
    if let RsaHashCache::Resolved(hash) = cache {
        return Ok(*hash);
    }
    let hash = handle
        .best_supported_rsa_hash()
        .await?
        .unwrap_or(Some(FALLBACK_RSA_HASH));
    *cache = RsaHashCache::Resolved(hash);
    Ok(hash)
}
