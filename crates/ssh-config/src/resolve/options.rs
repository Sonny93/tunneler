use std::collections::{BTreeMap, HashMap};

use crate::directive::{
    AddKeysToAgent, AddressFamily, DynamicForward, EnvironmentVariable, ForwardAgentValue,
    ForwardSpec, JumpHost, Keyword, ParsedDirective, ProxyJumpValue, PubkeyAuthentication,
    RemoteForward, SendEnvEntry, StrictHostKeyChecking,
};
use crate::error::Origin;
use crate::include::ConfigScope;
use crate::pattern::matches_wildcard;

const NONE_VALUE: &str = "none";

/// A value together with the line that set it, for error reporting.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sourced<T> {
    pub value: T,
    pub origin: Origin,
}

/// The first of `ProxyCommand` and `ProxyJump` wins, except that `ProxyJump none`
/// leaves room for a later `ProxyCommand` (observed with `ssh -G`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProxySetting {
    /// `ProxyCommand none`.
    Disabled,
    /// `ProxyJump none`: blocks later `ProxyJump` lines but not `ProxyCommand`.
    JumpDisabled,
    Command(Sourced<String>),
    Jump(Vec<JumpHost>),
}

/// A file entry that remembers whether the user configuration provided it,
/// which is part of OpenSSH's duplicate detection.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FileEntry {
    pub path: Sourced<String>,
    pub is_user_provided: bool,
}

/// What the directives seen so far have set. Everything starts unset and the
/// first value wins, except for the cumulative directives.
#[derive(Debug, Default)]
pub(crate) struct Options {
    pub flags: HashMap<Keyword, bool>,
    pub counts: HashMap<Keyword, u32>,
    pub seconds: HashMap<Keyword, u32>,
    pub texts: HashMap<Keyword, Sourced<String>>,
    pub port: Option<u16>,
    pub identity_files: Vec<FileEntry>,
    pub certificate_files: Vec<FileEntry>,
    pub proxy: Option<ProxySetting>,
    pub local_forwards: Vec<Sourced<ForwardSpec>>,
    pub remote_forwards: Vec<Sourced<RemoteForward>>,
    pub dynamic_forwards: Vec<Sourced<DynamicForward>>,
    pub forward_agent_enabled: Option<bool>,
    pub forward_agent_socket: Option<Sourced<String>>,
    pub strict_host_key_checking: Option<StrictHostKeyChecking>,
    pub pubkey_authentication: Option<PubkeyAuthentication>,
    pub address_family: Option<AddressFamily>,
    pub add_keys_to_agent: Option<AddKeysToAgent>,
    pub user_known_hosts_files: Option<Vec<Sourced<String>>>,
    pub global_known_hosts_files: Option<Vec<String>>,
    pub send_env: Vec<String>,
    pub set_env: Option<Vec<Sourced<EnvironmentVariable>>>,
    pub other: BTreeMap<String, Vec<String>>,
}

impl Options {
    pub(crate) fn text(&self, keyword: Keyword) -> Option<&Sourced<String>> {
        self.texts.get(&keyword)
    }

