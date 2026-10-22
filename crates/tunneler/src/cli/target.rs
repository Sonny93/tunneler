//! Turns the ports, `host:port` pairs and tags typed on the command line into
//! tunnels.
//!
//! Tunnels have no name: a number selects the tunnel listening on that port,
//! `host:port` the tunnel of that host listening on that port, and anything
//! else is a tag. A tag can neither be only digits nor contain a colon, so the
//! three forms never collide.

use ipc::protocol::{SnapshotDto, TunnelDefinitionDto};
use thiserror::Error;

use super::render::{describe_tunnel_route, kind_word};
use super::style::Style;

const ENDPOINT_SEPARATOR: char = ':';
const EXAMPLE_ENDPOINT: &str = "nova:3333";

/// How the arguments of `up`, `down` and `open` are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetInterpretation {
    /// An argument is a port, a `host:port` pair or a tag.
    PortEndpointOrTag,
    /// Every argument is a tag.
    TagOnly,
}

/// A tunnel picked by the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTunnel {
    /// The identifier the daemon knows it by.
    pub id: String,
    /// Its route, such as `3333 → nova:3333`, for the messages.
    pub label: String,
}

/// Why an argument does not designate the tunnels it should.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ResolveError {
    /// The argument looks like a port or a `host:port` pair but is not one.
    #[error("\"{argument}\" is not a valid port (1-65535) or host:port")]
    InvalidSelector {
        /// The argument.
        argument: String,
    },
    /// No tunnel listens on the port.
    #[error("no tunnel listens on port {port}")]
    UnknownPort {
        /// The port.
        port: u16,
    },
    /// No tunnel of the host listens on the port.
    #[error("no tunnel of host \"{host}\" listens on port {port}")]
    UnknownEndpoint {
        /// The host alias.
        host: String,
        /// The port.
        port: u16,
    },
    /// No tunnel carries the tag.
    #[error("no tunnel carries the tag \"{tag}\"")]
    UnknownTag {
        /// The argument.
        tag: String,
    },
    /// Several tunnels of different hosts listen on the port.
    #[error("{} tunnels listen on port {port}: {}", .candidates.len(), .candidates.join(", "))]
    AmbiguousPort {
        /// The port.
        port: u16,
        /// The tunnels, as `host:port`.
        candidates: Vec<String>,
    },
    /// Several tunnels of the same host listen on the same port.
    #[error("{} tunnels listen on {endpoint}: {}", .candidates.len(), .candidates.join(", "))]
    AmbiguousEndpoint {
        /// The `host:port` pair.
        endpoint: String,
        /// The tunnels, as `host:port (kind)`.
        candidates: Vec<String>,
    },
    /// A command that needs one tunnel got a tag carried by several.
    #[error("the tag \"{tag}\" designates {} tunnels: {}", .candidates.len(), .candidates.join(", "))]
    AmbiguousTag {
        /// The tag.
        tag: String,
        /// The tunnels, as `host:port`.
        candidates: Vec<String>,
    },
}

impl ResolveError {
    /// What to do about it.
    pub fn hint(&self) -> String {
        match self {
            Self::InvalidSelector { .. }
            | Self::UnknownPort { .. }
            | Self::UnknownEndpoint { .. }
            | Self::UnknownTag { .. } => {
                "run `tunneler list` to see the tunnels and their tags".to_owned()
            }
            Self::AmbiguousPort { candidates, .. } => format!(
                "pass host:port to pick one, such as `{}`",
                candidates.first().map_or(EXAMPLE_ENDPOINT, String::as_str)
            ),
            Self::AmbiguousEndpoint { .. } => {
                "give the tunnel you want a tag, then select it by that tag".to_owned()
            }
            Self::AmbiguousTag { .. } => "pass a port or host:port to pick one tunnel".to_owned(),
        }
    }
}

enum Selector {
    Port(u16),
    Endpoint { host: String, port: u16 },
    Tag(String),
}

fn parse_port(text: &str) -> Option<u16> {
    text.parse::<u16>().ok().filter(|port| *port != 0)
}

