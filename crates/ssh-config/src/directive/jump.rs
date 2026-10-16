use crate::error::ValueError;

use super::values::parse_port_number;

const NONE_VALUE: &str = "none";
const COMMENT_MARKER: char = '#';
const JUMP_SEPARATOR: char = ',';
const USER_SEPARATOR: char = '@';
const URI_SCHEME_PREFIX: &str = "ssh://";
const URI_PARAMETERS_SEPARATOR: char = ';';
const PATH_SEPARATOR: char = '/';
const PORT_SEPARATOR: char = ':';
const IPV6_OPEN: char = '[';
const IPV6_CLOSE: char = ']';
const DOMAIN_SEPARATOR: char = '.';

/// One hop of a `ProxyJump` chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpHost {
    /// The login name for this hop, when one was written.
    pub user: Option<String>,
    /// The host name or address, without IPv6 brackets.
    pub host: String,
    /// The SSH port of this hop, when one was written.
    pub port: Option<u16>,
}

/// The value of a `ProxyJump` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProxyJumpValue {
    None,
    Hosts(Vec<JumpHost>),
}

/// Parses the raw text after `ProxyJump`, quirks of OpenSSH `parse_jump` included:
/// everything from `#` and from the first whitespace is dropped, and `none` is
/// recognised on the untruncated text.
pub(crate) fn parse_proxy_jump(raw_value: &str) -> Result<ProxyJumpValue, ValueError> {
    let invalid = || ValueError::InvalidProxyJump {
        value: raw_value.to_owned(),
    };
    if raw_value.eq_ignore_ascii_case(NONE_VALUE) {
        return Ok(ProxyJumpValue::None);
    }
    let hosts = truncate_at_first_whitespace(strip_comment(raw_value))
        .split(JUMP_SEPARATOR)
        .map(parse_jump_element)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(invalid)?;
    Ok(ProxyJumpValue::Hosts(hosts))
}

fn strip_comment(text: &str) -> &str {
    text.find(COMMENT_MARKER)
        .map_or(text, |index| &text[..index])
}

fn truncate_at_first_whitespace(text: &str) -> &str {
    let mut characters = text.char_indices();
    characters.next();
    let cut = characters
        .find(|(_, character)| character.is_whitespace())
        .map(|(index, _)| index);
    cut.map_or(text, |index| &text[..index])
}

fn parse_jump_element(element: &str) -> Option<JumpHost> {
    match element.strip_prefix(URI_SCHEME_PREFIX) {
        Some(uri_body) => parse_uri_body(uri_body),
        None => parse_user_host_port(element),
    }
}

fn parse_user_host_port(text: &str) -> Option<JumpHost> {
    let (user, host_and_port) = split_user(text)?;
    let split = split_host_token(host_and_port)?;
    if split.token.is_empty() {
        return None;
    }
    let port = match split.delimiter {
        Some((PATH_SEPARATOR, _)) => return None,
        Some((_, port_text)) if !port_text.is_empty() => Some(parse_nonzero_port(port_text)?),
        _ => None,
    };
    Some(JumpHost {
        user: user.map(str::to_owned),
        host: clean_host_name(split.token).to_owned(),
        port,
    })
}

fn parse_nonzero_port(text: &str) -> Option<u16> {
    parse_port_number(text).filter(|port| *port != 0)
}

fn split_user(text: &str) -> Option<(Option<&str>, &str)> {
    let Some(separator_index) = text.rfind(USER_SEPARATOR) else {
        return Some((None, text));
    };
    let user = &text[..separator_index];
    if user.is_empty() {
        return None;
    }
    Some((Some(user), &text[separator_index + 1..]))
}

struct HostSplit<'text> {
    token: &'text str,
    delimiter: Option<(char, &'text str)>,
}

