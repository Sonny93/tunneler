//! Rendering of a [`ResolvedConfig`] in `ssh -G` spelling.

use std::path::PathBuf;

use crate::directive::{
    AddKeysMode, AddressFamily, Endpoint, ForwardSpec, JumpHost, PubkeyAuthentication,
    RemoteForward, StrictHostKeyChecking,
};
use crate::resolved::{AgentSocket, ForwardAgent, IdentityAgent, ResolvedConfig};

const OPENSSH_DEFAULT_SERVER_ALIVE_COUNT_MAX: u32 = 3;
const SSH_AUTH_SOCK: &str = "SSH_AUTH_SOCK";
const REMOTE_SOCKS_TARGET: &str = "[socks]:0";
const NONE: &str = "none";

type Line = (&'static str, String);

impl ResolvedConfig {
    /// The options in `ssh -G` spelling and order: lowercase keyword and value
    /// as `ssh -G` prints them, multi-valued keywords repeated once per value.
    ///
    /// Unmodeled directives ([`other`](Self::other)) come last, as the
    /// lowercased keyword followed by its arguments joined by a space.
    #[must_use]
    pub fn ssh_g_options(&self) -> Vec<(String, String)> {
        let modeled = render_scalars(self)
            .into_iter()
            .chain(render_optionals(self))
            .chain(render_lists(self))
            .map(|(key, value)| (key.to_owned(), value));
        let unmodeled = self
            .other
            .iter()
            .map(|(keyword, arguments)| (keyword.clone(), arguments.join(" ")));
        modeled.chain(unmodeled).collect()
    }
}

impl Endpoint {
    /// The endpoint in `ssh -G` spelling: `port`, `[address]:port` or the socket path.
    #[must_use]
    pub fn ssh_g_form(&self) -> String {
        format_endpoint(self)
    }
}

impl ForwardSpec {
    /// The forward in `ssh -G` spelling: `<listen> <target>`.
    #[must_use]
    pub fn ssh_g_form(&self) -> String {
        format_forward(self)
    }
}

impl RemoteForward {
    /// The forward in `ssh -G` spelling; a remote SOCKS proxy targets `[socks]:0`.
    #[must_use]
    pub fn ssh_g_form(&self) -> String {
        format_remote_forward(self)
    }
}

pub(crate) fn format_jump_chain(hosts: &[JumpHost]) -> String {
    hosts
        .iter()
        .map(format_jump_host)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_jump_host(jump: &JumpHost) -> String {
    let user = jump
        .user
        .as_ref()
        .map_or(String::new(), |user| format!("{user}@"));
    let host = if jump.host.contains(':') {
        format!("[{}]", jump.host)
    } else {
        jump.host.clone()
    };
    let port = jump.port.map_or(String::new(), |port| format!(":{port}"));
    format!("{user}{host}{port}")
}

pub(crate) fn format_endpoint(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::Tcp {
            address: None,
            port,
        } => port.to_string(),
        Endpoint::Tcp {
            address: Some(address),
            port,
        } => format!("[{address}]:{port}"),
        Endpoint::Unix(path) => path.to_string_lossy().into_owned(),
    }
}

pub(crate) fn format_forward(forward: &ForwardSpec) -> String {
    let listen = format_endpoint(&forward.listen);
    let target = match &forward.target {
        Endpoint::Tcp {
            address: None,
            port,
        } => port.to_string(),
        other => format_endpoint(other),
    };
    format!("{listen} {target}")
}

pub(crate) fn format_remote_forward(forward: &RemoteForward) -> String {
    match forward {
        RemoteForward::Fixed(spec) => format_forward(spec),
        RemoteForward::Dynamic(dynamic) => {
            format!("{} {REMOTE_SOCKS_TARGET}", format_endpoint(&dynamic.listen))
        }
    }
}

fn yes_no(value: bool) -> String {
    if value { "yes" } else { "no" }.to_owned()
}

fn space_separated_or_none(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return NONE.to_owned();
    }
    paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_agent_socket(socket: &AgentSocket) -> String {
    match socket {
        AgentSocket::Environment(name) if name == SSH_AUTH_SOCK => SSH_AUTH_SOCK.to_owned(),
        AgentSocket::Environment(name) => format!("${name}"),
        AgentSocket::Path(path) => path.to_string_lossy().into_owned(),
    }
}

fn render_add_keys_to_agent(config: &ResolvedConfig) -> String {
    let setting = config.add_keys_to_agent;
    let mode = match setting.mode {
        AddKeysMode::No => "no",
        AddKeysMode::Yes => "yes",
        AddKeysMode::Ask => "ask",
        AddKeysMode::Confirm => "confirm",
    };
    match (setting.lifetime, setting.mode) {
        (Some(lifetime), AddKeysMode::Confirm) => format!("confirm {}", lifetime.as_secs()),
        (Some(lifetime), _) => lifetime.as_secs().to_string(),
        (None, _) => mode.to_owned(),
    }
}

fn render_forward_agent(config: &ResolvedConfig) -> String {
    match &config.forward_agent {
        ForwardAgent::Disabled => "no".to_owned(),
        ForwardAgent::Enabled => "yes".to_owned(),
        ForwardAgent::Socket(socket) => render_agent_socket(socket),
    }
}

fn render_strict_host_key_checking(config: &ResolvedConfig) -> &'static str {
    match config.strict_host_key_checking {
        StrictHostKeyChecking::Yes => "yes",
        StrictHostKeyChecking::No => "no",
        StrictHostKeyChecking::Ask => "ask",
        StrictHostKeyChecking::AcceptNew => "accept-new",
    }
}