fn parse_selector(
    argument: &str,
    interpretation: TargetInterpretation,
) -> Result<Selector, ResolveError> {
    if interpretation == TargetInterpretation::TagOnly {
        return Ok(Selector::Tag(argument.to_owned()));
    }
    let invalid = || ResolveError::InvalidSelector {
        argument: argument.to_owned(),
    };
    let is_number =
        !argument.is_empty() && argument.chars().all(|character| character.is_ascii_digit());
    if is_number {
        return parse_port(argument).map(Selector::Port).ok_or_else(invalid);
    }
    let Some((host, port)) = argument.rsplit_once(ENDPOINT_SEPARATOR) else {
        return Ok(Selector::Tag(argument.to_owned()));
    };
    match (host.is_empty(), parse_port(port)) {
        (false, Some(port)) => Ok(Selector::Endpoint {
            host: host.to_owned(),
            port,
        }),
        _ => Err(invalid()),
    }
}

fn is_same_text(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
}

fn endpoint_of(definition: &TunnelDefinitionDto) -> String {
    format!("{}:{}", definition.spec.host, definition.spec.listen_port)
}

fn describe_candidates(candidates: &[&TunnelDefinitionDto]) -> Vec<String> {
    let endpoints: Vec<String> = candidates
        .iter()
        .map(|candidate| endpoint_of(candidate))
        .collect();
    candidates
        .iter()
        .zip(&endpoints)
        .map(|(candidate, endpoint)| {
            let is_shared = endpoints.iter().filter(|other| *other == endpoint).count() > 1;
            if is_shared {
                format!("{endpoint} ({})", kind_word(candidate.spec.kind))
            } else {
                endpoint.clone()
            }
        })
        .collect()
}

fn count_hosts(definitions: &[&TunnelDefinitionDto]) -> usize {
    let mut hosts: Vec<String> = definitions
        .iter()
        .map(|definition| definition.spec.host.to_lowercase())
        .collect();
    hosts.sort();
    hosts.dedup();
    hosts.len()
}

fn on_port<'a>(
    port: u16,
    definitions: &[&'a TunnelDefinitionDto],
) -> Result<Vec<&'a TunnelDefinitionDto>, ResolveError> {
    let matching: Vec<&TunnelDefinitionDto> = definitions
        .iter()
        .copied()
        .filter(|definition| definition.spec.listen_port == port)
        .collect();
    match matching.as_slice() {
        [] => Err(ResolveError::UnknownPort { port }),
        [_] => Ok(matching),
        [first, ..] if count_hosts(&matching) == 1 => Err(ResolveError::AmbiguousEndpoint {
            endpoint: endpoint_of(first),
            candidates: describe_candidates(&matching),
        }),
        _ => Err(ResolveError::AmbiguousPort {
            port,
            candidates: describe_candidates(&matching),
        }),
    }
}

fn on_endpoint<'a>(
    host: &str,
    port: u16,
    definitions: &[&'a TunnelDefinitionDto],
) -> Result<Vec<&'a TunnelDefinitionDto>, ResolveError> {
    let matching: Vec<&TunnelDefinitionDto> = definitions
        .iter()
        .copied()
        .filter(|definition| {
            definition.spec.listen_port == port && is_same_text(&definition.spec.host, host)
        })
        .collect();
    match matching.as_slice() {
        [] => Err(ResolveError::UnknownEndpoint {
            host: host.to_owned(),
            port,
        }),
        [_] => Ok(matching),
        _ => Err(ResolveError::AmbiguousEndpoint {
            endpoint: format!("{host}{ENDPOINT_SEPARATOR}{port}"),
            candidates: describe_candidates(&matching),
        }),
    }
}

fn tagged<'a>(
    tag: &str,
    definitions: &[&'a TunnelDefinitionDto],
) -> Result<Vec<&'a TunnelDefinitionDto>, ResolveError> {
    let carrying: Vec<&TunnelDefinitionDto> = definitions
        .iter()
        .copied()
        .filter(|definition| {
            definition
                .spec
                .tags
                .iter()
                .any(|other| is_same_text(other, tag))
        })
        .collect();
    if carrying.is_empty() {
        return Err(ResolveError::UnknownTag {
            tag: tag.to_owned(),
        });
    }
    Ok(carrying)
}

