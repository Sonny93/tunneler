//! Reads the forwards and the destination of an `ssh` command line.
//!
//! Options without argument (`46AaCfGgKkMNnqsTtVvXxYy`) and unknown ones are
//! skipped.
//!
//! Known limit: `-o` options (for example `-o LocalForward=...`) are not
//! interpreted.

use ssh_config::{
    DynamicForward, Endpoint, ForwardSpec, RemoteForward, parse_dynamic_forward_argument,
    parse_local_forward_argument, parse_remote_forward_argument,
};

use crate::store::TunnelKind;

const OPTIONS_WITH_ARGUMENT: &str = "BbcDEeFIiJLlmOoPpQRSWw";
const OPTION_PREFIX: char = '-';
const OPTIONS_END: &str = "--";
const LOCAL_FORWARD_OPTION: char = 'L';
const REMOTE_FORWARD_OPTION: char = 'R';
const DYNAMIC_FORWARD_OPTION: char = 'D';
const URL_SCHEME: &str = "ssh://";
const USER_SEPARATOR: char = '@';
const PORT_SEPARATOR: char = ':';
const IPV6_BRACKETS: [char; 2] = ['[', ']'];

/// What an `ssh` command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshInvocation {
    /// The host to connect to, without user, port or brackets.
    pub destination: Option<String>,
    /// The TCP forwards requested with `-L`, `-R` and `-D`.
    pub forwards: Vec<SshForward>,
}

/// One TCP forward requested on an `ssh` command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshForward {
    /// The direction of the forward.
    pub kind: TunnelKind,
    /// The bind address; `None` when none was written.
    pub listen_address: Option<String>,
    /// The port to listen on.
    pub listen_port: u16,
    /// The host the traffic is forwarded to; `None` for a dynamic forward.
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to; `None` for a dynamic forward.
    pub target_port: Option<u16>,
}

struct OptionOccurrence {
    letter: char,
    value: String,
}

/// Parses an `ssh` command line the way OpenSSH reads its options.
///
/// `arguments[0]` is the program. Unix socket forwards, remote SOCKS proxies
/// and malformed forwards are skipped: a malformed command line yields fewer
/// forwards, never an error.
pub fn parse_ssh_command_line(arguments: &[String]) -> SshInvocation {
    let program_arguments = arguments.get(1..).unwrap_or_default();
    let (occurrences, remaining) = read_options(program_arguments);
    SshInvocation {
        destination: remaining.first().and_then(|text| parse_destination(text)),
        forwards: occurrences.iter().filter_map(forward_of).collect(),
    }
}

fn read_options(arguments: &[String]) -> (Vec<OptionOccurrence>, &[String]) {
    let mut occurrences = Vec::new();
    let mut remaining = arguments;
    while let Some((argument, following)) = remaining.split_first() {
        if argument == OPTIONS_END {
            remaining = following;
            break;
        }
        let Some(bundle) = option_bundle(argument) else {
            break;
        };
        let (bundle_occurrences, after_bundle) = read_bundle(bundle, following);
        occurrences.extend(bundle_occurrences);
        remaining = after_bundle;
    }
    (occurrences, remaining)
}

fn option_bundle(argument: &str) -> Option<&str> {
    argument
        .strip_prefix(OPTION_PREFIX)
        .filter(|bundle| !bundle.is_empty())
}

fn read_bundle<'a>(bundle: &str, following: &'a [String]) -> (Vec<OptionOccurrence>, &'a [String]) {
    let Some((offset, letter)) = bundle
        .char_indices()
        .find(|(_, letter)| OPTIONS_WITH_ARGUMENT.contains(*letter))
    else {
        return (Vec::new(), following);
    };
    let glued_value = &bundle[offset + letter.len_utf8()..];
    if !glued_value.is_empty() {
        return (vec![occurrence(letter, glued_value)], following);
    }
    match following.split_first() {
        Some((value, after_value)) => (vec![occurrence(letter, value)], after_value),
        None => (Vec::new(), following),
    }
}

fn occurrence(letter: char, value: &str) -> OptionOccurrence {
    OptionOccurrence {
        letter,
        value: value.to_owned(),
    }
}

fn forward_of(occurrence: &OptionOccurrence) -> Option<SshForward> {
    match occurrence.letter {
        LOCAL_FORWARD_OPTION => local_forward(&occurrence.value),
        REMOTE_FORWARD_OPTION => remote_forward(&occurrence.value),
        DYNAMIC_FORWARD_OPTION => dynamic_forward(&occurrence.value),
        _ => None,
    }
}

