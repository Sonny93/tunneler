//! Finds the `ssh` invocations and the forward options of a logical line.

use ssh_config::parse_local_forward_argument;

use super::tokens::Token;
use crate::external::{SshDestination, parse_ssh_command_line};
use crate::store::TunnelKind;

const SSH_PROGRAMS: [&str; 2] = ["ssh", "ssh.exe"];
const PATH_SEPARATORS: [char; 2] = ['/', '\\'];
const NO_ARGUMENT_FLAGS: &str = "46AaCfGgKkMNnqsTtVvXxYy";
const OPTION_PREFIX: char = '-';
const VARIABLE_PREFIXES: [char; 2] = ['$', '@'];
const SPEC_PUNCTUATION: [char; 5] = ['"', '\'', ',', ';', '`'];
const SPEC_START_CHARACTERS: [char; 3] = ['[', '$', '*'];
const BARE_SPEC_MINIMUM_SEPARATORS: usize = 2;
const FIELD_SEPARATOR: char = ':';

/// One `-L`, `-R` or `-D` specification found in a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RawForward {
    pub kind: TunnelKind,
    /// The specification as written, quotes and punctuation removed.
    pub text: String,
    /// 1-based.
    pub line: u32,
    /// The destination of the `ssh` invocation the option belongs to.
    pub destination: Option<SshDestination>,
}

/// What a logical line holds.
#[derive(Debug, Default)]
pub(super) struct LineScan {
    pub forwards: Vec<RawForward>,
    /// The destination of every `ssh` invocation, in text order.
    pub destinations: Vec<SshDestination>,
}

struct ForwardOption {
    kind: TunnelKind,
    value: String,
    line: u32,
    consumed: usize,
}

/// Reads the forward options and the `ssh` invocations of one logical line.
///
/// A line made only of `local_port:host:port` words is a list of bare local
/// forwards.
pub(super) fn scan_line(tokens: &[Token]) -> LineScan {
    if let Some(forwards) = bare_forwards(tokens) {
        return LineScan {
            forwards,
            destinations: Vec::new(),
        };
    }
    let mut scan = LineScan::default();
    let mut destination: Option<SshDestination> = None;
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        if token.is_separator {
            destination = None;
        } else if is_ssh_program(&token.text) {
            destination = invocation_destination(&tokens[index..]);
            scan.destinations.extend(destination.clone());
        } else if let Some(option) = read_forward_option(tokens, index) {
            scan.forwards.push(RawForward {
                kind: option.kind,
                text: option.value,
                line: option.line,
                destination: destination.clone(),
            });
            index += option.consumed;
            continue;
        }
        index += 1;
    }
    scan
}

fn is_ssh_program(text: &str) -> bool {
    let program = text.rsplit(PATH_SEPARATORS).next().unwrap_or(text);
    SSH_PROGRAMS
        .iter()
        .any(|candidate| program.eq_ignore_ascii_case(candidate))
}

fn invocation_destination(tokens: &[Token]) -> Option<SshDestination> {
    let arguments: Vec<String> = tokens
        .iter()
        .take_while(|token| !token.is_separator)
        .filter(|token| !is_variable(&token.text))
        .map(|token| token.text.clone())
        .collect();
    parse_ssh_command_line(&arguments).destination
}

fn is_variable(text: &str) -> bool {
    text.starts_with(VARIABLE_PREFIXES)
}

fn read_forward_option(tokens: &[Token], index: usize) -> Option<ForwardOption> {
    let token = tokens.get(index)?;
    let bundle = token.text.strip_prefix(OPTION_PREFIX)?;
    let (kind, glued_value) = split_bundle(bundle)?;
    if glued_value.is_empty() {
        let value = tokens.get(index + 1).filter(|next| !next.is_separator)?;
        return Some(ForwardOption {
            kind,
            value: clean_spec(&value.text),
            line: value.line,
            consumed: 2,
        });
    }
    let starts_like_spec = glued_value.starts_with(|character: char| {
        character.is_ascii_digit() || SPEC_START_CHARACTERS.contains(&character)
    });
    starts_like_spec.then(|| ForwardOption {
        kind,
        value: clean_spec(glued_value),
        line: token.line,
        consumed: 1,
    })
}