fn resolve_argument<'a>(
    argument: &str,
    interpretation: TargetInterpretation,
    definitions: &[&'a TunnelDefinitionDto],
) -> Result<Vec<&'a TunnelDefinitionDto>, ResolveError> {
    match parse_selector(argument, interpretation)? {
        Selector::Port(port) => on_port(port, definitions),
        Selector::Endpoint { host, port } => on_endpoint(&host, port, definitions),
        Selector::Tag(tag) => tagged(&tag, definitions),
    }
}

fn definitions_of(snapshot: &SnapshotDto) -> Vec<&TunnelDefinitionDto> {
    snapshot
        .tunnels
        .iter()
        .map(|tunnel| &tunnel.definition)
        .collect()
}

fn resolved_tunnel(
    definition: &TunnelDefinitionDto,
    snapshot: &SnapshotDto,
    style: Style,
) -> ResolvedTunnel {
    let label = snapshot
        .tunnels
        .iter()
        .find(|tunnel| tunnel.definition.id == definition.id)
        .map_or_else(
            || definition.id.clone(),
            |tunnel| describe_tunnel_route(tunnel, &snapshot.settings, style),
        );
    ResolvedTunnel {
        id: definition.id.clone(),
        label,
    }
}

/// The tunnels the arguments designate, without duplicates, in the order they
/// are first designated.
///
/// # Errors
///
/// Fails on the first argument that matches nothing or is ambiguous.
pub fn resolve_targets(
    arguments: &[String],
    interpretation: TargetInterpretation,
    snapshot: &SnapshotDto,
    style: Style,
) -> Result<Vec<ResolvedTunnel>, ResolveError> {
    let definitions = definitions_of(snapshot);
    let mut resolved: Vec<ResolvedTunnel> = Vec::new();
    for argument in arguments {
        for tunnel in resolve_argument(argument, interpretation, &definitions)? {
            if resolved.iter().all(|known| known.id != tunnel.id) {
                resolved.push(resolved_tunnel(tunnel, snapshot, style));
            }
        }
    }
    Ok(resolved)
}

/// The one tunnel an argument designates.
///
/// # Errors
///
/// Fails when the argument matches nothing, or several tunnels.
pub fn find_tunnel<'a>(
    argument: &str,
    snapshot: &'a SnapshotDto,
) -> Result<&'a TunnelDefinitionDto, ResolveError> {
    let definitions = definitions_of(snapshot);
    let matching = resolve_argument(
        argument,
        TargetInterpretation::PortEndpointOrTag,
        &definitions,
    )?;
    match matching.as_slice() {
        [only] => Ok(only),
        _ => Err(ResolveError::AmbiguousTag {
            tag: argument.to_owned(),
            candidates: describe_candidates(&matching),
        }),
    }
}

/// Every configured tunnel.
pub fn all_tunnels(snapshot: &SnapshotDto, style: Style) -> Vec<ResolvedTunnel> {
    definitions_of(snapshot)
        .into_iter()
        .map(|definition| resolved_tunnel(definition, snapshot, style))
        .collect()
}

#[cfg(test)]
mod tests {
    use ipc::protocol::{
        SettingsDto, TrayStatusDto, TunnelDto, TunnelKindDto, TunnelSpecDto, TunnelStateDto,
    };
    use pretty_assertions::assert_eq;

    use super::*;

    const PLAIN: Style = Style {
        color: false,
        unicode: true,
    };

    fn definition(
        id: &str,
        host: &str,
        kind: TunnelKindDto,
        listen_port: u16,
        tags: &[&str],
    ) -> TunnelDefinitionDto {
        TunnelDefinitionDto {
            id: id.to_owned(),
            spec: TunnelSpecDto {
                host: host.to_owned(),
                kind,
                listen_port,
                listen_address: None,
                target_host: None,
                target_port: None,
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                open: ipc::protocol::OpenOptionsDto::default(),
                standby: None,
            },
        }
    }

