use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::directive::{
    AddKeysMode, AddKeysToAgent, AddressFamily, DynamicForward, Endpoint, EnvironmentVariable,
    ForwardSpec, Keyword, PubkeyAuthentication, RemoteForward, StrictHostKeyChecking,
};
use crate::environment::LocalEnvironment;
use crate::error::{Error, Origin};
use crate::resolved::{AgentSocket, ForwardAgent, IdentityAgent, ResolvedConfig};
use crate::tokens::{
    FULL_TOKENS, PROXY_COMMAND_TOKENS, TokenValues, USER_TOKENS, expand_tilde, expand_tokens,
    expand_tokens_and_environment,
};

use super::DEFAULT_SSH_PORT;
use super::options::{FileEntry, Options, ProxySetting, Sourced};

const NONE_VALUE: &str = "none";
const SSH_AUTH_SOCK: &str = "SSH_AUTH_SOCK";
const ENVIRONMENT_PREFIX: char = '$';
const ENVIRONMENT_BRACE: &str = "${";
const DEFAULT_SERVER_ALIVE_COUNT_MAX: u32 = 3;
const DEFAULT_CONNECTION_ATTEMPTS: u32 = 1;
const DEFAULT_PASSWORD_PROMPTS: u32 = 3;
const DEFAULT_IDENTITY_FILES: [&str; 6] = [
    "~/.ssh/id_rsa",
    "~/.ssh/id_ecdsa",
    "~/.ssh/id_ecdsa_sk",
    "~/.ssh/id_ed25519",
    "~/.ssh/id_ed25519_sk",
    "~/.ssh/id_xmss",
];
const DEFAULT_USER_KNOWN_HOSTS_FILES: [&str; 2] = ["~/.ssh/known_hosts", "~/.ssh/known_hosts2"];
const GLOBAL_KNOWN_HOSTS_FILE_NAMES: [&str; 2] = ["ssh_known_hosts", "ssh_known_hosts2"];
const DEFAULT_ORIGIN_PATH: &str = "<default>";

/// Applies defaults to what the directives set and expands tokens, producing
/// the values `ssh -G` reports.
pub(crate) fn finalize(
    options: &Options,
    alias: &str,
    host: &str,
    environment: &dyn LocalEnvironment,
) -> Result<ResolvedConfig, Error> {
    let expander = Expander::new(options, alias, host, environment)?;
    let proxy_jump = match &options.proxy {
        Some(ProxySetting::Jump(hosts)) => hosts.clone(),
        _ => Vec::new(),
    };
    let clear_all_forwardings = flag_or(options, Keyword::ClearAllForwardings, false);
    let (local_forwards, remote_forwards, dynamic_forwards) =
        expand_forwards(options, &expander, clear_all_forwardings)?;
    Ok(ResolvedConfig {
        alias: alias.to_owned(),
        hostname: host.to_owned(),
        port: expander.port,
        user: expander.user.clone(),
        identity_files: expand_identity_files(options, &expander)?,
        certificate_files: expand_certificate_files(options, &expander)?,
        identities_only: flag_or(options, Keyword::IdentitiesOnly, false),
        identity_agent: expand_identity_agent(options, &expander)?,
        proxy_command: expand_proxy_command(options, &expander)?,
        proxy_jump,
        local_forwards,
        remote_forwards,
        dynamic_forwards,
        forward_x11: flag_or(options, Keyword::ForwardX11, false),
        forward_x11_trusted: flag_or(options, Keyword::ForwardX11Trusted, false),
        forward_agent: expand_forward_agent(options, &expander)?,
        server_alive_interval: seconds_or_zero(options, Keyword::ServerAliveInterval),
        server_alive_count_max: count_or(
            options,
            Keyword::ServerAliveCountMax,
            DEFAULT_SERVER_ALIVE_COUNT_MAX,
        ),
        strict_host_key_checking: options
            .strict_host_key_checking
            .unwrap_or(StrictHostKeyChecking::Ask),
        user_known_hosts_files: expand_user_known_hosts_files(options, &expander)?,
        global_known_hosts_files: expand_global_known_hosts_files(options, &expander),
        hash_known_hosts: flag_or(options, Keyword::HashKnownHosts, false),
        host_key_alias: expander.configured_key_alias.clone(),
        preferred_authentications: preferred_authentications(options),
        pubkey_authentication: options
            .pubkey_authentication
            .unwrap_or(PubkeyAuthentication::Yes),
        password_authentication: flag_or(options, Keyword::PasswordAuthentication, true),
        kbd_interactive_authentication: flag_or(
            options,
            Keyword::KbdInteractiveAuthentication,
            true,
        ),
        number_of_password_prompts: count_or(
            options,
            Keyword::NumberOfPasswordPrompts,
            DEFAULT_PASSWORD_PROMPTS,
        ),
        connect_timeout: options
            .seconds
            .get(&Keyword::ConnectTimeout)
            .map(|seconds| Duration::from_secs(u64::from(*seconds))),
        connection_attempts: count_or(
            options,
            Keyword::ConnectionAttempts,
            DEFAULT_CONNECTION_ATTEMPTS,
        ),
        add_keys_to_agent: options.add_keys_to_agent.unwrap_or(AddKeysToAgent {
            mode: AddKeysMode::No,
            lifetime: None,
        }),
        address_family: options.address_family.unwrap_or(AddressFamily::Any),
        batch_mode: flag_or(options, Keyword::BatchMode, false),
        exit_on_forward_failure: flag_or(options, Keyword::ExitOnForwardFailure, false),
        gateway_ports: flag_or(options, Keyword::GatewayPorts, false),
        clear_all_forwardings,
        tcp_keep_alive: flag_or(options, Keyword::TcpKeepAlive, true),
        compression: flag_or(options, Keyword::Compression, false),
        send_env: options.send_env.clone(),
        set_env: expand_set_env(options, &expander)?,
        ignore_unknown: text_value(options, Keyword::IgnoreUnknown),
        tag: text_value(options, Keyword::Tag),
        other: options.other.clone(),
    })
}

