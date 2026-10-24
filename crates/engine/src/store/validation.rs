use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use thiserror::Error;

use super::model::{
    BROWSER_OPENER_ID, MAX_STANDBY_VOLUME, MIN_STANDBY_VOLUME, Opener, Settings, StoreDocument,
    TunnelDefinition, TunnelId, TunnelKind, X11Launcher,
};

const MIN_PORT: u16 = 1;

/// Names a tunnel in an error message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelReference {
    /// The tunnel identifier.
    pub id: TunnelId,
    /// The tunnel label, such as `3333 → nova:3333`.
    pub label: String,
}

impl TunnelReference {
    fn of(tunnel: &TunnelDefinition) -> Self {
        Self {
            id: tunnel.id,
            label: tunnel.label(),
        }
    }
}

impl fmt::Display for TunnelReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "tunnel \"{}\" ({})", self.label, self.id)
    }
}

/// What is wrong with a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TagProblem {
    /// The tag is empty.
    #[error("it is empty")]
    Empty,
    /// The tag contains whitespace.
    #[error("it contains whitespace")]
    ContainsWhitespace,
    /// The tag contains a comma.
    #[error("it contains a comma")]
    ContainsComma,
    /// The tag contains a colon, which the CLI reads as `host:port`.
    #[error("it contains a colon (the CLI reads host:port as a port selector)")]
    ContainsColon,
    /// The tag is only digits, which the CLI reads as a port.
    #[error("it is only digits (the CLI reads a number as a port selector)")]
    OnlyDigits,
}

/// What is wrong with the path of an open URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum OpenPathProblem {
    /// The path does not start with a slash.
    #[error("it must start with \"/\"")]
    MissingLeadingSlash,
    /// The path starts with two slashes, which a browser reads as a host.
    #[error("it must not start with \"//\"")]
    StartsWithDoubleSlash,
    /// The path contains whitespace.
    #[error("it contains whitespace")]
    ContainsWhitespace,
    /// The path contains a control character.
    #[error("it contains a control character")]
    ContainsControlCharacter,
}

/// What is wrong with the id of an opener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum OpenerIdProblem {
    /// The id is empty.
    #[error("it is empty")]
    Empty,
    /// The id holds something other than lowercase letters, digits and dashes.
    #[error("it may only hold lowercase letters, digits and dashes")]
    InvalidCharacter,
    /// The id is the one that selects the browser.
    #[error("\"browser\" is reserved for the browser")]
    Reserved,
}