fn local_forward(argument: &str) -> Option<SshForward> {
    let specification = parse_local_forward_argument(argument).ok()?;
    fixed_forward(TunnelKind::Local, specification)
}

fn remote_forward(argument: &str) -> Option<SshForward> {
    match parse_remote_forward_argument(argument).ok()? {
        RemoteForward::Fixed(specification) => fixed_forward(TunnelKind::Remote, specification),
        RemoteForward::Dynamic(_) => None,
    }
}

fn dynamic_forward(argument: &str) -> Option<SshForward> {
    let DynamicForward { listen } = parse_dynamic_forward_argument(argument).ok()?;
    let Endpoint::Tcp { address, port } = listen else {
        return None;
    };
    Some(SshForward {
        kind: TunnelKind::Dynamic,
        listen_address: address,
        listen_port: port,
        target_host: None,
        target_port: None,
    })
}

fn fixed_forward(kind: TunnelKind, specification: ForwardSpec) -> Option<SshForward> {
    let (listen_address, listen_port) = tcp_parts(specification.listen)?;
    let (target_host, target_port) = tcp_parts(specification.target)?;
    if listen_port == 0 {
        return None;
    }
    Some(SshForward {
        kind,
        listen_address,
        listen_port,
        target_host,
        target_port: Some(target_port),
    })
}

fn tcp_parts(endpoint: Endpoint) -> Option<(Option<String>, u16)> {
    match endpoint {
        Endpoint::Tcp { address, port } => Some((address, port)),
        Endpoint::Unix(_) => None,
    }
}

fn parse_destination(text: &str) -> Option<String> {
    let host = match text.strip_prefix(URL_SCHEME) {
        Some(url) => host_of_url(url),
        None => without_user(text).trim_matches(IPV6_BRACKETS).to_owned(),
    };
    (!host.is_empty()).then_some(host)
}

