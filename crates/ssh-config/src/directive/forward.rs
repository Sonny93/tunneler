use std::path::PathBuf;

use crate::environment::LocalEnvironment;
use crate::error::ValueError;
use crate::tokens::expand_environment;

use super::values::parse_port_number;

const FIELD_SEPARATOR: char = ':';
const PATH_SEPARATOR: char = '/';
const ESCAPE: char = '\\';
const IPV6_OPEN: char = '[';
const IPV6_CLOSE: char = ']';
const MAX_FIELDS: usize = 4;
const MAX_HOST_LENGTH: usize = 1025;
const MAX_SOCKET_PATH_LENGTH: usize = 108;

/// One end of a forwarding: a TCP address and port, or a Unix socket path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// A TCP endpoint.
    Tcp {
        /// The bind address or host; `None` when none was written.
        address: Option<String>,
        /// The port.
        port: u16,
    },
    /// A Unix domain socket path.
    Unix(PathBuf),
}

/// A `LocalForward` or fixed `RemoteForward`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardSpec {
    /// Where the forwarding listens.
    pub listen: Endpoint,
    /// Where connections are forwarded to.
    pub target: Endpoint,
}

/// A `DynamicForward`: a SOCKS proxy listening on `listen`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicForward {
    /// Where the SOCKS proxy listens.
    pub listen: Endpoint,
}

/// A `RemoteForward`, which is a SOCKS proxy on the remote side when it has
/// no target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteForward {
    /// A forwarding to a fixed target.
    Fixed(ForwardSpec),
    /// A SOCKS proxy on the remote side.
    Dynamic(DynamicForward),
}

struct Field {
    text: String,
    is_path: bool,
}

enum FieldRead {
    Field(Field, String),
    End,
    Malformed,
}

pub(crate) fn parse_local_forward(
    arguments: &[String],
    environment: &dyn LocalEnvironment,
) -> Result<ForwardSpec, ValueError> {
    let [listen, target] = arguments else {
        return Err(missing_or_extra(arguments, true));
    };
    let specification = format!("{listen}{FIELD_SEPARATOR}{target}");
    let expanded = expand_specification(&specification, environment)?;
    parse_fixed_forward(&expanded, ListenPort::NonZero)
}

pub(crate) fn parse_remote_forward(
    arguments: &[String],
    environment: &dyn LocalEnvironment,
) -> Result<RemoteForward, ValueError> {
    match arguments {
        [listen] => {
            let expanded = expand_specification(listen, environment)?;
            parse_dynamic_listen(&expanded, ListenPort::Any)
                .map(|listen| RemoteForward::Dynamic(DynamicForward { listen }))
        }
        [listen, target] => {
            let specification = format!("{listen}{FIELD_SEPARATOR}{target}");
            let expanded = expand_specification(&specification, environment)?;
            parse_fixed_forward(&expanded, ListenPort::Any).map(RemoteForward::Fixed)
        }
        _ => Err(missing_or_extra(arguments, false)),
    }
}

pub(crate) fn parse_dynamic_forward(
    arguments: &[String],
    environment: &dyn LocalEnvironment,
) -> Result<DynamicForward, ValueError> {
    let [listen] = arguments else {
        return Err(missing_or_extra(arguments, false));
    };
    let expanded = expand_specification(listen, environment)?;
    parse_dynamic_forward_argument(&expanded)
}

/// Parses the argument of `ssh -L`: `[bind_address:]port:host:hostport` or one
/// of the Unix socket forms. Unlike the configuration directive, nothing is
/// expanded: the shell already did.
pub fn parse_local_forward_argument(argument: &str) -> Result<ForwardSpec, ValueError> {
    parse_fixed_forward(argument, ListenPort::NonZero)
}