/// Splits `host[<delimiter>rest]` or `[ipv6][<delimiter>rest]` like OpenSSH
/// `hpdelim2`, where the delimiter is `:` or `/`.
fn split_host_token(text: &str) -> Option<HostSplit<'_>> {
    let search_start = if text.starts_with(IPV6_OPEN) {
        text.find(IPV6_CLOSE)? + 1
    } else {
        0
    };
    let Some(relative_index) = text[search_start..].find([PORT_SEPARATOR, PATH_SEPARATOR]) else {
        let is_complete = search_start == 0 || search_start == text.len();
        return is_complete.then_some(HostSplit {
            token: text,
            delimiter: None,
        });
    };
    let delimiter_index = search_start + relative_index;
    if search_start != 0 && delimiter_index != search_start {
        return None;
    }
    let delimiter = text[delimiter_index..].chars().next()?;
    Some(HostSplit {
        token: &text[..delimiter_index],
        delimiter: Some((delimiter, &text[delimiter_index + 1..])),
    })
}

fn clean_host_name(host: &str) -> &str {
    host.strip_prefix(IPV6_OPEN)
        .and_then(|inner| inner.strip_suffix(IPV6_CLOSE))
        .unwrap_or(host)
}

fn parse_uri_body(body: &str) -> Option<JumpHost> {
    let (user, host_part) = match body.split_once(USER_SEPARATOR) {
        Some((user_part, rest)) => (Some(decode_uri_user(user_part)?), rest),
        None => (None, body),
    };
    let split = split_host_token(host_part)?;
    let host = clean_host_name(split.token);
    if !is_valid_domain(host) {
        return None;
    }
    let port = match parse_uri_port(split.delimiter)? {
        UriPort::Absent => None,
        UriPort::Present(port) => Some(port),
    };
    Some(JumpHost {
        user,
        host: host.to_owned(),
        port,
    })
}

fn decode_uri_user(user_part: &str) -> Option<String> {
    let without_parameters = user_part
        .split_once(URI_PARAMETERS_SEPARATOR)
        .map_or(user_part, |(user, _)| user);
    if without_parameters.is_empty() {
        return None;
    }
    decode_percent_encoding(without_parameters)
}

enum UriPort {
    Absent,
    Present(u16),
}

fn parse_uri_port(delimiter: Option<(char, &str)>) -> Option<UriPort> {
    let Some((delimiter_character, rest)) = delimiter.filter(|(_, rest)| !rest.is_empty()) else {
        return Some(UriPort::Absent);
    };
    if delimiter_character != PORT_SEPARATOR {
        return None;
    }
    let (port_text, path) = match rest.split_once(PATH_SEPARATOR) {
        Some((port_text, path)) => (port_text, Some(path)),
        None => (rest, None),
    };
    if path.is_some_and(|path| !path.is_empty()) {
        return None;
    }
    parse_nonzero_port(port_text).map(UriPort::Present)
}