/// A rule broken by the content of the store.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    /// Two tunnels share an identifier.
    #[error("{tunnel}: the id is already used by another tunnel")]
    DuplicateId {
        /// The second tunnel carrying the identifier.
        tunnel: TunnelReference,
    },
    /// The host alias is empty.
    #[error("{tunnel}: the host must not be empty")]
    EmptyHost {
        /// The offending tunnel.
        tunnel: TunnelReference,
    },
    /// An optional text field is present but empty.
    #[error("{tunnel}: {field} must not be empty when set")]
    EmptyField {
        /// The offending tunnel.
        tunnel: TunnelReference,
        /// The field name as written in the file.
        field: &'static str,
    },
    /// The listen port is zero.
    #[error("{tunnel}: listen_port must be between 1 and 65535, found 0")]
    ZeroListenPort {
        /// The offending tunnel.
        tunnel: TunnelReference,
    },
    /// The target port is zero.
    #[error("{tunnel}: target_port must be between 1 and 65535, found 0")]
    ZeroTargetPort {
        /// The offending tunnel.
        tunnel: TunnelReference,
    },
    /// A dynamic tunnel names a target, but its clients choose where to go.
    #[error(
        "{tunnel}: {field} is not allowed on a dynamic tunnel, the SOCKS client names the target"
    )]
    DynamicTunnelWithTarget {
        /// The offending tunnel.
        tunnel: TunnelReference,
        /// The field name as written in the file.
        field: &'static str,
    },
    /// Two local or dynamic tunnels listen on the same local address and port.
    #[error("{tunnel}: {listen_address}:{listen_port} is already used by {other}")]
    LocalListenConflict {
        /// The later tunnel.
        tunnel: TunnelReference,
        /// The tunnel that holds the address first.
        other: TunnelReference,
        /// The contested address.
        listen_address: String,
        /// The contested port.
        listen_port: u16,
    },
    /// Two remote tunnels of the same host listen on the same remote address
    /// and port.
    #[error("{tunnel}: {listen_address}:{listen_port} on host {host} is already used by {other}")]
    RemoteListenConflict {
        /// The later tunnel.
        tunnel: TunnelReference,
        /// The tunnel that holds the address first.
        other: TunnelReference,
        /// The host both tunnels go through.
        host: String,
        /// The contested address.
        listen_address: String,
        /// The contested port.
        listen_port: u16,
    },
    /// A tag breaks the tag syntax.
    #[error("{tunnel}: tag \"{tag}\" is invalid: {problem}")]
    InvalidTag {
        /// The offending tunnel.
        tunnel: TunnelReference,
        /// The tag.
        tag: String,
        /// What is wrong with it.
        problem: TagProblem,
    },
    /// A tunnel lists the same tag twice.
    #[error("{tunnel}: tag \"{tag}\" is listed more than once")]
    DuplicateTag {
        /// The offending tunnel.
        tunnel: TunnelReference,
        /// The repeated tag.
        tag: String,
    },
    /// The path of the open URL breaks the path rules.
    #[error("{tunnel}: open.path \"{path}\" is invalid: {problem}")]
    InvalidOpenPath {
        /// The offending tunnel.
        tunnel: TunnelReference,
        /// The path.
        path: String,
        /// What is wrong with it.
        problem: OpenPathProblem,
    },
    /// The id of an opener is not acceptable.
    #[error("openers entry {position}: the id \"{id}\" is invalid: {problem}")]
    InvalidOpenerId {
        /// The 1-based position of the opener.
        position: usize,
        /// The id.
        id: String,
        /// What is wrong with it.
        problem: OpenerIdProblem,
    },
    /// Two openers share an id.
    #[error("openers: the id \"{id}\" is defined more than once")]
    DuplicateOpenerId {
        /// The repeated id.
        id: String,
    },
    /// An opener field is empty.
    #[error("openers entry {position}: {field} must not be empty")]
    EmptyOpenerField {
        /// The 1-based position of the opener.
        position: usize,
        /// The field name as written in the file.
        field: &'static str,
    },
    /// An argument of an opener holds a placeholder that does not exist.
    #[error(
        "openers entry {position}: the argument \"{argument}\" holds the unknown placeholder {placeholder}; only {{host}}, {{port}} and {{url}} exist"
    )]
    UnknownOpenerPlaceholder {
        /// The 1-based position of the opener.
        position: usize,
        /// The argument.
        argument: String,
        /// The placeholder, braces included.
        placeholder: String,
    },
    /// The first reconnection delay exceeds the maximum delay.
    #[error(
        "settings: reconnect_initial_delay_ms ({}) must not exceed reconnect_max_delay_ms ({})",
        initial.as_millis(),
        max.as_millis()
    )]
    ReconnectDelaysOutOfOrder {
        /// The configured initial delay.
        initial: Duration,
        /// The configured maximum delay.
        max: Duration,
    },
    /// The first reconnection delay is zero, which would retry in a busy loop.
    #[error("settings: reconnect_initial_delay_ms must be greater than 0")]
    ZeroReconnectInitialDelay,
    /// Keepalive is enabled but tolerates no missed probe.
    #[error(
        "settings: keepalive_max must be greater than 0 when keepalive is enabled (set keepalive_interval_s to 0 to disable it)"
    )]
    ZeroKeepaliveMax,
    /// The standby idle delay is zero, which would put a tunnel back in
    /// standby the moment it wakes.
    #[error("settings: standby_idle_s must be greater than 0")]
    ZeroStandbyIdle,
    /// The volume of the standby sounds is outside the allowed range.
    #[error(
        "settings: standby_volume ({volume}) must be between {} and {}",
        MIN_STANDBY_VOLUME,
        MAX_STANDBY_VOLUME
    )]
    StandbyVolumeOutOfRange {
        /// The configured volume, in percent.
        volume: u8,
    },
    /// A default bind address is empty.
    #[error("settings: {setting} must not be empty")]
    EmptyBindAddress {
        /// The setting name as written in the file.
        setting: &'static str,
    },
    /// A tag of `start_on_launch_tags` breaks the tag syntax.
    #[error("settings: start_on_launch_tags contains the invalid tag \"{tag}\": {problem}")]
    InvalidStartOnLaunchTag {
        /// The tag.
        tag: String,
        /// What is wrong with it.
        problem: TagProblem,
    },
    /// A launcher field is empty.
    #[error("x11_launchers entry {position}: {field} must not be empty")]
    EmptyLauncherField {
        /// The 1-based position of the launcher.
        position: usize,
        /// The field name as written in the file.
        field: &'static str,
    },
    /// Two launchers share a host and a name.
    #[error("x11_launchers: \"{name}\" is defined more than once for host {host}")]
    DuplicateLauncher {
        /// The host alias.
        host: String,
        /// The launcher name.
        name: String,
    },
}

/// Checks every rule and returns all violations, not just the first.
pub(crate) fn validate(document: &StoreDocument) -> Result<(), Vec<ValidationError>> {
    let errors: Vec<ValidationError> = validate_settings(&document.settings)
        .into_iter()
        .chain(validate_tunnels(document))
        .chain(validate_launchers(&document.x11_launchers))
        .chain(validate_openers(&document.openers))
        .collect();
    if errors.is_empty() {
        return Ok(());
    }
    Err(errors)
}

