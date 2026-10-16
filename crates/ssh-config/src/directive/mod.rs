//! The typed model of the supported directives and their value parsers.

mod forward;
mod jump;
mod keyword;
mod values;

pub use forward::{
    DynamicForward, Endpoint, ForwardSpec, RemoteForward, parse_dynamic_forward_argument,
    parse_local_forward_argument, parse_remote_forward_argument,
};
pub use jump::JumpHost;
pub use keyword::Keyword;
pub use values::{
    AddKeysMode, AddKeysToAgent, AddressFamily, CanonicalizeHostname, EnvironmentVariable,
    PubkeyAuthentication, StrictHostKeyChecking,
};

pub(crate) use jump::{ProxyJumpValue, parse_proxy_jump};
pub(crate) use keyword::{KeywordClass, classify_keyword};
pub(crate) use values::{ForwardAgentValue, SendEnvEntry};

use crate::environment::LocalEnvironment;
use crate::error::ValueError;

const LEADING_COMMAND_CHARACTERS: [char; 5] = [' ', '\t', '\r', '\n', '='];
const NONE_VALUE: &str = "none";

/// A directive whose arguments have been validated and typed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ParsedDirective {
    Flag(Keyword, bool),
    Seconds(Keyword, Option<u32>),
    Count(Keyword, u32),
    Text(Keyword, String),
    Port(u16),
    ProxyCommand(String),
    ProxyJump(ProxyJumpValue),
    LocalForward(ForwardSpec),
    RemoteForward(RemoteForward),
    DynamicForward(DynamicForward),
    ForwardAgent(ForwardAgentValue),
    StrictHostKeyChecking(StrictHostKeyChecking),
    PubkeyAuthentication(PubkeyAuthentication),
    AddressFamily(AddressFamily),
    AddKeysToAgent(AddKeysToAgent),
    CanonicalizeHostname(CanonicalizeHostname),
    KnownHostsFiles(Keyword, Vec<String>),
    SendEnv(Vec<SendEnvEntry>),
    SetEnv(Vec<EnvironmentVariable>),
    Include(Vec<String>),
}

/// Validates and types the arguments of a modeled directive.
///
/// `raw_value` is the text after the keyword, needed by the directives that
/// take the rest of the line (`ProxyCommand`, `ProxyJump`).
pub(crate) fn parse_directive(
    keyword: Keyword,
    arguments: &[String],
    raw_value: &str,
    environment: &dyn LocalEnvironment,
) -> Result<ParsedDirective, ValueError> {
    use ParsedDirective as Parsed;
    match keyword {
        Keyword::IdentitiesOnly
        | Keyword::ForwardX11
        | Keyword::ForwardX11Trusted
        | Keyword::HashKnownHosts
        | Keyword::PasswordAuthentication
        | Keyword::KbdInteractiveAuthentication
        | Keyword::BatchMode
        | Keyword::ExitOnForwardFailure
        | Keyword::GatewayPorts
        | Keyword::ClearAllForwardings
        | Keyword::TcpKeepAlive => {
            values::parse_flag(arguments).map(|flag| Parsed::Flag(keyword, flag))
        }
        Keyword::Compression => {
            values::parse_compression(arguments).map(|flag| Parsed::Flag(keyword, flag))
        }
        Keyword::ConnectTimeout | Keyword::ServerAliveInterval => {
            values::parse_seconds(arguments).map(|seconds| Parsed::Seconds(keyword, seconds))
        }
        Keyword::ServerAliveCountMax
        | Keyword::ConnectionAttempts
        | Keyword::NumberOfPasswordPrompts => {
            values::parse_count(arguments).map(|count| Parsed::Count(keyword, count))
        }
        Keyword::Hostname
        | Keyword::User
        | Keyword::HostKeyAlias
        | Keyword::PreferredAuthentications
        | Keyword::IgnoreUnknown
        | Keyword::Tag
        | Keyword::IdentityFile
        | Keyword::CertificateFile => {
            values::parse_text(arguments).map(|text| Parsed::Text(keyword, text))
        }
        Keyword::IdentityAgent => values::parse_identity_agent(arguments, environment)
            .map(|text| Parsed::Text(keyword, text)),
        Keyword::Port => values::parse_port(arguments).map(Parsed::Port),
        Keyword::ProxyCommand => parse_proxy_command(raw_value).map(Parsed::ProxyCommand),
        Keyword::ProxyJump => {
            jump::parse_proxy_jump(strip_leading(raw_value)).map(Parsed::ProxyJump)
        }
        Keyword::LocalForward => {
            forward::parse_local_forward(arguments, environment).map(Parsed::LocalForward)
        }
        Keyword::RemoteForward => {
            forward::parse_remote_forward(arguments, environment).map(Parsed::RemoteForward)
        }
        Keyword::DynamicForward => {
            forward::parse_dynamic_forward(arguments, environment).map(Parsed::DynamicForward)
        }
        Keyword::ForwardAgent => {
            values::parse_forward_agent(arguments, environment).map(Parsed::ForwardAgent)
        }
        Keyword::StrictHostKeyChecking => {
            values::parse_strict_host_key_checking(arguments).map(Parsed::StrictHostKeyChecking)
        }
        Keyword::PubkeyAuthentication => {
            values::parse_pubkey_authentication(arguments).map(Parsed::PubkeyAuthentication)
        }
        Keyword::AddressFamily => {
            values::parse_address_family(arguments).map(Parsed::AddressFamily)
        }
        Keyword::AddKeysToAgent => {
            values::parse_add_keys_to_agent(arguments).map(Parsed::AddKeysToAgent)
        }
        Keyword::CanonicalizeHostname => {
            values::parse_canonicalize_hostname(arguments).map(Parsed::CanonicalizeHostname)
        }
        Keyword::UserKnownHostsFile | Keyword::GlobalKnownHostsFile => {
            values::parse_path_list(arguments).map(|paths| Parsed::KnownHostsFiles(keyword, paths))
        }
        Keyword::SendEnv => values::parse_send_env(arguments).map(Parsed::SendEnv),
        Keyword::SetEnv => values::parse_set_env(arguments).map(Parsed::SetEnv),
        Keyword::Include => values::parse_include_patterns(arguments).map(Parsed::Include),
    }
}

