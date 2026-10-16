use std::sync::Arc;

use async_trait::async_trait;
use russh::keys::{Certificate, PrivateKey};

use crate::prompter::PromptError;

pub(crate) enum Identity {
    Key {
        key: Arc<PrivateKey>,
        certificate: Option<Certificate>,
    },
}

/// Where public key identities come from: identity files now, an agent later.
///
/// Identities are produced lazily, so that a passphrase is only asked when the
/// previous identities did not already authenticate.
#[async_trait]
pub(crate) trait IdentitySource: Send {
    async fn next_identity(&mut self) -> Result<Option<Identity>, PromptError>;
}