/// Checks the rules of a single tunnel, such as a temporary one that is not
/// part of any document.
pub(crate) fn validate_tunnel_definition(
    tunnel: &TunnelDefinition,
) -> Result<(), Vec<ValidationError>> {
    let errors = validate_tunnel(tunnel);
    if errors.is_empty() {
        return Ok(());
    }
    Err(errors)
}

fn validate_settings(settings: &Settings) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if settings.reconnect_initial_delay.is_zero() {
        errors.push(ValidationError::ZeroReconnectInitialDelay);
    }
    if settings.reconnect_initial_delay > settings.reconnect_max_delay {
        errors.push(ValidationError::ReconnectDelaysOutOfOrder {
            initial: settings.reconnect_initial_delay,
            max: settings.reconnect_max_delay,
        });
    }
    if !settings.keepalive_interval.is_zero() && settings.keepalive_max == 0 {
        errors.push(ValidationError::ZeroKeepaliveMax);
    }
    if settings.standby_idle.is_zero() {
        errors.push(ValidationError::ZeroStandbyIdle);
    }
    if !(MIN_STANDBY_VOLUME..=MAX_STANDBY_VOLUME).contains(&settings.standby_volume) {
        errors.push(ValidationError::StandbyVolumeOutOfRange {
            volume: settings.standby_volume,
        });
    }
    errors.extend(validate_bind_addresses(settings));
    errors.extend(settings.start_on_launch_tags.iter().filter_map(|tag| {
        find_tag_problem(tag).map(|problem| ValidationError::InvalidStartOnLaunchTag {
            tag: tag.clone(),
            problem,
        })
    }));
    errors
}

fn validate_bind_addresses(settings: &Settings) -> Vec<ValidationError> {
    [
        ("default_bind_address", &settings.default_bind_address),
        (
            "default_remote_bind_address",
            &settings.default_remote_bind_address,
        ),
    ]
    .into_iter()
    .filter(|(_, address)| address.trim().is_empty())
    .map(|(setting, _)| ValidationError::EmptyBindAddress { setting })
    .collect()
}

fn validate_tunnels(document: &StoreDocument) -> Vec<ValidationError> {
    let tunnels = &document.tunnels;
    tunnels
        .iter()
        .flat_map(validate_tunnel)
        .chain(find_duplicate_ids(tunnels))
        .chain(find_listen_conflicts(tunnels, &document.settings))
        .collect()
}

fn validate_tunnel(tunnel: &TunnelDefinition) -> Vec<ValidationError> {
    let reference = TunnelReference::of(tunnel);
    let mut errors = Vec::new();
    if tunnel.host.trim().is_empty() {
        errors.push(ValidationError::EmptyHost {
            tunnel: reference.clone(),
        });
    }
    errors.extend(validate_optional_fields(tunnel, &reference));
    if tunnel.listen_port < MIN_PORT {
        errors.push(ValidationError::ZeroListenPort {
            tunnel: reference.clone(),
        });
    }
    if tunnel.target_port.is_some_and(|port| port < MIN_PORT) {
        errors.push(ValidationError::ZeroTargetPort {
            tunnel: reference.clone(),
        });
    }
    errors.extend(validate_dynamic_target(tunnel, &reference));
    errors.extend(validate_tags(tunnel, &reference));
    errors.extend(validate_open_path(tunnel, &reference));
    errors
}

fn validate_open_path(
    tunnel: &TunnelDefinition,
    reference: &TunnelReference,
) -> Option<ValidationError> {
    let path = tunnel.open.path.as_ref()?;
    let problem = find_open_path_problem(path)?;
    Some(ValidationError::InvalidOpenPath {
        tunnel: reference.clone(),
        path: path.clone(),
        problem,
    })
}

pub(crate) fn find_open_path_problem(path: &str) -> Option<OpenPathProblem> {
    if !path.starts_with('/') {
        return Some(OpenPathProblem::MissingLeadingSlash);
    }
    if path.starts_with("//") {
        return Some(OpenPathProblem::StartsWithDoubleSlash);
    }
    if path.contains(char::is_whitespace) {
        return Some(OpenPathProblem::ContainsWhitespace);
    }
    if path.contains(char::is_control) {
        return Some(OpenPathProblem::ContainsControlCharacter);
    }
    None
}

fn validate_dynamic_target(
    tunnel: &TunnelDefinition,
    reference: &TunnelReference,
) -> Vec<ValidationError> {
    if tunnel.kind != TunnelKind::Dynamic {
        return Vec::new();
    }
    [
        ("target_host", tunnel.target_host.is_some()),
        ("target_port", tunnel.target_port.is_some()),
    ]
    .into_iter()
    .filter(|(_, is_set)| *is_set)
    .map(|(field, _)| ValidationError::DynamicTunnelWithTarget {
        tunnel: reference.clone(),
        field,
    })
    .collect()
}

