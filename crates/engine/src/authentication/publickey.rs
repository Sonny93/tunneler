use russh::client::{AuthResult, Handle};
use russh::keys::agent::client::AgentClient;
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKey};
use russh::{AgentAuthError, Error as RusshError};

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
            let Some(result) = try_identity(handle, context, identity, &mut rsa_hash).await? else {
                continue;
            };
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
) -> Result<Option<AuthResult>, ConnectError> {
    match identity {
        Identity::Key { key, certificate } => {
            let user = context.user.as_str();
            if let Some(certificate) = certificate {
                let result = handle
                    .authenticate_openssh_cert(user, key, *certificate)
                    .await?;
                return Ok(Some(result));
            }
            let hash = signature_hash(handle, key.algorithm().is_rsa(), rsa_hash).await?;
            let key_with_hash = PrivateKeyWithHashAlg::new(key, hash);
            Ok(Some(
                handle.authenticate_publickey(user, key_with_hash).await?,
            ))
        }
        Identity::Agent { public_key, client } => {
            let hash = signature_hash(handle, public_key.algorithm().is_rsa(), rsa_hash).await?;
            let mut agent = client.lock().await;
            sign_with_agent(handle, context, public_key, hash, &mut agent).await
        }
    }
}

async fn sign_with_agent(
    handle: &mut Handle<ClientHandler>,
    context: &AuthenticationContext,
    public_key: PublicKey,
    hash: Option<HashAlg>,
    agent: &mut AgentClient<Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>>,
) -> Result<Option<AuthResult>, ConnectError> {
    let outcome = handle
        .authenticate_publickey_with(context.user.as_str(), public_key, hash, agent)
        .await;
    match outcome {
        Ok(result) => Ok(Some(result)),
        Err(AgentAuthError::Send(_)) => Err(ConnectError::Protocol(RusshError::SendError)),
        Err(AgentAuthError::Key(error)) => Err(ConnectError::Agent(Box::new(error))),
    }
}

async fn signature_hash(
    handle: &Handle<ClientHandler>,
    is_rsa: bool,
    cache: &mut RsaHashCache,
) -> Result<Option<HashAlg>, ConnectError> {
    if !is_rsa {
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