    fn snapshot_of(definitions: Vec<TunnelDefinitionDto>) -> SnapshotDto {
        SnapshotDto {
            tunnels: definitions
                .into_iter()
                .map(|definition| TunnelDto {
                    definition,
                    state: TunnelStateDto::Stopped,
                    health: ipc::protocol::TargetHealthDto::Unknown,
                    metrics: ipc::protocol::MetricsDto::default(),
                    is_awake: false,
                    is_temporary: false,
                    default_opener: ipc::protocol::DefaultOpenerDto {
                        id: "browser".to_owned(),
                        name: "Browser".to_owned(),
                    },
                })
                .collect(),
            settings: SettingsDto {
                default_bind_address: "127.0.0.1".to_owned(),
                default_remote_bind_address: "localhost".to_owned(),
                keepalive_interval_ms: 0,
                keepalive_max: 3,
                session_idle_grace_ms: 0,
                reconnect_initial_delay_ms: 0,
                reconnect_max_delay_ms: 0,
                restore_on_launch: true,
                start_on_launch_tags: Vec::new(),
                launch_at_login: false,
                use_os_keyring: false,
                x11_display: None,
                open_host: ipc::protocol::OpenHostDto::Localhost,
                standby: false,
                standby_idle_s: 300,
                standby_notify: ipc::protocol::StandbyNotifyDto::Visual,
                standby_sound: ipc::protocol::StandbySoundDto::Pluck,
                language: ipc::protocol::LanguageDto::System,
                check_for_updates: true,
            },
            tray: TrayStatusDto::default(),
            externals: Vec::new(),
            x11_anchors: Vec::new(),
            x11_launchers: Vec::new(),
            openers: Vec::new(),
        }
    }

    fn catalog() -> SnapshotDto {
        snapshot_of(vec![
            definition("1", "nova", TunnelKindDto::Local, 3333, &["dev"]),
            definition("2", "nova", TunnelKindDto::Local, 5432, &["dev", "db"]),
            definition("3", "staging", TunnelKindDto::Local, 3333, &[]),
            definition("4", "staging", TunnelKindDto::Dynamic, 1080, &[]),
        ])
    }