fn decode_percent_encoding(text: &str) -> Option<String> {
    let mut decoded = Vec::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => decoded.push(b' '),
            b'%' => {
                let hex_digits = text.get(index + 1..index + 3)?;
                decoded.push(u8::from_str_radix(hex_digits, 16).ok()?);
                index += 2;
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8(decoded).ok()
}

/// OpenSSH `valid_domain` without lowercasing.
fn is_valid_domain(name: &str) -> bool {
    let Some(first) = name.chars().next() else {
        return false;
    };
    if !(first.is_ascii_alphanumeric() || first == '_') {
        return false;
    }
    let has_valid_characters = name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_'));
    let has_consecutive_separators = name
        .as_bytes()
        .windows(2)
        .any(|pair| pair == [DOMAIN_SEPARATOR as u8; 2]);
    has_valid_characters && !has_consecutive_separators
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jump(user: Option<&str>, host: &str, port: Option<u16>) -> JumpHost {
        JumpHost {
            user: user.map(str::to_owned),
            host: host.to_owned(),
            port,
        }
    }

    fn hosts(raw_value: &str) -> Vec<JumpHost> {
        match parse_proxy_jump(raw_value).unwrap() {
            ProxyJumpValue::Hosts(hosts) => hosts,
            ProxyJumpValue::None => Vec::new(),
        }
    }

    #[test]
    fn should_return_none_when_value_is_none() {
        assert_eq!(parse_proxy_jump("none").unwrap(), ProxyJumpValue::None);
    }

    #[test]
    fn should_return_none_when_value_is_none_in_any_case() {
        assert_eq!(parse_proxy_jump("NoNe").unwrap(), ProxyJumpValue::None);
    }

    #[test]
    fn should_parse_bare_host_when_value_is_a_host() {
        assert_eq!(hosts("bastion"), vec![jump(None, "bastion", None)]);
    }

    #[test]
    fn should_parse_user_and_port_when_value_has_them() {
        assert_eq!(
            hosts("alice@bastion:2222"),
            vec![jump(Some("alice"), "bastion", Some(2222))]
        );
    }

    #[test]
    fn should_keep_order_when_value_is_a_list() {
        assert_eq!(
            hosts("a,b:22,u@c"),
            vec![
                jump(None, "a", None),
                jump(None, "b", Some(22)),
                jump(Some("u"), "c", None)
            ]
        );
    }

    #[test]
    fn should_parse_bracketed_ipv6_when_host_is_bracketed() {
        assert_eq!(hosts("[::1]:5"), vec![jump(None, "::1", Some(5))]);
    }

    #[test]
    fn should_parse_ssh_uri_when_value_starts_with_the_scheme() {
        assert_eq!(
            hosts("ssh://v@b:2200"),
            vec![jump(Some("v"), "b", Some(2200))]
        );
    }

    #[test]
    fn should_decode_user_when_uri_user_is_percent_encoded() {
        assert_eq!(hosts("ssh://u%40x@h"), vec![jump(Some("u@x"), "h", None)]);
    }

    #[test]
    fn should_accept_trailing_slash_when_uri_has_an_empty_path() {
        assert_eq!(hosts("ssh://h/"), vec![jump(None, "h", None)]);
    }

    #[test]
    fn should_mix_uris_and_plain_hosts_when_value_is_a_list() {
        assert_eq!(
            hosts("a,ssh://b:2200"),
            vec![jump(None, "a", None), jump(None, "b", Some(2200))]
        );
    }

    #[test]
    fn should_fail_when_uri_has_a_path() {
        assert!(parse_proxy_jump("ssh://u@h/path").is_err());
    }

    #[test]
    fn should_fail_when_uri_host_is_an_ipv6_literal() {
        assert!(parse_proxy_jump("ssh://[::1]").is_err());
    }

    #[test]
    fn should_fail_when_uri_user_encoding_is_invalid() {
        assert!(parse_proxy_jump("ssh://u%zz@h").is_err());
    }

    #[test]
    fn should_fail_when_element_is_empty() {
        assert!(parse_proxy_jump("a,,b").is_err());
    }

    #[test]
    fn should_fail_when_user_is_empty() {
        assert!(parse_proxy_jump("@host").is_err());
    }

    #[test]
    fn should_fail_when_port_is_not_numeric() {
        assert!(parse_proxy_jump("host:ssh").is_err());
    }

    #[test]
    fn should_fail_when_port_is_zero() {
        assert!(parse_proxy_jump("host:0").is_err());
    }

    #[test]
    fn should_fail_when_unbracketed_ipv6_is_used() {
        assert!(parse_proxy_jump("::1").is_err());
    }

    #[test]
    fn should_drop_trailing_comment_when_value_has_one() {
        assert_eq!(
            hosts("a,b # note"),
            vec![jump(None, "a", None), jump(None, "b", None)]
        );
    }

    #[test]
    fn should_treat_none_followed_by_comment_as_a_host_named_none() {
        assert_eq!(hosts("none # c"), vec![jump(None, "none", None)]);
    }

    #[test]
    fn should_fail_when_whitespace_follows_a_comma() {
        assert!(parse_proxy_jump("a, b").is_err());
    }
}