fn flag_or(options: &Options, keyword: Keyword, default: bool) -> bool {
    options.flags.get(&keyword).copied().unwrap_or(default)
}

fn count_or(options: &Options, keyword: Keyword, default: u32) -> u32 {
    options.counts.get(&keyword).copied().unwrap_or(default)
}

fn seconds_or_zero(options: &Options, keyword: Keyword) -> Duration {
    let seconds = options.seconds.get(&keyword).copied().unwrap_or(0);
    Duration::from_secs(u64::from(seconds))
}

fn text_value(options: &Options, keyword: Keyword) -> Option<String> {
    options.text(keyword).map(|text| text.value.clone())
}

fn preferred_authentications(options: &Options) -> Option<Vec<String>> {
    let methods = text_value(options, Keyword::PreferredAuthentications)?;
    Some(methods.split(',').map(str::to_owned).collect())
}

/// Expands the tokens known once the final host, port and user are settled.
struct Expander<'environment> {
    environment: &'environment dyn LocalEnvironment,
    values: TokenValues,
    home_directory: String,
    port: u16,
    user: String,
    configured_key_alias: Option<String>,
}

impl<'environment> Expander<'environment> {
    fn new(
        options: &Options,
        alias: &str,
        host: &str,
        environment: &'environment dyn LocalEnvironment,
    ) -> Result<Self, Error> {
        let port = options.port.unwrap_or(DEFAULT_SSH_PORT);
        let configured_key_alias =
            text_value(options, Keyword::HostKeyAlias).map(|alias| alias.to_ascii_lowercase());
        let values_without_user = TokenValues {
            remote_host: host.to_owned(),
            port: port.to_string(),
            key_alias: configured_key_alias
                .clone()
                .unwrap_or_else(|| alias.to_owned()),
            jump_host: options.jump_host_name(),
            original_host: alias.to_owned(),
            ..TokenValues::from_environment(environment)
        };
        let user = expand_user(options, &values_without_user, environment)?;
        Ok(Self {
            environment,
            home_directory: values_without_user.home_directory.clone(),
            values: values_without_user.with_remote_user(user.clone()),
            port,
            user,
            configured_key_alias,
        })
    }

    fn expand_path(&self, sourced: &Sourced<String>) -> Result<String, Error> {
        let with_home = expand_tilde(&sourced.value, &self.home_directory);
        self.expand_with_environment(&with_home, &sourced.origin)
    }

    fn expand_with_environment(&self, template: &str, origin: &Origin) -> Result<String, Error> {
        expand_tokens_and_environment(template, FULL_TOKENS, &self.values, self.environment)
            .map_err(|source| token_error(origin, source))
    }