    fn arguments(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    fn resolved_ids(
        texts: &[&str],
        interpretation: TargetInterpretation,
        snapshot: &SnapshotDto,
    ) -> Result<Vec<String>, ResolveError> {
        resolve_targets(&arguments(texts), interpretation, snapshot, PLAIN)
            .map(|tunnels| tunnels.into_iter().map(|tunnel| tunnel.id).collect())
    }

    fn resolved_in_catalog(texts: &[&str]) -> Result<Vec<String>, ResolveError> {
        resolved_ids(texts, TargetInterpretation::PortEndpointOrTag, &catalog())
    }

    #[test]
    fn should_resolve_the_tunnel_listening_on_a_port_when_the_argument_is_a_number() {
        assert_eq!(resolved_in_catalog(&["5432"]), Ok(vec!["2".to_owned()]));
    }

    #[test]
    fn should_report_every_candidate_as_host_and_port_when_several_tunnels_listen_on_the_port() {
        assert_eq!(
            resolved_in_catalog(&["3333"]),
            Err(ResolveError::AmbiguousPort {
                port: 3333,
                candidates: vec!["nova:3333".to_owned(), "staging:3333".to_owned()],
            })
        );
    }

    #[test]
    fn should_report_an_unknown_port_when_no_tunnel_listens_on_it() {
        assert_eq!(
            resolved_in_catalog(&["9999"]),
            Err(ResolveError::UnknownPort { port: 9999 })
        );
    }

    #[test]
    fn should_reject_a_port_out_of_range() {
        assert_eq!(
            resolved_in_catalog(&["70000"]),
            Err(ResolveError::InvalidSelector {
                argument: "70000".to_owned()
            })
        );
    }

    #[test]
    fn should_resolve_the_tunnel_of_the_host_when_the_argument_is_host_and_port() {
        assert_eq!(
            resolved_in_catalog(&["staging:3333"]),
            Ok(vec!["3".to_owned()])
        );
    }

    #[test]
    fn should_match_the_host_ignoring_case() {
        assert_eq!(
            resolved_in_catalog(&["NOVA:3333"]),
            Ok(vec!["1".to_owned()])
        );
    }

    #[test]
    fn should_report_an_unknown_endpoint_when_the_host_has_no_such_tunnel() {
        assert_eq!(
            resolved_in_catalog(&["nova:1080"]),
            Err(ResolveError::UnknownEndpoint {
                host: "nova".to_owned(),
                port: 1080
            })
        );
    }

    #[test]
    fn should_reject_a_host_and_port_pair_with_a_bad_port() {
        assert_eq!(
            resolved_in_catalog(&["nova:abc"]),
            Err(ResolveError::InvalidSelector {
                argument: "nova:abc".to_owned()
            })
        );
    }

    #[test]
    fn should_tell_the_kinds_apart_when_a_local_and_a_remote_tunnel_share_host_and_port() {
        let snapshot = snapshot_of(vec![
            definition("1", "nova", TunnelKindDto::Local, 8080, &[]),
            definition("2", "nova", TunnelKindDto::Remote, 8080, &[]),
        ]);

        assert_eq!(
            resolved_ids(
                &["nova:8080"],
                TargetInterpretation::PortEndpointOrTag,
                &snapshot
            ),
            Err(ResolveError::AmbiguousEndpoint {
                endpoint: "nova:8080".to_owned(),
                candidates: vec![
                    "nova:8080 (local)".to_owned(),
                    "nova:8080 (remote)".to_owned()
                ],
            })
        );
    }

    #[test]
    fn should_resolve_every_tunnel_of_a_tag_when_the_argument_is_a_tag() {
        assert_eq!(
            resolved_in_catalog(&["dev"]),
            Ok(vec!["1".to_owned(), "2".to_owned()])
        );
    }

    #[test]
    fn should_match_a_tag_ignoring_case() {
        assert_eq!(resolved_in_catalog(&["DB"]), Ok(vec!["2".to_owned()]));
    }

    #[test]
    fn should_report_an_unknown_tag_when_no_tunnel_carries_it() {
        assert_eq!(
            resolved_in_catalog(&["ghost"]),
            Err(ResolveError::UnknownTag {
                tag: "ghost".to_owned()
            })
        );
    }

    #[test]
    fn should_read_every_argument_as_a_tag_when_the_tag_flag_is_set() {
        assert_eq!(
            resolved_ids(&["5432"], TargetInterpretation::TagOnly, &catalog()),
            Err(ResolveError::UnknownTag {
                tag: "5432".to_owned()
            })
        );
    }

    #[test]
    fn should_list_a_tunnel_once_when_several_arguments_designate_it() {
        assert_eq!(
            resolved_in_catalog(&["5432", "dev"]),
            Ok(vec!["2".to_owned(), "1".to_owned()])
        );
    }

    #[test]
    fn should_label_the_resolved_tunnel_with_its_route() {
        let resolved = resolve_targets(
            &arguments(&["5432"]),
            TargetInterpretation::PortEndpointOrTag,
            &catalog(),
            PLAIN,
        )
        .unwrap();

        assert_eq!(resolved[0].label, "5432 → nova:5432");
    }

    #[test]
    fn should_suggest_host_and_port_when_several_tunnels_listen_on_the_port() {
        let error = ResolveError::AmbiguousPort {
            port: 3333,
            candidates: vec!["nova:3333".to_owned(), "staging:3333".to_owned()],
        };

        assert!(error.hint().contains("`nova:3333`"));
    }

    #[test]
    fn should_suggest_listing_the_tunnels_when_the_target_is_unknown() {
        let error = ResolveError::UnknownTag {
            tag: "ghost".to_owned(),
        };

        assert!(error.hint().contains("tunneler list"));
    }

    #[test]
    fn should_find_the_only_tunnel_listening_on_a_port() {
        assert_eq!(
            find_tunnel("5432", &catalog()).map(|tunnel| tunnel.id.as_str()),
            Ok("2")
        );
    }

    #[test]
    fn should_find_the_only_tunnel_carrying_a_tag() {
        assert_eq!(
            find_tunnel("db", &catalog()).map(|tunnel| tunnel.id.as_str()),
            Ok("2")
        );
    }

    #[test]
    fn should_refuse_to_find_one_tunnel_when_a_tag_designates_several() {
        assert_eq!(
            find_tunnel("dev", &catalog()),
            Err(ResolveError::AmbiguousTag {
                tag: "dev".to_owned(),
                candidates: vec!["nova:3333".to_owned(), "nova:5432".to_owned()],
            })
        );
    }

    #[test]
    fn should_list_every_tunnel_when_resolving_all() {
        assert_eq!(all_tunnels(&catalog(), PLAIN).len(), 4);
    }
}
