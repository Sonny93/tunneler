//! Core business logic: sessions, authentication, forwards, states, metrics, health, port scan, store.
//!
//! This slice covers opening an authenticated SSH session to a host alias:
//! [`Connector::connect`] resolves the alias with a [`HostConfigResolver`],
//! opens the transport, verifies the host key through a [`HostKeyVerifier`]
//! against the `known_hosts` files, then authenticates with public keys,
//! keyboard-interactive and password, asking secrets from a
//! [`CredentialPrompter`]. Hosts can be reached directly, through a
//! `ProxyCommand` or through `ProxyJump` hosts; identities come from the SSH
//! agent first, then from identity files.
//!
//! The engine depends on those abstractions only, never on IPC or a UI.
//!
//! # Known gaps
//!
//! - Host certificates (`@cert-authority`) are unusable: the client never
//!   advertises certificate host key algorithms, so servers present plain keys.
//! - `AddKeysToAgent` is not implemented: keys are never added to the agent.
//! - Certificates held by the agent are ignored; only plain agent keys are used.
//! - The agent on Windows (OpenSSH named pipe, then Pageant) is wired through
//!   russh but has never been run.
//! - The `ConnectTimeout` bound on the handshake pauses while the user is asked
//!   about an unknown host key, but not while an identity file passphrase is
//!   asked (that happens after the handshake).
//! - Behavior on Windows is not verified.
#![warn(missing_docs)]

mod agent;
mod authentication;
mod blocking;
mod client_config;
mod connection_chain;
mod connector;
mod error;
mod handshake;
mod host_config;
mod host_key;
mod prompter;
mod session;
mod store;
mod transport;

pub use agent::{AgentEndpoint, AgentLocator, SystemAgentLocator};
pub use authentication::AuthenticationMethod;
pub use connector::{Connector, ConnectorDependencies};
pub use error::{ConnectError, HostKeyFailure, JumpChainError, SessionError, TransportError};
pub use host_config::{HostConfigError, HostConfigResolver, SshConfigResolver};
pub use host_key::{
    HostKeyDecision, HostKeyVerifier, RejectedHostKey, RejectionReason, UnknownHostKey,
};
pub use prompter::{
    CredentialPrompter, KeyboardInteractiveRequest, PassphraseRequest, PasswordRequest, Prompt,
    PromptError,
};
pub use session::SshSession;
pub use store::{
    HostOverrides, InvalidTunnelId, LoadOutcome, Settings, SourceLocation, Store, StoreDocument,
    StoreError, StoreEvent, StoreWatcher, TagProblem, TunnelDefinition, TunnelId, TunnelKind,
    TunnelReference, ValidationError, X11Launcher, default_store_path,
};