    fn expand_tokens_only(&self, template: &str, origin: &Origin) -> Result<String, Error> {
        expand_tokens(template, FULL_TOKENS, &self.values)
            .map_err(|source| token_error(origin, source))
    }
}

fn token_error(origin: &Origin, source: crate::error::TokenError) -> Error {
    Error::Token {
        origin: origin.clone(),
        source,
    }
}

fn expand_user(
    options: &Options,
    values: &TokenValues,
    environment: &dyn LocalEnvironment,
) -> Result<String, Error> {
    let Some(configured) = options.text(Keyword::User) else {
        return Ok(environment.user_name());
    };
    expand_tokens_and_environment(&configured.value, USER_TOKENS, values, environment)
        .map_err(|source| token_error(&configured.origin, source))
}

fn default_entry(path: &str) -> FileEntry {
    FileEntry {
        path: Sourced {
            value: path.to_owned(),
            origin: Origin::new(Path::new(DEFAULT_ORIGIN_PATH), 0),
        },
        is_user_provided: false,
    }
}

fn expand_identity_files(
    options: &Options,
    expander: &Expander<'_>,
) -> Result<Vec<PathBuf>, Error> {
    let defaults: Vec<FileEntry>;
    let entries = if options.identity_files.is_empty() {
        defaults = DEFAULT_IDENTITY_FILES
            .iter()
            .map(|path| default_entry(path))
            .collect();
        &defaults
    } else {
        &options.identity_files
    };
    entries
        .iter()
        .filter(|entry| !entry.path.value.eq_ignore_ascii_case(NONE_VALUE))
        .map(|entry| expander.expand_path(&entry.path).map(PathBuf::from))
        .collect()
}

fn expand_certificate_files(
    options: &Options,
    expander: &Expander<'_>,
) -> Result<Vec<PathBuf>, Error> {
    options
        .certificate_files
        .iter()
        .map(|entry| expander.expand_path(&entry.path).map(PathBuf::from))
        .collect()
}

fn expand_user_known_hosts_files(
    options: &Options,
    expander: &Expander<'_>,
) -> Result<Vec<PathBuf>, Error> {
    let defaults: Vec<Sourced<String>>;
    let configured = if let Some(paths) = &options.user_known_hosts_files {
        paths
    } else {
        defaults = DEFAULT_USER_KNOWN_HOSTS_FILES
            .iter()
            .map(|path| default_entry(path).path)
            .collect();
        &defaults
    };
    if is_single_none(configured.iter().map(|path| path.value.as_str())) {
        return Ok(Vec::new());
    }
    configured
        .iter()
        .map(|path| expander.expand_path(path).map(PathBuf::from))
        .collect()
}

fn expand_global_known_hosts_files(options: &Options, expander: &Expander<'_>) -> Vec<PathBuf> {
    let configured = options.global_known_hosts_files.clone().unwrap_or_else(|| {
        let directory = expander.environment.system_config_directory();
        GLOBAL_KNOWN_HOSTS_FILE_NAMES
            .iter()
            .map(|name| directory.join(name).to_string_lossy().into_owned())
            .collect()
    });
    if is_single_none(configured.iter().map(String::as_str)) {
        return Vec::new();
    }
    configured
        .iter()
        .map(|path| PathBuf::from(expand_tilde(path, &expander.home_directory)))
        .collect()
}

fn is_single_none<'a>(mut paths: impl Iterator<Item = &'a str>) -> bool {
    let first = paths.next();
    first.is_some_and(|path| path.eq_ignore_ascii_case(NONE_VALUE)) && paths.next().is_none()
}

fn expand_identity_agent(
    options: &Options,
    expander: &Expander<'_>,
) -> Result<Option<IdentityAgent>, Error> {
    let Some(configured) = options.text(Keyword::IdentityAgent) else {
        return Ok(None);
    };
    let socket_text = expander.expand_path(configured)?;
    Ok(Some(classify_identity_agent(socket_text)))
}

fn classify_identity_agent(expanded: String) -> IdentityAgent {
    if expanded == SSH_AUTH_SOCK {
        return IdentityAgent::Socket(AgentSocket::Environment(SSH_AUTH_SOCK.to_owned()));
    }
    if expanded == NONE_VALUE {
        return IdentityAgent::Disabled;
    }
    IdentityAgent::Socket(classify_agent_socket(expanded))
}