fn host_of_url(url: &str) -> String {
    let authority = url.split('/').next().unwrap_or_default();
    let host_and_port = without_user(authority);
    if let Some(bracketed) = host_and_port.strip_prefix(IPV6_BRACKETS[0]) {
        return bracketed
            .split(IPV6_BRACKETS[1])
            .next()
            .unwrap_or_default()
            .to_owned();
    }
    host_and_port
        .split(PORT_SEPARATOR)
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn without_user(text: &str) -> &str {
    text.rsplit_once(USER_SEPARATOR)
        .map_or(text, |(_, host)| host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn invocation(command_line: &str) -> SshInvocation {
        let arguments: Vec<String> = command_line.split_whitespace().map(str::to_owned).collect();
        parse_ssh_command_line(&arguments)
    }

    fn local(listen_port: u16, target_host: &str, target_port: u16) -> SshForward {
        SshForward {
            kind: TunnelKind::Local,
            listen_address: None,
            listen_port,
            target_host: Some(target_host.to_owned()),
            target_port: Some(target_port),
        }
    }

    #[test]
    fn should_read_separate_local_forward_when_argument_follows_the_option() {
        assert_eq!(
            invocation("ssh -L 3333:localhost:3333 nova").forwards,
            vec![local(3333, "localhost", 3333)]
        );
    }

    #[test]
    fn should_read_glued_local_forward_when_argument_is_attached_to_the_option() {
        assert_eq!(
            invocation("ssh -L3333:db:5432 nova").forwards,
            vec![local(3333, "db", 5432)]
        );
    }

    #[test]
    fn should_read_bundled_local_forward_when_flags_precede_the_option() {
        assert_eq!(
            invocation("ssh -NfL3333:localhost:3333 nova").forwards,
            vec![local(3333, "localhost", 3333)]
        );
    }

    #[test]
    fn should_read_bind_address_when_local_forward_has_one() {
        let forwards = invocation("ssh -L 0.0.0.0:80:web:8080 nova").forwards;

        assert_eq!(forwards[0].listen_address.as_deref(), Some("0.0.0.0"));
    }

    #[test]
    fn should_read_bracketed_ipv6_when_local_forward_uses_it() {
        let forwards = invocation("ssh -L [::1]:80:web:8080 nova").forwards;

        assert_eq!(forwards[0].listen_address.as_deref(), Some("::1"));
    }

    #[test]
    fn should_read_dynamic_forward_without_target_when_option_is_d() {
        assert_eq!(
            invocation("ssh -D 1080 nova").forwards,
            vec![SshForward {
                kind: TunnelKind::Dynamic,
                listen_address: None,
                listen_port: 1080,
                target_host: None,
                target_port: None,
            }]
        );
    }

    #[test]
    fn should_read_remote_forward_when_option_is_r_with_a_target() {
        assert_eq!(
            invocation("ssh -R 9000:localhost:80 nova").forwards,
            vec![SshForward {
                kind: TunnelKind::Remote,
                listen_address: None,
                listen_port: 9000,
                target_host: Some("localhost".to_owned()),
                target_port: Some(80),
            }]
        );
    }

    #[test]
    fn should_read_every_forward_when_several_options_are_given() {
        assert_eq!(
            invocation("ssh -L 1:a:1 -D 2 -L 3:b:3 nova").forwards.len(),
            3
        );
    }

    #[test]
    fn should_skip_remote_dynamic_forward_when_r_has_only_a_port() {
        assert_eq!(invocation("ssh -R 9000 nova").forwards, Vec::new());
    }

    #[test]
    fn should_skip_remote_forward_when_listen_port_is_zero() {
        assert_eq!(
            invocation("ssh -R 0:localhost:80 nova").forwards,
            Vec::new()
        );
    }

    #[test]
    fn should_skip_unix_socket_forward_when_a_side_is_a_path() {
        assert_eq!(
            invocation("ssh -L 8080:/run/app.sock nova").forwards,
            Vec::new()
        );
    }

    #[test]
    fn should_skip_forward_when_specification_is_malformed() {
        assert_eq!(invocation("ssh -L nonsense nova").forwards, Vec::new());
    }

    #[test]
    fn should_not_read_forwards_when_they_follow_the_destination() {
        assert_eq!(
            invocation("ssh nova -L 3333:localhost:3333").forwards,
            Vec::new()
        );
    }

    #[test]
    fn should_not_read_forwards_when_they_follow_the_options_end_marker() {
        assert_eq!(
            invocation("ssh -- nova -L 3333:localhost:3333").forwards,
            Vec::new()
        );
    }

    #[test]
    fn should_read_destination_after_the_options_end_marker() {
        assert_eq!(
            invocation("ssh -- nova").destination,
            Some("nova".to_owned())
        );
    }

    #[test]
    fn should_not_mistake_option_arguments_for_the_destination() {
        assert_eq!(
            invocation("ssh -p 22 -i key -N nova").destination,
            Some("nova".to_owned())
        );
    }

    #[test]
    fn should_skip_unknown_options_when_letters_are_not_known() {
        assert_eq!(
            invocation("ssh -Z -N nova").destination,
            Some("nova".to_owned())
        );
    }

    #[test]
    fn should_strip_user_when_destination_has_one() {
        assert_eq!(
            invocation("ssh alice@nova").destination,
            Some("nova".to_owned())
        );
    }

    #[test]
    fn should_strip_up_to_the_last_at_sign_when_user_contains_one() {
        assert_eq!(
            invocation("ssh a@b@nova").destination,
            Some("nova".to_owned())
        );
    }

    #[test]
    fn should_read_host_when_destination_is_an_ssh_url_with_port() {
        assert_eq!(
            invocation("ssh ssh://alice@nova:2222").destination,
            Some("nova".to_owned())
        );
    }

    #[test]
    fn should_read_host_when_destination_is_an_ssh_url_with_path() {
        assert_eq!(
            invocation("ssh ssh://nova/").destination,
            Some("nova".to_owned())
        );
    }

    #[test]
    fn should_remove_brackets_when_url_host_is_ipv6() {
        assert_eq!(
            invocation("ssh ssh://alice@[::1]:2222").destination,
            Some("::1".to_owned())
        );
    }

    #[test]
    fn should_remove_brackets_when_destination_is_bracketed_ipv6() {
        assert_eq!(
            invocation("ssh -N [::1]").destination,
            Some("::1".to_owned())
        );
    }

    #[test]
    fn should_ignore_remote_command_when_it_follows_the_destination() {
        assert_eq!(
            invocation("ssh nova ls -la").destination,
            Some("nova".to_owned())
        );
    }

    #[test]
    fn should_have_no_destination_when_only_options_are_given() {
        assert_eq!(invocation("ssh -N -L 1:a:1").destination, None);
    }

    #[test]
    fn should_have_no_destination_when_the_option_argument_is_missing() {
        assert_eq!(invocation("ssh -L").destination, None);
    }

    #[test]
    fn should_return_empty_invocation_when_there_are_no_arguments() {
        assert_eq!(
            parse_ssh_command_line(&[]),
            SshInvocation {
                destination: None,
                forwards: Vec::new()
            }
        );
    }

    #[test]
    fn should_treat_a_lone_dash_as_the_destination() {
        assert_eq!(invocation("ssh -").destination, Some("-".to_owned()));
    }
}