fn strip_leading(raw_value: &str) -> &str {
    raw_value.trim_start_matches(LEADING_COMMAND_CHARACTERS)
}

/// `ProxyCommand` takes the rest of the line verbatim; `none` disables it.
fn parse_proxy_command(raw_value: &str) -> Result<String, ValueError> {
    let command = strip_leading(raw_value);
    if command.is_empty() {
        return Err(ValueError::MissingArgument);
    }
    if command.eq_ignore_ascii_case(NONE_VALUE) {
        return Ok(NONE_VALUE.to_owned());
    }
    Ok(command.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::fake::FakeEnvironment;

    fn parse(keyword: Keyword, raw_value: &str) -> Result<ParsedDirective, ValueError> {
        let arguments = crate::lexer::split_arguments(raw_value).unwrap();
        parse_directive(keyword, &arguments, raw_value, &FakeEnvironment::default())
    }

    #[test]
    fn should_parse_flag_when_keyword_is_boolean() {
        assert_eq!(
            parse(Keyword::BatchMode, "yes"),
            Ok(ParsedDirective::Flag(Keyword::BatchMode, true))
        );
    }

    #[test]
    fn should_parse_text_when_keyword_is_hostname() {
        assert_eq!(
            parse(Keyword::Hostname, "example.com"),
            Ok(ParsedDirective::Text(
                Keyword::Hostname,
                "example.com".to_owned()
            ))
        );
    }

    #[test]
    fn should_reject_extra_argument_when_user_has_two_values() {
        assert_eq!(parse(Keyword::User, "a b"), Err(ValueError::ExtraArguments));
    }

    #[test]
    fn should_keep_whole_line_when_parsing_proxy_command() {
        assert_eq!(
            parse(Keyword::ProxyCommand, "ssh -W %h:%p jump # why"),
            Ok(ParsedDirective::ProxyCommand(
                "ssh -W %h:%p jump # why".to_owned()
            ))
        );
    }

    #[test]
    fn should_normalise_none_when_parsing_proxy_command() {
        assert_eq!(
            parse(Keyword::ProxyCommand, "None"),
            Ok(ParsedDirective::ProxyCommand("none".to_owned()))
        );
    }

    #[test]
    fn should_fail_when_proxy_command_is_empty() {
        assert_eq!(
            parse(Keyword::ProxyCommand, "="),
            Err(ValueError::MissingArgument)
        );
    }

    #[test]
    fn should_parse_proxy_jump_hosts_when_value_is_a_list() {
        assert!(matches!(
            parse(Keyword::ProxyJump, "a,b"),
            Ok(ParsedDirective::ProxyJump(ProxyJumpValue::Hosts(hosts))) if hosts.len() == 2
        ));
    }

    #[test]
    fn should_parse_port_when_keyword_is_port() {
        assert_eq!(
            parse(Keyword::Port, "2200"),
            Ok(ParsedDirective::Port(2200))
        );
    }

    #[test]
    fn should_parse_known_hosts_files_when_keyword_is_user_known_hosts_file() {
        assert_eq!(
            parse(Keyword::UserKnownHostsFile, "a b"),
            Ok(ParsedDirective::KnownHostsFiles(
                Keyword::UserKnownHostsFile,
                vec!["a".to_owned(), "b".to_owned()]
            ))
        );
    }
}
