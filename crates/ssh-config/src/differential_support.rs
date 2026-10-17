//! Hooks for the differential tests against `ssh -G`. Not part of the stable API.

use crate::directive::{JumpHost, ProxyJumpValue, parse_proxy_jump};
use crate::environment::LocalEnvironment;
use crate::error::TokenError;
use crate::tokens::{
    FULL_TOKENS, PROXY_COMMAND_TOKENS, TokenValues, expand_tilde, expand_tokens,
    expand_tokens_and_environment,
};

/// The connection facts `ssh -G` reports, from which its unexpanded values can
/// be expanded the way a connection would.
#[derive(Debug, Clone)]
pub struct ExpansionContext {
    pub original_host: String,
    pub hostname: String,
    pub port: u16,
    pub user: String,
    pub host_key_alias: Option<String>,
    pub jump_host: String,
}

impl ExpansionContext {
    fn values(&self, environment: &impl LocalEnvironment) -> TokenValues {
        TokenValues {
            remote_host: self.hostname.clone(),
            port: self.port.to_string(),
            key_alias: self
                .host_key_alias
                .clone()
                .unwrap_or_else(|| self.original_host.clone()),
            jump_host: self.jump_host.clone(),
            original_host: self.original_host.clone(),
            ..TokenValues::from_environment(environment)
        }
        .with_remote_user(self.user.clone())
    }

    /// Expands `~`, tokens and `${VAR}` like `IdentityFile` does.
    pub fn expand_path(
        &self,
        template: &str,
        environment: &impl LocalEnvironment,
    ) -> Result<String, TokenError> {
        let home_directory = environment.home_directory().to_string_lossy().into_owned();
        let with_home = expand_tilde(template, &home_directory);
        expand_tokens_and_environment(
            &with_home,
            FULL_TOKENS,
            &self.values(environment),
            environment,
        )
    }

    /// Expands the tokens of a `ProxyCommand`.
    pub fn expand_proxy_command(
        &self,
        template: &str,
        environment: &impl LocalEnvironment,
    ) -> Result<String, TokenError> {
        expand_tokens(template, PROXY_COMMAND_TOKENS, &self.values(environment))
    }
}

/// Parses a `ProxyJump` value, `None` when it is invalid; `none` gives no hosts.
pub fn parse_jump_chain(text: &str) -> Option<Vec<JumpHost>> {
    match parse_proxy_jump(text).ok()? {
        ProxyJumpValue::None => Some(Vec::new()),
        ProxyJumpValue::Hosts(hosts) => Some(hosts),
    }
}

/// Renders a jump chain the way `ssh -G` prints `ProxyJump`.
pub fn format_jump_chain(hosts: &[JumpHost]) -> String {
    crate::render::format_jump_chain(hosts)
}