fn classify_agent_socket(expanded: String) -> AgentSocket {
    match expanded.strip_prefix(ENVIRONMENT_PREFIX) {
        Some(name) if !expanded.starts_with(ENVIRONMENT_BRACE) => {
            AgentSocket::Environment(name.to_owned())
        }
        _ => AgentSocket::Path(PathBuf::from(expanded)),
    }
}

fn expand_forward_agent(options: &Options, expander: &Expander<'_>) -> Result<ForwardAgent, Error> {
    if options.forward_agent_enabled != Some(true) {
        return Ok(ForwardAgent::Disabled);
    }
    let Some(socket) = &options.forward_agent_socket else {
        return Ok(ForwardAgent::Enabled);
    };
    let socket_text = expander.expand_path(socket)?;
    Ok(ForwardAgent::Socket(classify_agent_socket(socket_text)))
}

fn expand_proxy_command(
    options: &Options,
    expander: &Expander<'_>,
) -> Result<Option<String>, Error> {
    let Some(ProxySetting::Command(command)) = &options.proxy else {
        return Ok(None);
    };
    expand_tokens(&command.value, PROXY_COMMAND_TOKENS, &expander.values)
        .map(Some)
        .map_err(|source| token_error(&command.origin, source))
}

fn expand_set_env(
    options: &Options,
    expander: &Expander<'_>,
) -> Result<Vec<EnvironmentVariable>, Error> {
    let Some(variables) = &options.set_env else {
        return Ok(Vec::new());
    };
    variables
        .iter()
        .map(|variable| {
            let value =
                expander.expand_with_environment(&variable.value.value, &variable.origin)?;
            Ok(EnvironmentVariable {
                name: variable.value.name.clone(),
                value,
            })
        })
        .collect()
}

type ExpandedForwards = (Vec<ForwardSpec>, Vec<RemoteForward>, Vec<DynamicForward>);

fn expand_forwards(
    options: &Options,
    expander: &Expander<'_>,
    clear_all_forwardings: bool,
) -> Result<ExpandedForwards, Error> {
    if clear_all_forwardings {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    }
    let local = options
        .local_forwards
        .iter()
        .map(|forward| expand_forward_spec(&forward.value, &forward.origin, expander))
        .collect::<Result<_, _>>()?;
    let remote = options
        .remote_forwards
        .iter()
        .map(|forward| expand_remote_forward(&forward.value, &forward.origin, expander))
        .collect::<Result<_, _>>()?;
    let dynamic = options
        .dynamic_forwards
        .iter()
        .map(|forward| expand_dynamic_forward(&forward.value, &forward.origin, expander))
        .collect::<Result<_, _>>()?;
    Ok((local, remote, dynamic))
}

fn expand_forward_spec(
    forward: &ForwardSpec,
    origin: &Origin,
    expander: &Expander<'_>,
) -> Result<ForwardSpec, Error> {
    Ok(ForwardSpec {
        listen: expand_endpoint(&forward.listen, origin, expander)?,
        target: expand_endpoint(&forward.target, origin, expander)?,
    })
}

fn expand_remote_forward(
    forward: &RemoteForward,
    origin: &Origin,
    expander: &Expander<'_>,
) -> Result<RemoteForward, Error> {
    Ok(match forward {
        RemoteForward::Fixed(spec) => {
            RemoteForward::Fixed(expand_forward_spec(spec, origin, expander)?)
        }
        RemoteForward::Dynamic(dynamic) => {
            RemoteForward::Dynamic(expand_dynamic_forward(dynamic, origin, expander)?)
        }
    })
}

fn expand_dynamic_forward(
    forward: &DynamicForward,
    origin: &Origin,
    expander: &Expander<'_>,
) -> Result<DynamicForward, Error> {
    Ok(DynamicForward {
        listen: expand_endpoint(&forward.listen, origin, expander)?,
    })
}

fn expand_endpoint(
    endpoint: &Endpoint,
    origin: &Origin,
    expander: &Expander<'_>,
) -> Result<Endpoint, Error> {
    match endpoint {
        Endpoint::Tcp { .. } => Ok(endpoint.clone()),
        Endpoint::Unix(path) => {
            let socket_path = expander.expand_tokens_only(&path.to_string_lossy(), origin)?;
            Ok(Endpoint::Unix(PathBuf::from(socket_path)))
        }
    }
}
