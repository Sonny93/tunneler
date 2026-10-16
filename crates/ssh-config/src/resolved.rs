use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::directive::{
    AddKeysToAgent, AddressFamily, DynamicForward, EnvironmentVariable, ForwardSpec, JumpHost,
    PubkeyAuthentication, RemoteForward, StrictHostKeyChecking,
};
use crate::error::Warning;

/// Where an agent socket comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSocket {
    /// The socket path is the value of this environment variable.
    Environment(String),
    /// The socket is at this path.
    Path(PathBuf),
}

/// `IdentityAgent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityAgent {
    /// `none`: do not use an agent.
    Disabled,
    /// Use the agent at this socket.
    Socket(AgentSocket),
}

/// `ForwardAgent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardAgent {
    /// Agent forwarding is off.
    Disabled,
    /// Forward the agent found through the usual `SSH_AUTH_SOCK`.
    Enabled,
    /// Forward the agent at this socket.
    Socket(AgentSocket),
}

/// The configuration of one host after OpenSSH-compatible evaluation, defaults
/// and token expansion: what `ssh -G` would report, in typed form.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct ResolvedConfig {
    /// The alias as given by the caller (`%n`).
    pub alias: String,
    /// The host to connect to, lowercased unless it is an IP address.
    pub hostname: String,
    /// The remote port; 22 by default.
    pub port: u16,
    /// The remote login name, `%` tokens expanded; the local user by default.
    pub user: String,
    /// Private key files to try, in order, expanded; the OpenSSH defaults when none is set.
    pub identity_files: Vec<PathBuf>,
    /// Certificate files, in order, expanded.
    pub certificate_files: Vec<PathBuf>,
    /// `IdentitiesOnly`: only use the configured identities.
    pub identities_only: bool,
    /// `None` when `IdentityAgent` is not set.
    pub identity_agent: Option<IdentityAgent>,
    /// `None` when unset or `none`.
    pub proxy_command: Option<String>,
    /// Empty when unset or `none`.
    pub proxy_jump: Vec<JumpHost>,
    /// `LocalForward`s, in order, without duplicates; empty under `ClearAllForwardings`.
    pub local_forwards: Vec<ForwardSpec>,
    /// `RemoteForward`s, in order, without duplicates; empty under `ClearAllForwardings`.
    pub remote_forwards: Vec<RemoteForward>,
    /// `DynamicForward`s, in order, without duplicates; empty under `ClearAllForwardings`.
    pub dynamic_forwards: Vec<DynamicForward>,
    /// `ForwardX11`.
    pub forward_x11: bool,
    /// `ForwardX11Trusted`; `false` by default, as upstream OpenSSH (some distributions differ).
    pub forward_x11_trusted: bool,
    /// `ForwardAgent`.
    pub forward_agent: ForwardAgent,
    /// `ServerAliveInterval`; zero disables keepalive messages. `None` when no
    /// line sets it, so the caller can apply its own default.
    pub server_alive_interval: Option<Duration>,
    /// `ServerAliveCountMax`. `None` when no line sets it (OpenSSH then uses 3).
    pub server_alive_count_max: Option<u32>,
    /// `StrictHostKeyChecking`; `Ask` by default.
    pub strict_host_key_checking: StrictHostKeyChecking,
    /// Empty when `none`.
    pub user_known_hosts_files: Vec<PathBuf>,
    /// Empty when `none`.
    pub global_known_hosts_files: Vec<PathBuf>,
    /// `HashKnownHosts`.
    pub hash_known_hosts: bool,
    /// Lowercased.
    pub host_key_alias: Option<String>,
    /// `PreferredAuthentications` as a list; `None` when unset.
    pub preferred_authentications: Option<Vec<String>>,
    /// `PubkeyAuthentication`; `Yes` by default.
    pub pubkey_authentication: PubkeyAuthentication,
    /// `PasswordAuthentication`; `true` by default.
    pub password_authentication: bool,
    /// `KbdInteractiveAuthentication`; `true` by default.
    pub kbd_interactive_authentication: bool,
    /// `NumberOfPasswordPrompts`; 3 by default.
    pub number_of_password_prompts: u32,
    /// `None` when unset or `none`.
    pub connect_timeout: Option<Duration>,
    /// `ConnectionAttempts`; 1 by default.
    pub connection_attempts: u32,
    /// `AddKeysToAgent`; `No` by default.
    pub add_keys_to_agent: AddKeysToAgent,
    /// `AddressFamily`; `Any` by default.
    pub address_family: AddressFamily,
    /// `BatchMode`.
    pub batch_mode: bool,
    /// `ExitOnForwardFailure`.
    pub exit_on_forward_failure: bool,
    /// `GatewayPorts`.
    pub gateway_ports: bool,
    /// `ClearAllForwardings`; when set, the forward lists above are empty.
    pub clear_all_forwardings: bool,
    /// `TCPKeepAlive`; `true` by default.
    pub tcp_keep_alive: bool,
    /// `Compression`.
    pub compression: bool,
    /// `SendEnv` patterns accumulated in order, after `-pattern` removals.
    pub send_env: Vec<String>,
    /// `SetEnv` assignments with values expanded; the first `SetEnv` line wins as a whole.
    pub set_env: Vec<EnvironmentVariable>,
    /// `IgnoreUnknown` as written.
    pub ignore_unknown: Option<String>,
    /// `Tag`.
    pub tag: Option<String>,
    /// Known but unmodeled directives, lowercased keyword to raw arguments
    /// (first occurrence wins).
    pub other: BTreeMap<String, Vec<String>>,
}

/// The outcome of [`resolve`](crate::resolve): the configuration and what
/// deserved a warning on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolution {
    /// The resolved configuration.
    pub config: ResolvedConfig,
    /// Warnings in the order they were found, without duplicates.
    pub warnings: Vec<Warning>,
}