    /// The host of the last jump, which `%j` stands for.
    pub(crate) fn jump_host_name(&self) -> String {
        match &self.proxy {
            Some(ProxySetting::Jump(hosts)) => hosts
                .last()
                .map(|jump| jump.host.clone())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// Records the effect of an active directive.
    pub(crate) fn apply(
        &mut self,
        directive: ParsedDirective,
        origin: &Origin,
        scope: ConfigScope,
    ) {
        match directive {
            ParsedDirective::Flag(keyword, value) => {
                self.flags.entry(keyword).or_insert(value);
            }
            ParsedDirective::Seconds(keyword, Some(value)) => {
                self.seconds.entry(keyword).or_insert(value);
            }
            ParsedDirective::Count(keyword, value) => {
                self.counts.entry(keyword).or_insert(value);
            }
            ParsedDirective::Text(keyword, value) => self.apply_text(keyword, value, origin, scope),
            ParsedDirective::Port(port) => {
                self.port.get_or_insert(port);
            }
            ParsedDirective::ProxyCommand(command) => self.apply_proxy_command(&command, origin),
            ParsedDirective::ProxyJump(value) => self.apply_proxy_jump(value),
            ParsedDirective::LocalForward(forward) => {
                push_unique(&mut self.local_forwards, forward, origin);
            }
            ParsedDirective::RemoteForward(forward) => {
                push_unique(&mut self.remote_forwards, forward, origin);
            }
            ParsedDirective::DynamicForward(forward) => {
                push_unique(&mut self.dynamic_forwards, forward, origin);
            }
            ParsedDirective::ForwardAgent(value) => self.apply_forward_agent(value, origin),
            ParsedDirective::StrictHostKeyChecking(value) => {
                self.strict_host_key_checking.get_or_insert(value);
            }
            ParsedDirective::PubkeyAuthentication(value) => {
                self.pubkey_authentication.get_or_insert(value);
            }
            ParsedDirective::AddressFamily(value) => {
                self.address_family.get_or_insert(value);
            }
            ParsedDirective::AddKeysToAgent(value) => {
                self.add_keys_to_agent.get_or_insert(value);
            }
            ParsedDirective::KnownHostsFiles(keyword, paths) => {
                self.apply_known_hosts_files(keyword, paths, origin);
            }
            ParsedDirective::SendEnv(entries) => self.apply_send_env(entries),
            ParsedDirective::SetEnv(variables) => self.apply_set_env(variables, origin),
            ParsedDirective::Seconds(_, None)
            | ParsedDirective::CanonicalizeHostname(_)
            | ParsedDirective::Include(_) => {}
        }
    }

    fn apply_text(&mut self, keyword: Keyword, value: String, origin: &Origin, scope: ConfigScope) {
        let sourced = Sourced {
            value,
            origin: origin.clone(),
        };
        match keyword {
            Keyword::IdentityFile => push_file_entry(&mut self.identity_files, sourced, scope),
            Keyword::CertificateFile => {
                push_file_entry(&mut self.certificate_files, sourced, scope);
            }
            _ => {
                self.texts.entry(keyword).or_insert(sourced);
            }
        }
    }

    fn apply_proxy_command(&mut self, command: &str, origin: &Origin) {
        if self.proxy.is_some() && self.proxy != Some(ProxySetting::JumpDisabled) {
            return;
        }
        self.proxy = Some(if command.eq_ignore_ascii_case(NONE_VALUE) {
            ProxySetting::Disabled
        } else {
            ProxySetting::Command(Sourced {
                value: command.to_owned(),
                origin: origin.clone(),
            })
        });
    }

    fn apply_proxy_jump(&mut self, value: ProxyJumpValue) {
        if self.proxy.is_some() {
            return;
        }
        self.proxy = Some(match value {
            ProxyJumpValue::None => ProxySetting::JumpDisabled,
            ProxyJumpValue::Hosts(hosts) => ProxySetting::Jump(hosts),
        });
    }

    fn apply_forward_agent(&mut self, value: ForwardAgentValue, origin: &Origin) {
        match value {
            ForwardAgentValue::Flag(flag) => {
                self.forward_agent_enabled.get_or_insert(flag);
            }
            ForwardAgentValue::Socket(path) => {
                self.forward_agent_enabled.get_or_insert(true);
                self.forward_agent_socket.get_or_insert(Sourced {
                    value: path,
                    origin: origin.clone(),
                });
            }
        }
    }

    fn apply_known_hosts_files(&mut self, keyword: Keyword, paths: Vec<String>, origin: &Origin) {
        if paths.is_empty() {
            return;
        }
        if keyword == Keyword::GlobalKnownHostsFile {
            self.global_known_hosts_files.get_or_insert(paths);
            return;
        }
        self.user_known_hosts_files.get_or_insert_with(|| {
            paths
                .into_iter()
                .map(|value| Sourced {
                    value,
                    origin: origin.clone(),
                })
                .collect()
        });
    }

    fn apply_send_env(&mut self, entries: Vec<SendEnvEntry>) {
        for entry in entries {
            match entry {
                SendEnvEntry::Add(pattern) => self.send_env.push(pattern),
                SendEnvEntry::Remove(pattern) => {
                    self.send_env
                        .retain(|variable| !matches_wildcard(variable, &pattern));
                }
            }
        }
    }

    fn apply_set_env(&mut self, variables: Vec<EnvironmentVariable>, origin: &Origin) {
        self.set_env.get_or_insert_with(|| {
            variables
                .into_iter()
                .map(|value| Sourced {
                    value,
                    origin: origin.clone(),
                })
                .collect()
        });
    }
}

fn push_unique<T: PartialEq>(list: &mut Vec<Sourced<T>>, value: T, origin: &Origin) {
    if list.iter().any(|existing| existing.value == value) {
        return;
    }
    list.push(Sourced {
        value,
        origin: origin.clone(),
    });
}

fn push_file_entry(entries: &mut Vec<FileEntry>, path: Sourced<String>, scope: ConfigScope) {
    let is_user_provided = scope == ConfigScope::User;
    let is_duplicate = entries
        .iter()
        .any(|entry| entry.is_user_provided == is_user_provided && entry.path.value == path.value);
    if is_duplicate {
        return;
    }
    entries.push(FileEntry {
        path,
        is_user_provided,
    });
}