fn render_pubkey_authentication(config: &ResolvedConfig) -> &'static str {
    match config.pubkey_authentication {
        PubkeyAuthentication::Yes => "yes",
        PubkeyAuthentication::No => "no",
        PubkeyAuthentication::Unbound => "unbound",
        PubkeyAuthentication::HostBound => "host-bound",
    }
}

fn render_address_family(config: &ResolvedConfig) -> &'static str {
    match config.address_family {
        AddressFamily::Any => "any",
        AddressFamily::Inet => "inet",
        AddressFamily::Inet6 => "inet6",
    }
}

fn render_scalars(config: &ResolvedConfig) -> Vec<Line> {
    let connect_timeout = config
        .connect_timeout
        .map_or_else(|| NONE.to_owned(), |timeout| timeout.as_secs().to_string());
    vec![
        ("user", config.user.clone()),
        ("hostname", config.hostname.clone()),
        ("port", config.port.to_string()),
        ("addressfamily", render_address_family(config).to_owned()),
        ("batchmode", yes_no(config.batch_mode)),
        ("clearallforwardings", yes_no(config.clear_all_forwardings)),
        ("compression", yes_no(config.compression)),
        (
            "exitonforwardfailure",
            yes_no(config.exit_on_forward_failure),
        ),
        ("forwardx11", yes_no(config.forward_x11)),
        ("forwardx11trusted", yes_no(config.forward_x11_trusted)),
        ("gatewayports", yes_no(config.gateway_ports)),
        ("hashknownhosts", yes_no(config.hash_known_hosts)),
        ("identitiesonly", yes_no(config.identities_only)),
        (
            "kbdinteractiveauthentication",
            yes_no(config.kbd_interactive_authentication),
        ),
        (
            "passwordauthentication",
            yes_no(config.password_authentication),
        ),
        (
            "pubkeyauthentication",
            render_pubkey_authentication(config).to_owned(),
        ),
        (
            "stricthostkeychecking",
            render_strict_host_key_checking(config).to_owned(),
        ),
        ("tcpkeepalive", yes_no(config.tcp_keep_alive)),
        ("connectionattempts", config.connection_attempts.to_string()),
        (
            "numberofpasswordprompts",
            config.number_of_password_prompts.to_string(),
        ),
        (
            "serveralivecountmax",
            config
                .server_alive_count_max
                .unwrap_or(OPENSSH_DEFAULT_SERVER_ALIVE_COUNT_MAX)
                .to_string(),
        ),
        (
            "serveraliveinterval",
            config
                .server_alive_interval
                .map_or(0, |interval| interval.as_secs())
                .to_string(),
        ),
        ("addkeystoagent", render_add_keys_to_agent(config)),
        ("forwardagent", render_forward_agent(config)),
        ("connecttimeout", connect_timeout),
    ]
}

fn render_identity_agent(agent: &IdentityAgent) -> String {
    match agent {
        IdentityAgent::Disabled => NONE.to_owned(),
        IdentityAgent::Socket(socket) => render_agent_socket(socket),
    }
}