/// Parses the argument of `ssh -R`: `[bind_address:]port:host:hostport` is a
/// fixed forward and `[bind_address:]port` a SOCKS proxy on the remote side.
/// Nothing is expanded.
pub fn parse_remote_forward_argument(argument: &str) -> Result<RemoteForward, ValueError> {
    parse_fixed_forward(argument, ListenPort::Any)
        .map(RemoteForward::Fixed)
        .or_else(|_| {
            parse_dynamic_listen(argument, ListenPort::Any)
                .map(|listen| RemoteForward::Dynamic(DynamicForward { listen }))
        })
}

/// Parses the argument of `ssh -D`: `[bind_address:]port`. Nothing is
/// expanded.
pub fn parse_dynamic_forward_argument(argument: &str) -> Result<DynamicForward, ValueError> {
    parse_dynamic_listen(argument, ListenPort::NonZero).map(|listen| DynamicForward { listen })
}

fn expand_specification(
    specification: &str,
    environment: &dyn LocalEnvironment,
) -> Result<String, ValueError> {
    expand_environment(specification, environment).map_err(|_| ValueError::InvalidForward {
        value: specification.to_owned(),
    })
}

fn missing_or_extra(arguments: &[String], needs_target: bool) -> ValueError {
    match arguments.len() {
        0 => ValueError::MissingArgument,
        1 if needs_target => ValueError::MissingForwardTarget,
        _ => ValueError::ExtraArguments,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListenPort {
    NonZero,
    Any,
}

fn parse_fixed_forward(
    specification: &str,
    listen_port: ListenPort,
) -> Result<ForwardSpec, ValueError> {
    let invalid = || ValueError::InvalidForward {
        value: specification.to_owned(),
    };
    let fields = read_fields(specification).ok_or_else(invalid)?;
    let (listen, target) = interpret_fixed_fields(&fields).ok_or_else(invalid)?;
    let is_valid = is_valid_listen(&listen, listen_port) && is_valid_target(&target);
    is_valid
        .then_some(ForwardSpec { listen, target })
        .ok_or_else(invalid)
}

fn parse_dynamic_listen(
    specification: &str,
    listen_port: ListenPort,
) -> Result<Endpoint, ValueError> {
    let invalid = || ValueError::InvalidForward {
        value: specification.to_owned(),
    };
    let fields = read_fields(specification).ok_or_else(invalid)?;
    let listen = interpret_dynamic_fields(&fields).ok_or_else(invalid)?;
    is_valid_listen(&listen, listen_port)
        .then_some(listen)
        .ok_or_else(invalid)
}

fn read_fields(specification: &str) -> Option<Vec<Field>> {
    let mut remainder = specification.trim_start().to_owned();
    let mut fields = Vec::new();
    while fields.len() < MAX_FIELDS {
        match read_field(&remainder) {
            FieldRead::Field(field, rest) => {
                fields.push(field);
                remainder = rest;
            }
            FieldRead::End => {
                remainder.clear();
                break;
            }
            FieldRead::Malformed => return None,
        }
    }
    remainder.is_empty().then_some(fields)
}

fn read_field(text: &str) -> FieldRead {
    if text.is_empty() {
        return FieldRead::End;
    }
    if text.starts_with(IPV6_OPEN) {
        return read_bracketed_field(text).map_or(FieldRead::Malformed, |(field, rest)| {
            FieldRead::Field(field, rest)
        });
    }
    read_plain_field(text).map_or(FieldRead::Malformed, |(field, rest)| {
        FieldRead::Field(field, rest)
    })
}

fn read_bracketed_field(text: &str) -> Option<(Field, String)> {
    let closing_index = text.find(IPV6_CLOSE)?;
    let inside = &text[1..closing_index];
    let after_bracket = &text[closing_index + 1..];
    let rest = match after_bracket.strip_prefix(FIELD_SEPARATOR) {
        Some(rest) => rest,
        None if after_bracket.is_empty() => after_bracket,
        None => return None,
    };
    let field = Field {
        text: inside.to_owned(),
        is_path: inside.contains(PATH_SEPARATOR),
    };
    Some((field, rest.to_owned()))
}

fn read_plain_field(text: &str) -> Option<(Field, String)> {
    let mut field_text = String::new();
    let mut is_path = false;
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        match character {
            ESCAPE => field_text.push(characters.next()?),
            FIELD_SEPARATOR => {
                let field = Field {
                    text: field_text,
                    is_path,
                };
                return Some((field, characters.collect()));
            }
            PATH_SEPARATOR => {
                is_path = true;
                field_text.push(character);
            }
            _ => field_text.push(character),
        }
    }
    Some((
        Field {
            text: field_text,
            is_path,
        },
        String::new(),
    ))
}

fn tcp_endpoint(address: Option<&Field>, port: &Field) -> Option<Endpoint> {
    let port = port_value(port)?;
    Some(Endpoint::Tcp {
        address: address.map(|field| field.text.clone()),
        port,
    })
}

fn port_value(field: &Field) -> Option<u16> {
    parse_port_number(&field.text)
}

fn unix_endpoint(field: &Field) -> Endpoint {
    Endpoint::Unix(PathBuf::from(&field.text))
}

fn interpret_fixed_fields(fields: &[Field]) -> Option<(Endpoint, Endpoint)> {
    match fields {
        [listen_path, connect_path] if listen_path.is_path && connect_path.is_path => {
            Some((unix_endpoint(listen_path), unix_endpoint(connect_path)))
        }
        [listen_port, connect_path] if connect_path.is_path => Some((
            tcp_endpoint(None, listen_port)?,
            unix_endpoint(connect_path),
        )),
        [listen_path, connect_host, connect_port] if listen_path.is_path => Some((
            unix_endpoint(listen_path),
            tcp_endpoint(Some(connect_host), connect_port)?,
        )),
        [listen_host, listen_port, connect_path] if connect_path.is_path => Some((
            tcp_endpoint(Some(listen_host), listen_port)?,
            unix_endpoint(connect_path),
        )),
        [listen_port, connect_host, connect_port] => Some((
            tcp_endpoint(None, listen_port)?,
            tcp_endpoint(Some(connect_host), connect_port)?,
        )),
        [listen_host, listen_port, connect_host, connect_port] => Some((
            tcp_endpoint(Some(listen_host), listen_port)?,
            tcp_endpoint(Some(connect_host), connect_port)?,
        )),
        _ => None,
    }
}

fn interpret_dynamic_fields(fields: &[Field]) -> Option<Endpoint> {
    match fields {
        [listen] if listen.is_path => Some(unix_endpoint(listen)),
        [listen_port] => tcp_endpoint(None, listen_port),
        [listen_host, listen_port] => tcp_endpoint(Some(listen_host), listen_port),
        _ => None,
    }
}

fn is_valid_listen(endpoint: &Endpoint, listen_port: ListenPort) -> bool {
    match endpoint {
        Endpoint::Tcp { address, port } => {
            let is_port_allowed = listen_port == ListenPort::Any || *port != 0;
            let is_address_short = address
                .as_ref()
                .is_none_or(|address| address.len() < MAX_HOST_LENGTH);
            is_port_allowed && is_address_short
        }
        Endpoint::Unix(path) => path.as_os_str().len() < MAX_SOCKET_PATH_LENGTH,
    }
}

fn is_valid_target(endpoint: &Endpoint) -> bool {
    match endpoint {
        Endpoint::Tcp { address, port } => {
            *port != 0
                && address
                    .as_ref()
                    .is_none_or(|address| address.len() < MAX_HOST_LENGTH)
        }
        Endpoint::Unix(path) => path.as_os_str().len() < MAX_SOCKET_PATH_LENGTH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::fake::FakeEnvironment;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn tcp(address: Option<&str>, port: u16) -> Endpoint {
        Endpoint::Tcp {
            address: address.map(str::to_owned),
            port,
        }
    }

    fn unix(path: &str) -> Endpoint {
        Endpoint::Unix(PathBuf::from(path))
    }

    fn local(listen: &str, target: &str) -> Result<ForwardSpec, ValueError> {
        parse_local_forward(&strings(&[listen, target]), &FakeEnvironment::default())
    }

    #[test]
    fn should_parse_port_and_target_when_listen_has_no_address() {
        assert_eq!(
            local("3333", "localhost:3333"),
            Ok(ForwardSpec {
                listen: tcp(None, 3333),
                target: tcp(Some("localhost"), 3333)
            })
        );
    }

    #[test]
    fn should_parse_bind_address_when_listen_has_one() {
        assert_eq!(
            local("0.0.0.0:80", "web:8080").unwrap().listen,
            tcp(Some("0.0.0.0"), 80)
        );
    }

    #[test]
    fn should_parse_ipv6_addresses_when_they_are_bracketed() {
        assert_eq!(
            local("[::1]:4444", "[fe80::1]:5555"),
            Ok(ForwardSpec {
                listen: tcp(Some("::1"), 4444),
                target: tcp(Some("fe80::1"), 5555)
            })
        );
    }

    #[test]
    fn should_parse_unix_socket_target_when_target_is_a_path() {
        assert_eq!(
            local("1.2.3.4:6000", "/tmp/s.sock"),
            Ok(ForwardSpec {
                listen: tcp(Some("1.2.3.4"), 6000),
                target: unix("/tmp/s.sock")
            })
        );
    }

    #[test]
    fn should_parse_unix_socket_listen_when_listen_is_a_path() {
        assert_eq!(
            local("/tmp/l.sock", "host:80"),
            Ok(ForwardSpec {
                listen: unix("/tmp/l.sock"),
                target: tcp(Some("host"), 80)
            })
        );
    }

    #[test]
    fn should_parse_two_unix_sockets_when_both_ends_are_paths() {
        assert_eq!(
            local("/tmp/l.sock", "/tmp/r.sock"),
            Ok(ForwardSpec {
                listen: unix("/tmp/l.sock"),
                target: unix("/tmp/r.sock")
            })
        );
    }

    #[test]
    fn should_parse_port_listen_and_socket_target_when_listen_is_a_bare_port() {
        assert_eq!(
            local("8080", "/var/run/app.sock").unwrap().listen,
            tcp(None, 8080)
        );
    }

    #[test]
    fn should_keep_colon_when_it_is_escaped_with_a_backslash() {
        assert_eq!(
            local("80", "ho\\:st:81").unwrap().target,
            tcp(Some("ho:st"), 81)
        );
    }

    #[test]
    fn should_expand_environment_variables_when_spec_has_them() {
        let mut environment = FakeEnvironment::default();
        environment
            .variables
            .insert("RUNTIME".to_owned(), "/run/user/1000".to_owned());
        let spec = parse_local_forward(&strings(&["${RUNTIME}/l.sock", "h:1"]), &environment);
        assert_eq!(spec.unwrap().listen, unix("/run/user/1000/l.sock"));
    }

    #[test]
    fn should_fail_when_local_forward_has_no_target() {
        assert_eq!(
            parse_local_forward(&strings(&["80"]), &FakeEnvironment::default()),
            Err(ValueError::MissingForwardTarget)
        );
    }

    #[test]
    fn should_fail_when_local_forward_has_no_argument() {
        assert_eq!(
            parse_local_forward(&[], &FakeEnvironment::default()),
            Err(ValueError::MissingArgument)
        );
    }

    #[test]
    fn should_fail_when_local_forward_has_extra_arguments() {
        assert_eq!(
            parse_local_forward(&strings(&["1", "h:2", "x"]), &FakeEnvironment::default()),
            Err(ValueError::ExtraArguments)
        );
    }

    #[test]
    fn should_fail_when_local_listen_port_is_zero() {
        assert!(local("0", "h:1").is_err());
    }

    #[test]
    fn should_fail_when_target_port_is_zero() {
        assert!(local("1", "h:0").is_err());
    }

    #[test]
    fn should_fail_when_port_is_not_numeric() {
        assert!(local("1", "h:http").is_err());
    }

    #[test]
    fn should_fail_when_bracket_is_not_closed() {
        assert!(local("[::1:80", "h:1").is_err());
    }

    #[test]
    fn should_fail_when_bracketed_field_is_followed_by_garbage() {
        assert!(local("[::1]x:80", "h:1").is_err());
    }

    #[test]
    fn should_fail_when_spec_has_too_many_fields() {
        assert!(local("a:1:b", "c:2:3").is_err());
    }

    #[test]
    fn should_fail_when_spec_has_a_trailing_backslash() {
        assert!(local("80", "host:81\\").is_err());
    }

    #[test]
    fn should_fail_when_environment_variable_is_undefined() {
        assert!(local("${NOPE}", "h:1").is_err());
    }

    #[test]
    fn should_fail_when_socket_path_is_too_long() {
        let long_path = format!("/{}", "a".repeat(MAX_SOCKET_PATH_LENGTH));
        assert!(local(&long_path, "h:1").is_err());
    }

    fn remote(arguments: &[&str]) -> Result<RemoteForward, ValueError> {
        parse_remote_forward(&strings(arguments), &FakeEnvironment::default())
    }

    #[test]
    fn should_parse_fixed_remote_forward_when_target_is_given() {
        assert_eq!(
            remote(&["9000", "localhost:9000"]),
            Ok(RemoteForward::Fixed(ForwardSpec {
                listen: tcp(None, 9000),
                target: tcp(Some("localhost"), 9000)
            }))
        );
    }

    #[test]
    fn should_parse_dynamic_remote_forward_when_only_one_argument_is_given() {
        assert_eq!(
            remote(&["9001"]),
            Ok(RemoteForward::Dynamic(DynamicForward {
                listen: tcp(None, 9001)
            }))
        );
    }

    #[test]
    fn should_allow_port_zero_when_remote_forward_listens() {
        assert!(remote(&["0", "h:1"]).is_ok());
    }

    #[test]
    fn should_keep_wildcard_address_when_remote_forward_binds_to_star() {
        assert_eq!(
            remote(&["*:9002", "h:2"]),
            Ok(RemoteForward::Fixed(ForwardSpec {
                listen: tcp(Some("*"), 9002),
                target: tcp(Some("h"), 2)
            }))
        );
    }

    fn dynamic(argument: &str) -> Result<DynamicForward, ValueError> {
        parse_dynamic_forward(&strings(&[argument]), &FakeEnvironment::default())
    }

    #[test]
    fn should_parse_port_when_dynamic_forward_has_no_bind_address() {
        assert_eq!(
            dynamic("1080"),
            Ok(DynamicForward {
                listen: tcp(None, 1080)
            })
        );
    }

    #[test]
    fn should_parse_bind_address_when_dynamic_forward_has_one() {
        assert_eq!(
            dynamic("0.0.0.0:1081"),
            Ok(DynamicForward {
                listen: tcp(Some("0.0.0.0"), 1081)
            })
        );
    }

    #[test]
    fn should_parse_bracketed_ipv6_when_dynamic_forward_binds_to_it() {
        assert_eq!(
            dynamic("[::]:1082"),
            Ok(DynamicForward {
                listen: tcp(Some("::"), 1082)
            })
        );
    }

    #[test]
    fn should_parse_socket_when_dynamic_forward_listens_on_a_path() {
        assert_eq!(
            dynamic("/tmp/socks.sock"),
            Ok(DynamicForward {
                listen: unix("/tmp/socks.sock")
            })
        );
    }

    #[test]
    fn should_fail_when_dynamic_forward_port_is_zero() {
        assert!(dynamic("0").is_err());
    }

    #[test]
    fn should_fail_when_dynamic_forward_has_a_target() {
        assert!(dynamic("1:2:3").is_err());
    }

    #[test]
    fn should_fail_when_dynamic_forward_has_extra_arguments() {
        assert_eq!(
            parse_dynamic_forward(&strings(&["1", "2"]), &FakeEnvironment::default()),
            Err(ValueError::ExtraArguments)
        );
    }

    #[test]
    fn should_parse_separated_local_argument_when_it_has_four_fields() {
        assert_eq!(
            parse_local_forward_argument("127.0.0.1:3333:db:5432"),
            Ok(ForwardSpec {
                listen: tcp(Some("127.0.0.1"), 3333),
                target: tcp(Some("db"), 5432)
            })
        );
    }

    #[test]
    fn should_parse_bracketed_ipv6_when_local_argument_uses_it() {
        assert_eq!(
            parse_local_forward_argument("[::1]:4444:[fe80::1]:5555"),
            Ok(ForwardSpec {
                listen: tcp(Some("::1"), 4444),
                target: tcp(Some("fe80::1"), 5555)
            })
        );
    }

    #[test]
    fn should_not_expand_environment_when_argument_is_local() {
        let spec = parse_local_forward_argument("80:${HOME}:81").unwrap();
        assert_eq!(spec.target, tcp(Some("${HOME}"), 81));
    }

    #[test]
    fn should_fail_when_local_argument_has_no_target() {
        assert!(parse_local_forward_argument("3333").is_err());
    }

    #[test]
    fn should_fail_when_local_argument_listens_on_port_zero() {
        assert!(parse_local_forward_argument("0:h:1").is_err());
    }

    #[test]
    fn should_parse_fixed_when_remote_argument_has_a_target() {
        assert_eq!(
            parse_remote_forward_argument("localhost:9000:app:80"),
            Ok(RemoteForward::Fixed(ForwardSpec {
                listen: tcp(Some("localhost"), 9000),
                target: tcp(Some("app"), 80)
            }))
        );
    }

    #[test]
    fn should_parse_dynamic_when_remote_argument_has_only_a_port() {
        assert_eq!(
            parse_remote_forward_argument("9001"),
            Ok(RemoteForward::Dynamic(DynamicForward {
                listen: tcp(None, 9001)
            }))
        );
    }

    #[test]
    fn should_parse_dynamic_with_bind_address_when_remote_argument_has_two_fields() {
        assert_eq!(
            parse_remote_forward_argument("[::1]:9001"),
            Ok(RemoteForward::Dynamic(DynamicForward {
                listen: tcp(Some("::1"), 9001)
            }))
        );
    }

    #[test]
    fn should_allow_port_zero_when_remote_argument_listens() {
        assert!(parse_remote_forward_argument("0:h:1").is_ok());
    }

    #[test]
    fn should_fail_when_remote_argument_is_invalid() {
        assert!(parse_remote_forward_argument("a:b:c:d:e").is_err());
    }

    #[test]
    fn should_not_expand_environment_when_argument_is_remote() {
        let forward = parse_remote_forward_argument("80:${HOME}:81");
        assert_eq!(
            forward,
            Ok(RemoteForward::Fixed(ForwardSpec {
                listen: tcp(None, 80),
                target: tcp(Some("${HOME}"), 81)
            }))
        );
    }

    #[test]
    fn should_parse_port_when_dynamic_argument_has_no_bind_address() {
        assert_eq!(
            parse_dynamic_forward_argument("1080"),
            Ok(DynamicForward {
                listen: tcp(None, 1080)
            })
        );
    }

    #[test]
    fn should_parse_bind_address_when_dynamic_argument_has_one() {
        assert_eq!(
            parse_dynamic_forward_argument("[::]:1082"),
            Ok(DynamicForward {
                listen: tcp(Some("::"), 1082)
            })
        );
    }

    #[test]
    fn should_fail_when_dynamic_argument_is_invalid() {
        assert!(parse_dynamic_forward_argument("0").is_err());
    }

    #[test]
    fn should_not_expand_environment_when_argument_is_dynamic() {
        assert!(parse_dynamic_forward_argument("${HOME}").is_err());
    }
}
