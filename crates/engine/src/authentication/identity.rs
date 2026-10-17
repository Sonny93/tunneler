use std::sync::Arc;

use async_trait::async_trait;
use russh::keys::{Certificate, PrivateKey, PublicKey};
use tokio::sync::Mutex;

use crate::agent::AgentClientConnection;
use crate::error::ConnectError;

pub(crate) enum Identity {
    Key {
        key: Arc<PrivateKey>,
        certificate: Option<Box<Certificate>>,
    },
    Agent {
        public_key: PublicKey,
        client: Arc<Mutex<AgentClientConnection>>,
    },
}

/// Where public key identities come from: the agent, then identity files.
///
/// Identities are produced lazily, so that a passphrase is only asked when the
/// previous identities did not already authenticate.
#[async_trait]
pub(crate) trait IdentitySource: Send {
    async fn next_identity(&mut self) -> Result<Option<Identity>, ConnectError>;
}