fn validate_optional_fields(
    tunnel: &TunnelDefinition,
    reference: &TunnelReference,
) -> Vec<ValidationError> {
    [
        ("listen_address", &tunnel.listen_address),
        ("target_host", &tunnel.target_host),
    ]
    .into_iter()
    .filter(|(_, value)| value.as_deref().is_some_and(|text| text.trim().is_empty()))
    .map(|(field, _)| ValidationError::EmptyField {
        tunnel: reference.clone(),
        field,
    })
    .collect()
}

fn validate_tags(tunnel: &TunnelDefinition, reference: &TunnelReference) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for (position, tag) in tunnel.tags.iter().enumerate() {
        if let Some(problem) = find_tag_problem(tag) {
            errors.push(ValidationError::InvalidTag {
                tunnel: reference.clone(),
                tag: tag.clone(),
                problem,
            });
        } else if tunnel.tags[..position].contains(tag) {
            errors.push(ValidationError::DuplicateTag {
                tunnel: reference.clone(),
                tag: tag.clone(),
            });
        }
    }
    errors
}

pub(crate) fn find_tag_problem(tag: &str) -> Option<TagProblem> {
    if tag.is_empty() {
        return Some(TagProblem::Empty);
    }
    if tag.contains(char::is_whitespace) {
        return Some(TagProblem::ContainsWhitespace);
    }
    if tag.contains(',') {
        return Some(TagProblem::ContainsComma);
    }
    if tag.contains(':') {
        return Some(TagProblem::ContainsColon);
    }
    if tag.chars().all(|character| character.is_ascii_digit()) {
        return Some(TagProblem::OnlyDigits);
    }
    None
}

fn find_duplicate_ids(tunnels: &[TunnelDefinition]) -> Vec<ValidationError> {
    let mut seen = Vec::new();
    let mut errors = Vec::new();
    for tunnel in tunnels {
        if seen.contains(&tunnel.id) {
            errors.push(ValidationError::DuplicateId {
                tunnel: TunnelReference::of(tunnel),
            });
        }
        seen.push(tunnel.id);
    }
    errors
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct ListenEndpoint {
    remote_host: Option<String>,
    address: String,
    port: u16,
}

impl ListenEndpoint {
    fn of(tunnel: &TunnelDefinition, settings: &Settings) -> Self {
        let remote_host = match tunnel.kind {
            TunnelKind::Remote => Some(tunnel.host.clone()),
            TunnelKind::Local | TunnelKind::Dynamic => None,
        };
        Self {
            remote_host,
            address: tunnel
                .effective_listen_address(settings)
                .trim()
                .to_lowercase(),
            port: tunnel.listen_port,
        }
    }
}

fn find_listen_conflicts(
    tunnels: &[TunnelDefinition],
    settings: &Settings,
) -> Vec<ValidationError> {
    let mut first_by_endpoint: HashMap<ListenEndpoint, &TunnelDefinition> = HashMap::new();
    let mut errors = Vec::new();
    for tunnel in tunnels
        .iter()
        .filter(|tunnel| tunnel.listen_port >= MIN_PORT)
    {
        let endpoint = ListenEndpoint::of(tunnel, settings);
        match first_by_endpoint.get(&endpoint) {
            Some(first) => errors.push(listen_conflict(tunnel, first, endpoint)),
            None => {
                first_by_endpoint.insert(endpoint, tunnel);
            }
        }
    }
    errors
}

fn listen_conflict(
    tunnel: &TunnelDefinition,
    first: &TunnelDefinition,
    endpoint: ListenEndpoint,
) -> ValidationError {
    let tunnel = TunnelReference::of(tunnel);
    let other = TunnelReference::of(first);
    match endpoint.remote_host {
        Some(host) => ValidationError::RemoteListenConflict {
            tunnel,
            other,
            host,
            listen_address: endpoint.address,
            listen_port: endpoint.port,
        },
        None => ValidationError::LocalListenConflict {
            tunnel,
            other,
            listen_address: endpoint.address,
            listen_port: endpoint.port,
        },
    }
}

fn validate_launchers(launchers: &[X11Launcher]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for (index, launcher) in launchers.iter().enumerate() {
        let position = index + 1;
        for (field, value) in [
            ("host", &launcher.host),
            ("name", &launcher.name),
            ("command", &launcher.command),
        ] {
            if value.trim().is_empty() {
                errors.push(ValidationError::EmptyLauncherField { position, field });
            }
        }
        let is_repeated = launchers[..index]
            .iter()
            .any(|earlier| earlier.host == launcher.host && earlier.name == launcher.name);
        if is_repeated {
            errors.push(ValidationError::DuplicateLauncher {
                host: launcher.host.clone(),
                name: launcher.name.clone(),
            });
        }
    }
    errors
}

const OPENER_PLACEHOLDERS: [&str; 3] = ["{host}", "{port}", "{url}"];

/// Checks the rules of a list of openers.
pub(crate) fn validate_openers(openers: &[Opener]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for (index, opener) in openers.iter().enumerate() {
        let position = index + 1;
        if let Some(problem) = find_opener_id_problem(&opener.id) {
            errors.push(ValidationError::InvalidOpenerId {
                position,
                id: opener.id.clone(),
                problem,
            });
        } else if openers[..index]
            .iter()
            .any(|earlier| earlier.id == opener.id)
        {
            errors.push(ValidationError::DuplicateOpenerId {
                id: opener.id.clone(),
            });
        }
        for (field, value) in [("name", &opener.name), ("command", &opener.command)] {
            if value.trim().is_empty() {
                errors.push(ValidationError::EmptyOpenerField { position, field });
            }
        }
        errors.extend(opener.args.iter().filter_map(|argument| {
            find_unknown_placeholder(argument).map(|placeholder| {
                ValidationError::UnknownOpenerPlaceholder {
                    position,
                    argument: argument.clone(),
                    placeholder,
                }
            })
        }));
    }
    errors
}

fn find_opener_id_problem(id: &str) -> Option<OpenerIdProblem> {
    if id.is_empty() {
        return Some(OpenerIdProblem::Empty);
    }
    let is_slug = id.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    });
    if !is_slug {
        return Some(OpenerIdProblem::InvalidCharacter);
    }
    (id == BROWSER_OPENER_ID).then_some(OpenerIdProblem::Reserved)
}

