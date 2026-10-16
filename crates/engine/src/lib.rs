//! Core business logic: sessions, authentication, forwards, states, metrics, health, port scan, store.
//!
//! This slice covers opening an authenticated SSH session to a host alias:
//! [`Connector::connect`] resolves the alias with a [`HostConfigResolver`],
//! opens the transport, verifies the host key through a [`HostKeyVerifier`]
//! against the `known_hosts` files, then authenticates with public keys,
//! keyboard-interactive and password, asking secrets from a
//! [`CredentialPrompter`].
//!
//! The engine depends on those abstractions only, never on IPC or a UI.
//!
//! # Known gaps
//!
//! - Host certificates (`@cert-authority`) are unusable: the client never
//!   advertises certificate host key algorithms, so servers present plain keys.
//! - `ProxyJump`, `ProxyCommand` and agent authentication are not implemented
//!   yet (a host configured with a proxy fails instead of connecting directly);
//!   the transport and the identity sources are the seams they plug into.
//! - The `ConnectTimeout` bound on the handshake pauses while the user is asked
//!   about an unknown host key, but not while an identity file passphrase is
//!   asked (that happens after the handshake).
//! - Behavior on Windows is not verified.
#![warn(missing_docs)]

mod authentication;
mod client_config;
mod connector;
mod error;
mod handshake;
mod host_config;
mod host_key;
mod prompter;
mod session;
mod transport;

pub use authentication::AuthenticationMethod;
pub use connector::{Connector, ConnectorDependencies};
pub use error::{ConnectError, HostKeyFailure, SessionError, TransportError};
pub use host_config::{HostConfigError, HostConfigResolver, SshConfigResolver};
pub use host_key::{
    HostKeyDecision, HostKeyVerifier, RejectedHostKey, RejectionReason, UnknownHostKey,
};
pub use prompter::{
    CredentialPrompter, KeyboardInteractiveRequest, PassphraseRequest, PasswordRequest, Prompt,
    PromptError,
};
pub use session::SshSession;