fn split_bundle(bundle: &str) -> Option<(TunnelKind, &str)> {
    let offset = bundle.find(|flag| !NO_ARGUMENT_FLAGS.contains(flag))?;
    let mut remaining = bundle[offset..].chars();
    let kind = match remaining.next()? {
        'L' => TunnelKind::Local,
        'R' => TunnelKind::Remote,
        'D' => TunnelKind::Dynamic,
        _ => return None,
    };
    Some((kind, remaining.as_str()))
}

fn clean_spec(text: &str) -> String {
    text.trim_matches(SPEC_PUNCTUATION).to_owned()
}

fn bare_forwards(tokens: &[Token]) -> Option<Vec<RawForward>> {
    let forwards: Option<Vec<RawForward>> = tokens.iter().map(bare_forward).collect();
    forwards.filter(|found| !found.is_empty())
}

fn bare_forward(token: &Token) -> Option<RawForward> {
    let text = clean_spec(&token.text);
    let separators = text.matches(FIELD_SEPARATOR).count();
    let is_forward = !token.is_separator
        && separators >= BARE_SPEC_MINIMUM_SEPARATORS
        && parse_local_forward_argument(&text).is_ok();
    is_forward.then_some(RawForward {
        kind: TunnelKind::Local,
        text,
        line: token.line,
        destination: None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::comments::CodeSegment;
    use super::super::tokens::tokenize;
    use super::*;

    fn scan(line: &str) -> LineScan {
        scan_line(&tokenize(&[CodeSegment {
            number: 1,
            text: line.to_owned(),
        }]))
    }

    fn specs(line: &str) -> Vec<(TunnelKind, String)> {
        scan(line)
            .forwards
            .into_iter()
            .map(|forward| (forward.kind, forward.text))
            .collect()
    }

    #[test]
    fn should_read_separate_and_glued_options_when_both_forms_are_used() {
        assert_eq!(
            specs("ssh -N -L 3333:localhost:3333 -L8080:db:80 -D 1080 nova"),
            vec![
                (TunnelKind::Local, "3333:localhost:3333".to_owned()),
                (TunnelKind::Local, "8080:db:80".to_owned()),
                (TunnelKind::Dynamic, "1080".to_owned()),
            ]
        );
    }

    #[test]
    fn should_read_option_when_flags_are_bundled_before_it() {
        assert_eq!(
            specs("ssh -NfL3333:localhost:3333 nova"),
            vec![(TunnelKind::Local, "3333:localhost:3333".to_owned())]
        );
    }

    #[test]
    fn should_ignore_powershell_parameters_when_they_start_with_a_forward_letter() {
        assert_eq!(specs("Get-ChildItem -Directory -Recurse -Log"), Vec::new());
    }

    #[test]
    fn should_read_spec_when_it_sits_in_a_quoted_string() {
        assert_eq!(
            specs("{ \"-L ${_}:127.0.0.1:${_}\" }"),
            vec![(TunnelKind::Local, "${_}:127.0.0.1:${_}".to_owned())]
        );
    }

    #[test]
    fn should_read_destination_when_variables_precede_it() {
        let found = scan("ssh -v -N @forwards jules@198.51.100.157");

        assert_eq!(found.destinations[0].host, "198.51.100.157");
    }

    #[test]
    fn should_find_ssh_when_program_is_a_windows_path() {
        let found = scan("& \"C:\\Windows\\System32\\OpenSSH\\ssh.exe\" -L 1:a:1 nova");

        assert_eq!(found.destinations[0].host, "nova");
    }

    #[test]
    fn should_attach_destination_to_forwards_when_they_follow_the_program() {
        let found = scan("ssh -L 1:a:1 nova");

        assert_eq!(
            found.forwards[0]
                .destination
                .as_ref()
                .map(|d| d.host.as_str()),
            Some("nova")
        );
    }

    #[test]
    fn should_stop_invocation_when_a_separator_ends_the_command() {
        let found = scan("ssh nova; echo -L 1:a:1");

        assert_eq!(found.forwards[0].destination, None);
    }

    #[test]
    fn should_read_bare_forwards_when_line_holds_only_specs() {
        assert_eq!(
            specs("3333:127.0.0.1:3333"),
            vec![(TunnelKind::Local, "3333:127.0.0.1:3333".to_owned())]
        );
    }

    #[test]
    fn should_not_read_bare_forward_when_line_holds_other_words() {
        assert_eq!(specs("Write-Host 3333:127.0.0.1:3333"), Vec::new());
    }
}