fn find_unknown_placeholder(argument: &str) -> Option<String> {
    let mut rest = argument;
    while let Some(start) = rest.find('{') {
        let after_start = &rest[start..];
        let end = after_start.find('}')?;
        let placeholder = &after_start[..=end];
        if !OPENER_PLACEHOLDERS.contains(&placeholder) {
            return Some(placeholder.to_owned());
        }
        rest = &after_start[end + 1..];
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::test_support::{tunnel, tunnel_on};
    use pretty_assertions::assert_eq;

    fn document_with(tunnels: Vec<TunnelDefinition>) -> StoreDocument {
        StoreDocument {
            tunnels,
            ..StoreDocument::default()
        }
    }

    fn document_with_settings(settings: Settings) -> StoreDocument {
        StoreDocument {
            settings,
            ..StoreDocument::default()
        }
    }

    fn errors_of(document: &StoreDocument) -> Vec<ValidationError> {
        validate(document).err().unwrap_or_default()
    }

    #[test]
    fn should_accept_document_when_every_rule_holds() {
        let document = document_with(vec![
            tunnel_on(TunnelKind::Local, 3333),
            tunnel_on(TunnelKind::Remote, 3333),
        ]);

        assert_eq!(validate(&document), Ok(()));
    }

    #[test]
    fn should_reject_duplicate_ids() {
        let first = tunnel(TunnelKind::Local);
        let second = TunnelDefinition {
            listen_port: 4444,
            ..first.clone()
        };

        let errors = errors_of(&document_with(vec![first, second]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::DuplicateId { .. }]
        ));
    }

    #[test]
    fn should_reject_empty_host() {
        let document = document_with(vec![TunnelDefinition {
            host: String::new(),
            ..tunnel(TunnelKind::Local)
        }]);

        let errors = errors_of(&document);

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::EmptyHost { .. }]
        ));
    }

    #[test]
    fn should_reject_empty_listen_address_when_it_is_set() {
        let document = document_with(vec![TunnelDefinition {
            listen_address: Some(String::new()),
            ..tunnel(TunnelKind::Local)
        }]);

        let errors = errors_of(&document);

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::EmptyField {
                field: "listen_address",
                ..
            }]
        ));
    }

    #[test]
    fn should_reject_zero_listen_port() {
        let document = document_with(vec![tunnel_on(TunnelKind::Local, 0)]);

        let errors = errors_of(&document);

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::ZeroListenPort { .. }]
        ));
    }

    #[test]
    fn should_reject_zero_target_port() {
        let document = document_with(vec![TunnelDefinition {
            target_port: Some(0),
            ..tunnel(TunnelKind::Local)
        }]);

        let errors = errors_of(&document);

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::ZeroTargetPort { .. }]
        ));
    }

    #[test]
    fn should_reject_dynamic_tunnel_when_it_has_a_target_host() {
        let dynamic = TunnelDefinition {
            target_host: Some("db.internal".to_owned()),
            ..tunnel_on(TunnelKind::Dynamic, 1080)
        };

        let errors = errors_of(&document_with(vec![dynamic]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::DynamicTunnelWithTarget {
                field: "target_host",
                ..
            }]
        ));
    }

    #[test]
    fn should_reject_dynamic_tunnel_when_it_has_a_target_port() {
        let dynamic = TunnelDefinition {
            target_port: Some(5432),
            ..tunnel_on(TunnelKind::Dynamic, 1080)
        };

        let errors = errors_of(&document_with(vec![dynamic]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::DynamicTunnelWithTarget {
                field: "target_port",
                ..
            }]
        ));
    }

    #[test]
    fn should_accept_dynamic_tunnel_when_it_has_no_target() {
        let dynamic = tunnel_on(TunnelKind::Dynamic, 1080);

        assert_eq!(validate(&document_with(vec![dynamic])), Ok(()));
    }

    #[test]
    fn should_accept_remote_tunnel_when_it_has_a_target() {
        let remote = TunnelDefinition {
            target_host: Some("localhost".to_owned()),
            target_port: Some(3000),
            ..tunnel_on(TunnelKind::Remote, 8080)
        };

        assert_eq!(validate(&document_with(vec![remote])), Ok(()));
    }

    #[test]
    fn should_accept_remote_and_dynamic_tunnels_when_the_remote_port_is_the_local_listen_port() {
        let remote = tunnel_on(TunnelKind::Remote, 1080);
        let dynamic = tunnel_on(TunnelKind::Dynamic, 1080);

        assert_eq!(validate(&document_with(vec![remote, dynamic])), Ok(()));
    }

    #[test]
    fn should_reject_local_tunnels_when_they_share_the_effective_listen_endpoint() {
        let explicit = TunnelDefinition {
            listen_address: Some("127.0.0.1".to_owned()),
            ..tunnel_on(TunnelKind::Local, 3333)
        };
        let implicit = tunnel_on(TunnelKind::Dynamic, 3333);

        let errors = errors_of(&document_with(vec![explicit, implicit]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::LocalListenConflict {
                listen_port: 3333,
                ..
            }]
        ));
    }

    #[test]
    fn should_accept_local_tunnels_when_listen_addresses_differ() {
        let loopback = tunnel_on(TunnelKind::Local, 3333);
        let any = TunnelDefinition {
            listen_address: Some("0.0.0.0".to_owned()),
            ..tunnel_on(TunnelKind::Local, 3333)
        };

        assert_eq!(validate(&document_with(vec![loopback, any])), Ok(()));
    }

    #[test]
    fn should_reject_remote_tunnels_when_they_share_endpoint_on_the_same_host() {
        let first = tunnel_on(TunnelKind::Remote, 8080);
        let second = tunnel_on(TunnelKind::Remote, 8080);

        let errors = errors_of(&document_with(vec![first, second]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::RemoteListenConflict { .. }]
        ));
    }

    #[test]
    fn should_accept_remote_tunnels_when_they_go_through_different_hosts() {
        let first = tunnel_on(TunnelKind::Remote, 8080);
        let second = TunnelDefinition {
            host: "other".to_owned(),
            ..tunnel_on(TunnelKind::Remote, 8080)
        };

        assert_eq!(validate(&document_with(vec![first, second])), Ok(()));
    }

    #[test]
    fn should_accept_local_and_remote_tunnels_when_they_share_a_port() {
        let local = tunnel_on(TunnelKind::Local, 8080);
        let remote = tunnel_on(TunnelKind::Remote, 8080);

        assert_eq!(validate(&document_with(vec![local, remote])), Ok(()));
    }

    fn open_path_problem_of(path: &str) -> Option<OpenPathProblem> {
        let definition = TunnelDefinition {
            open: crate::store::OpenOptions {
                path: Some(path.to_owned()),
                ..crate::store::OpenOptions::default()
            },
            ..tunnel(TunnelKind::Local)
        };
        match errors_of(&document_with(vec![definition])).as_slice() {
            [ValidationError::InvalidOpenPath { problem, .. }] => Some(*problem),
            _ => None,
        }
    }

    #[test]
    fn should_accept_an_open_path_that_starts_with_a_slash() {
        assert_eq!(open_path_problem_of("/admin?tab=1#top"), None);
    }

    #[test]
    fn should_accept_the_root_open_path() {
        assert_eq!(open_path_problem_of("/"), None);
    }

    #[test]
    fn should_reject_an_open_path_without_a_leading_slash() {
        assert_eq!(
            open_path_problem_of("admin"),
            Some(OpenPathProblem::MissingLeadingSlash)
        );
    }

    #[test]
    fn should_reject_an_empty_open_path() {
        assert_eq!(
            open_path_problem_of(""),
            Some(OpenPathProblem::MissingLeadingSlash)
        );
    }

    #[test]
    fn should_reject_an_open_path_starting_with_two_slashes() {
        assert_eq!(
            open_path_problem_of("//evil.example"),
            Some(OpenPathProblem::StartsWithDoubleSlash)
        );
    }

    #[test]
    fn should_reject_an_open_path_with_whitespace() {
        assert_eq!(
            open_path_problem_of("/my page"),
            Some(OpenPathProblem::ContainsWhitespace)
        );
    }

    #[test]
    fn should_reject_an_open_path_with_a_control_character() {
        assert_eq!(
            open_path_problem_of("/a\u{7}b"),
            Some(OpenPathProblem::ContainsControlCharacter)
        );
    }

    #[test]
    fn should_not_check_the_open_path_when_it_is_absent() {
        assert_eq!(
            validate(&document_with(vec![tunnel(TunnelKind::Local)])),
            Ok(())
        );
    }

    fn opener(id: &str) -> Opener {
        Opener {
            id: id.to_owned(),
            name: "Viewer".to_owned(),
            command: "viewer".to_owned(),
            args: vec!["{host}:{port}".to_owned(), "{url}".to_owned()],
            ports: vec![5900],
        }
    }

    fn opener_errors(openers: &[Opener]) -> Vec<ValidationError> {
        validate_openers(openers)
    }

    #[test]
    fn should_accept_a_valid_opener() {
        assert_eq!(opener_errors(&[opener("my-viewer2")]), vec![]);
    }

    #[test]
    fn should_reject_an_empty_opener_id() {
        assert!(matches!(
            opener_errors(&[opener("")]).as_slice(),
            [ValidationError::InvalidOpenerId {
                problem: OpenerIdProblem::Empty,
                ..
            }]
        ));
    }

    #[test]
    fn should_reject_an_opener_id_with_uppercase_letters() {
        assert!(matches!(
            opener_errors(&[opener("Viewer")]).as_slice(),
            [ValidationError::InvalidOpenerId {
                problem: OpenerIdProblem::InvalidCharacter,
                ..
            }]
        ));
    }

    #[test]
    fn should_reject_the_reserved_browser_id() {
        assert!(matches!(
            opener_errors(&[opener("browser")]).as_slice(),
            [ValidationError::InvalidOpenerId {
                problem: OpenerIdProblem::Reserved,
                ..
            }]
        ));
    }

    #[test]
    fn should_reject_a_repeated_opener_id() {
        assert_eq!(
            opener_errors(&[opener("a"), opener("a")]),
            vec![ValidationError::DuplicateOpenerId { id: "a".to_owned() }]
        );
    }

    #[test]
    fn should_reject_an_opener_without_a_command() {
        let blank = Opener {
            command: " ".to_owned(),
            ..opener("a")
        };

        assert_eq!(
            opener_errors(&[blank]),
            vec![ValidationError::EmptyOpenerField {
                position: 1,
                field: "command"
            }]
        );
    }

    #[test]
    fn should_reject_an_opener_without_a_name() {
        let blank = Opener {
            name: String::new(),
            ..opener("a")
        };

        assert_eq!(
            opener_errors(&[blank]),
            vec![ValidationError::EmptyOpenerField {
                position: 1,
                field: "name"
            }]
        );
    }

    #[test]
    fn should_reject_an_unknown_placeholder_in_an_argument() {
        let unknown = Opener {
            args: vec!["--db={database}".to_owned()],
            ..opener("a")
        };

        assert!(matches!(
            opener_errors(&[unknown]).as_slice(),
            [ValidationError::UnknownOpenerPlaceholder { placeholder, .. }] if placeholder == "{database}"
        ));
    }

    #[test]
    fn should_accept_a_lone_brace_in_an_argument() {
        let literal = Opener {
            args: vec!["a{b".to_owned()],
            ..opener("a")
        };

        assert_eq!(opener_errors(&[literal]), vec![]);
    }

    fn tunnel_with_tags(tags: &[&str]) -> TunnelDefinition {
        TunnelDefinition {
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            ..tunnel(TunnelKind::Local)
        }
    }

    #[test]
    fn should_reject_empty_tag() {
        let errors = errors_of(&document_with(vec![tunnel_with_tags(&[""])]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::InvalidTag {
                problem: TagProblem::Empty,
                ..
            }]
        ));
    }

    #[test]
    fn should_reject_tag_with_whitespace() {
        let errors = errors_of(&document_with(vec![tunnel_with_tags(&["my tag"])]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::InvalidTag {
                problem: TagProblem::ContainsWhitespace,
                ..
            }]
        ));
    }

    #[test]
    fn should_reject_tag_with_comma() {
        let errors = errors_of(&document_with(vec![tunnel_with_tags(&["a,b"])]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::InvalidTag {
                problem: TagProblem::ContainsComma,
                ..
            }]
        ));
    }

    #[test]
    fn should_reject_tag_with_colon() {
        let errors = errors_of(&document_with(vec![tunnel_with_tags(&["nova:3333"])]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::InvalidTag {
                problem: TagProblem::ContainsColon,
                ..
            }]
        ));
    }

    #[test]
    fn should_reject_tag_made_only_of_digits() {
        let errors = errors_of(&document_with(vec![tunnel_with_tags(&["3333"])]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::InvalidTag {
                problem: TagProblem::OnlyDigits,
                ..
            }]
        ));
    }

    #[test]
    fn should_accept_tag_mixing_digits_and_letters() {
        assert_eq!(
            validate(&document_with(vec![tunnel_with_tags(&["v2"])])),
            Ok(())
        );
    }

    #[test]
    fn should_reject_tag_listed_twice_in_a_tunnel() {
        let errors = errors_of(&document_with(vec![tunnel_with_tags(&["dev", "dev"])]));

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::DuplicateTag { .. }]
        ));
    }

    #[test]
    fn should_reject_initial_reconnect_delay_above_the_maximum() {
        let document = document_with_settings(Settings {
            reconnect_initial_delay: Duration::from_secs(10),
            reconnect_max_delay: Duration::from_secs(5),
            ..Settings::default()
        });

        let errors = errors_of(&document);

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::ReconnectDelaysOutOfOrder { .. }]
        ));
    }

    #[test]
    fn should_accept_equal_reconnect_delays() {
        let document = document_with_settings(Settings {
            reconnect_initial_delay: Duration::from_secs(5),
            reconnect_max_delay: Duration::from_secs(5),
            ..Settings::default()
        });

        assert_eq!(validate(&document), Ok(()));
    }

    #[test]
    fn should_reject_zero_initial_reconnect_delay() {
        let document = document_with_settings(Settings {
            reconnect_initial_delay: Duration::ZERO,
            ..Settings::default()
        });

        let errors = errors_of(&document);

        assert!(matches!(
            errors.as_slice(),
            [ValidationError::ZeroReconnectInitialDelay]
        ));
    }

    #[test]
    fn should_reject_zero_keepalive_max_when_keepalive_is_enabled() {
        let document = document_with_settings(Settings {
            keepalive_max: 0,
            ..Settings::default()
        });

        assert_eq!(
            errors_of(&document),
            vec![ValidationError::ZeroKeepaliveMax]
        );
    }

    #[test]
    fn should_accept_zero_keepalive_interval_when_keepalive_is_disabled() {
        let document = document_with_settings(Settings {
            keepalive_interval: Duration::ZERO,
            keepalive_max: 0,
            ..Settings::default()
        });

        assert_eq!(validate(&document), Ok(()));
    }

    #[test]
    fn should_reject_zero_standby_idle() {
        let document = document_with_settings(Settings {
            standby_idle: Duration::ZERO,
            ..Settings::default()
        });

        assert_eq!(errors_of(&document), vec![ValidationError::ZeroStandbyIdle]);
    }

    #[test]
    fn should_reject_a_standby_volume_outside_the_range() {
        for volume in [0, 4, 101] {
            let document = document_with_settings(Settings {
                standby_volume: volume,
                ..Settings::default()
            });

            assert_eq!(
                errors_of(&document),
                vec![ValidationError::StandbyVolumeOutOfRange { volume }]
            );
        }
    }

    #[test]
    fn should_accept_the_bounds_of_the_standby_volume() {
        for volume in [5, 100] {
            let document = document_with_settings(Settings {
                standby_volume: volume,
                ..Settings::default()
            });

            assert_eq!(validate(&document), Ok(()));
        }
    }

    #[test]
    fn should_reject_empty_default_bind_address() {
        let document = document_with_settings(Settings {
            default_bind_address: String::new(),
            ..Settings::default()
        });

        assert_eq!(
            errors_of(&document),
            vec![ValidationError::EmptyBindAddress {
                setting: "default_bind_address"
            }]
        );
    }

    #[test]
    fn should_reject_invalid_start_on_launch_tag() {
        let document = document_with_settings(Settings {
            start_on_launch_tags: vec!["a b".to_owned()],
            ..Settings::default()
        });

        assert!(matches!(
            errors_of(&document).as_slice(),
            [ValidationError::InvalidStartOnLaunchTag { .. }]
        ));
    }

    #[test]
    fn should_reject_launcher_with_empty_command() {
        let document = StoreDocument {
            x11_launchers: vec![X11Launcher {
                host: "nova".to_owned(),
                name: "Editor".to_owned(),
                command: String::new(),
            }],
            ..StoreDocument::default()
        };

        assert_eq!(
            errors_of(&document),
            vec![ValidationError::EmptyLauncherField {
                position: 1,
                field: "command"
            }]
        );
    }

    #[test]
    fn should_reject_launchers_sharing_host_and_name() {
        let launcher = X11Launcher {
            host: "nova".to_owned(),
            name: "Editor".to_owned(),
            command: "code".to_owned(),
        };
        let document = StoreDocument {
            x11_launchers: vec![launcher.clone(), launcher],
            ..StoreDocument::default()
        };

        assert!(matches!(
            errors_of(&document).as_slice(),
            [ValidationError::DuplicateLauncher { .. }]
        ));
    }

    #[test]
    fn should_collect_every_error_when_several_rules_are_broken() {
        let document = document_with(vec![
            tunnel_on(TunnelKind::Local, 0),
            TunnelDefinition {
                host: String::new(),
                ..tunnel_on(TunnelKind::Local, 3333)
            },
        ]);

        assert_eq!(errors_of(&document).len(), 2);
    }

    #[test]
    fn should_name_the_tunnel_label_and_id_when_formatting_an_error() {
        let definition = tunnel_on(TunnelKind::Local, 0);
        let message = errors_of(&document_with(vec![definition.clone()]))[0].to_string();

        assert_eq!(
            message,
            format!(
                "tunnel \"0 → nova:0\" ({}): listen_port must be between 1 and 65535, found 0",
                definition.id
            )
        );
    }
}