fn render_optionals(config: &ResolvedConfig) -> Vec<Line> {
    let mut lines = Vec::new();
    if let Some(alias) = &config.host_key_alias {
        lines.push(("hostkeyalias", alias.clone()));
    }
    if let Some(agent) = &config.identity_agent {
        lines.push(("identityagent", render_identity_agent(agent)));
    }
    if let Some(unknown) = &config.ignore_unknown {
        lines.push(("ignoreunknown", unknown.clone()));
    }
    if let Some(methods) = &config.preferred_authentications {
        lines.push(("preferredauthentications", methods.join(",")));
    }
    if let Some(tag) = &config.tag {
        lines.push(("tag", tag.clone()));
    }
    if let Some(command) = &config.proxy_command {
        lines.push(("proxycommand", command.clone()));
    }
    if !config.proxy_jump.is_empty() {
        lines.push(("proxyjump", format_jump_chain(&config.proxy_jump)));
    }
    lines
}

fn render_forwards(config: &ResolvedConfig) -> Vec<Line> {
    let dynamic = config
        .dynamic_forwards
        .iter()
        .map(|forward| ("dynamicforward", format_endpoint(&forward.listen)));
    let local = config
        .local_forwards
        .iter()
        .map(|forward| ("localforward", format_forward(forward)));
    let remote = config
        .remote_forwards
        .iter()
        .map(|forward| ("remoteforward", format_remote_forward(forward)));
    dynamic.chain(local).chain(remote).collect()
}

fn render_lists(config: &ResolvedConfig) -> Vec<Line> {
    let mut lines = render_forwards(config);
    for path in &config.identity_files {
        lines.push(("identityfile", path.to_string_lossy().into_owned()));
    }
    for path in &config.certificate_files {
        lines.push(("certificatefile", path.to_string_lossy().into_owned()));
    }
    lines.push((
        "globalknownhostsfile",
        space_separated_or_none(&config.global_known_hosts_files),
    ));
    lines.push((
        "userknownhostsfile",
        space_separated_or_none(&config.user_known_hosts_files),
    ));
    for variable in &config.send_env {
        lines.push(("sendenv", variable.clone()));
    }
    for variable in &config.set_env {
        lines.push(("setenv", format!("{}={}", variable.name, variable.value)));
    }
    lines
}

#[cfg(test)]
mod tests {
    use crate::environment::fake::FakeEnvironment;
    use crate::resolve;
    use crate::sources::ConfigSources;

    fn rendered(config: &str, alias: &str) -> Vec<String> {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config");
        std::fs::write(&path, config).unwrap();
        let environment = FakeEnvironment {
            home_directory: directory.path().join("home"),
            system_directory: directory.path().join("etc"),
            ..FakeEnvironment::default()
        };
        let resolution = resolve(&ConfigSources::user_only(path), alias, &environment).unwrap();
        resolution
            .config
            .ssh_g_options()
            .into_iter()
            .map(|(key, value)| format!("{key} {value}"))
            .collect()
    }

    fn contains(lines: &[String], expected: &str) -> bool {
        lines.iter().any(|line| line == expected)
    }

    #[test]
    fn should_render_forwards_in_ssh_g_spelling_when_host_has_forwards() {
        let lines = rendered(
            "Host a\n  LocalForward 3333 localhost:3333\n  RemoteForward 9000 db:5432\n  DynamicForward 1080\n  RemoteForward 7000\n",
            "a",
        );
        assert!(contains(&lines, "localforward 3333 [localhost]:3333"));
        assert!(contains(&lines, "remoteforward 9000 [db]:5432"));
        assert!(contains(&lines, "dynamicforward 1080"));
        assert!(contains(&lines, "remoteforward 7000 [socks]:0"));
    }

    #[test]
    fn should_render_jump_chain_when_host_has_proxy_jump() {
        let lines = rendered("Host a\n  ProxyJump bob@gate:2222,other\n", "a");
        assert!(contains(&lines, "proxyjump bob@gate:2222,other"));
    }

    #[test]
    fn should_render_none_when_known_hosts_lists_are_empty() {
        let lines = rendered(
            "Host a\n  UserKnownHostsFile none\n  GlobalKnownHostsFile none\n",
            "a",
        );
        assert!(contains(&lines, "userknownhostsfile none"));
        assert!(contains(&lines, "globalknownhostsfile none"));
    }

    #[test]
    fn should_append_unmodeled_directives_when_config_has_other_keywords() {
        let lines = rendered("Host a\n  LogLevel DEBUG\n", "a");
        assert_eq!(lines.last().map(String::as_str), Some("loglevel DEBUG"));
    }
}
