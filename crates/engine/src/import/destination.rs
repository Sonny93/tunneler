//! Maps an `ssh` destination to a configured `ssh_config` alias.

use crate::external::SshDestination;
use crate::host_config::HostConfigResolver;

/// Finds the alias an `ssh` destination stands for, remembering the answers.
pub(super) struct AliasMatcher<'resolver> {
    resolver: &'resolver dyn HostConfigResolver,
    aliases: Vec<String>,
    answers: Vec<(SshDestination, Option<String>)>,
}

impl<'resolver> AliasMatcher<'resolver> {
    pub(super) fn new(resolver: &'resolver dyn HostConfigResolver) -> Self {
        Self {
            resolver,
            aliases: resolver.list_hosts().unwrap_or_default(),
            answers: Vec::new(),
        }
    }

    /// An alias equal to the host wins; else the first alias whose `HostName`
    /// equals the host (ignoring ASCII case) and whose user and port equal
    /// the destination's when it gives them.
    pub(super) fn alias_for(&mut self, destination: &SshDestination) -> Option<String> {
        if let Some((_, answer)) = self.answers.iter().find(|(known, _)| known == destination) {
            return answer.clone();
        }
        let answer = self.find_alias(destination);
        self.answers.push((destination.clone(), answer.clone()));
        answer
    }

    fn find_alias(&self, destination: &SshDestination) -> Option<String> {
        if self.aliases.contains(&destination.host) {
            return Some(destination.host.clone());
        }
        self.aliases
            .iter()
            .find(|alias| self.is_alias_of(alias, destination))
            .cloned()
    }

    fn is_alias_of(&self, alias: &str, destination: &SshDestination) -> bool {
        let Ok(config) = self.resolver.resolve(alias) else {
            return false;
        };
        config.hostname.eq_ignore_ascii_case(&destination.host)
            && destination
                .user
                .as_ref()
                .is_none_or(|user| *user == config.user)
            && destination.port.is_none_or(|port| port == config.port)
    }
}

/// The destination as a user would write it: `user@host:port`.
pub(super) fn describe(destination: &SshDestination) -> String {
    let user = destination
        .user
        .as_ref()
        .map_or(String::new(), |user| format!("{user}@"));
    let host = if destination.host.contains(':') {
        format!("[{}]", destination.host)
    } else {
        destination.host.clone()
    };
    let port = destination
        .port
        .map_or(String::new(), |port| format!(":{port}"));
    format!("{user}{host}{port}")
}
