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
//! On top of it, [`Engine`] runs the port forwards of the store, local,
//! remote and dynamic (SOCKS5): one task per tunnel, one shared session per
//! host alias, reconnection with backoff, target health, metrics and logs,
//! all reported through [`EngineEvent`]s.
//!
//! While [`Engine::set_port_scanning`] is on, the engine also detects
//! external tunnels: a tunnel whose port is held by an `ssh` process with a
//! matching forward is [`TunnelState::External`], held by anything else it is
//! [`TunnelState::Conflict`], and the other `ssh` forwards are listed in
//! [`EngineSnapshot::externals`].
//!
//! [`Engine::start_x11`] keeps an X11 anchor per host: a session channel that
//! asks the server for X11 forwarding with a throwaway cookie and reports the
//! remote `DISPLAY`. The X11 channels the server then opens reach the local X
//! server, found through the `x11_display` setting, the per-host override or
//! `$DISPLAY`; the throwaway cookie of the X client's setup packet is swapped
//! for the real one read from the `Xauthority` file, so no `xauth` binary is
//! needed. [`Engine::launch_x11`] runs a saved command in such a session. The
//! local display and the files are reached through an [`X11Environment`].
//!
//! Passphrases of encrypted identity files can be remembered: in memory for
//! the life of the engine, and in an OS keyring behind [`SecretKeyring`] when
//! `Settings::use_os_keyring` is on; [`Engine::forget_secrets`] erases both.
//! The tunnels the user wants running and the keyring entries are kept in an
//! [`AppState`], behind a [`StateStore`], apart from the hand-edited
//! configuration; [`Engine::start_launch_tunnels`] starts the ones that
//! `Settings::restore_on_launch` and `Settings::start_on_launch_tags` select.
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
//! - Behavior on Windows is not verified.
#![warn(missing_docs)]

mod agent;
mod authentication;
mod blocking;
mod bounded;
mod client_config;
mod clock;
mod connect_timeouts;
mod connection_chain;
mod connector;
mod error;
mod external;
mod handshake;
mod host_config;
mod host_key;
mod import;
mod keyring;
mod openers;
mod opening;
mod passphrase_vault;
mod prompter;
mod remote_forwards;
mod runtime;
mod session;
mod state_tracker;
mod store;
mod transport;
mod x11;

pub use agent::{AgentEndpoint, AgentLocator, SystemAgentLocator};
pub use authentication::AuthenticationMethod;
pub use clock::{Clock, SystemClock};
pub use connect_timeouts::ConnectTimeouts;
pub use connector::{Connector, ConnectorDependencies};
pub use error::{ConnectError, HostKeyFailure, JumpChainError, SessionError, TransportError};
pub use external::{
    ListeningSocket, PortOccupation, PortOwner, PortScanner, ProcessIdentity, ProcessInfo,
    ScanError, SshDestination, SshForward, SshInvocation, SystemPortScanner, SystemScan,
    TerminateError, UnmanagedProcess, is_ssh_process, parse_ssh_command_line,
};
pub use host_config::{HostConfigError, HostConfigResolver, SshConfigResolver};
pub use host_key::{
    HostKeyDecision, HostKeyVerifier, RejectedHostKey, RejectionReason, UnknownHostKey,
};
pub use import::{
    ImportCandidate, ImportPreview, ImportWarning, preview_script, preview_ssh_config,
};
pub use keyring::{KeyringError, SecretKeyring};
pub use openers::{
    AvailableOpener, DefaultOpener, LaunchCommand, OpenPlan, OpenerSource, PresetPlatform,
    ProgramLocator, SystemProgramLocator, available_openers, default_opener, detect_presets,
};
pub use opening::OpenUrlError;
pub use prompter::{
    CredentialPrompter, KeyboardInteractiveRequest, PassphraseAnswer, PassphraseRequest,
    PasswordRequest, Prompt, PromptError, Retention,
};
pub use runtime::{
    Engine, EngineDependencies, EngineError, EngineEvent, EngineOptions, EngineSnapshot,
    FailureReason, LogLevel, NewTunnel, TargetHealth, TunnelLogEntry, TunnelMetrics,
    TunnelSnapshot, TunnelState, TunnelStatus, X11AnchorSnapshot, X11AnchorState,
};
pub use session::SshSession;
pub use store::{
    AppState, BROWSER_OPENER_ID, FileStateStore, HostOverrides, InvalidTunnelId, Language,
    LoadOutcome, OpenHost, OpenOptions, OpenPathProblem, OpenScheme, Opener, OpenerIdProblem,
    Settings, SourceLocation, StandbyNotify, StandbySound, StateStore, Store, StoreDocument,
    StoreError, StoreEvent, StoreWatcher, TagProblem, TunnelDefinition, TunnelId, TunnelKind,
    TunnelReference, ValidationError, X11Launcher, default_state_path, default_store_path,
};
pub use x11::{DisplayPlatform, SystemX11Environment, X11Environment, X11Error};
