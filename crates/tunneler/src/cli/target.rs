//! Turns the names and tags typed on the command line into tunnels.

use ipc::protocol::TunnelDefinitionDto;
use thiserror::Error;

/// How the arguments of `up` and `down` are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetInterpretation {
    /// An argument is a tunnel name, or else a tag.
    NameOrTag,
    /// Every argument is a tag.
    TagOnly,
}

/// A tunnel picked by the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTunnel {
    /// The identifier the daemon knows it by.
    pub id: String,
    /// Its name, for the messages.
    pub name: String,
}

/// Why an argument does not designate any tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ResolveError {
    /// Neither a tunnel name nor a tag matches.
    #[error("no tunnel or tag is named \"{name}\"")]
    UnknownTarget {
        /// The argument.
        name: String,
    },
    /// No tunnel carries the tag.
    #[error("no tunnel carries the tag \"{tag}\"")]
    UnknownTag {
        /// The argument.
        tag: String,
    },
    /// The argument is a tunnel name and a different set of tunnels' tag.
    #[error("\"{name}\" is both a tunnel name and a tag")]
    AmbiguousTarget {
        /// The argument.
        name: String,
    },
}

impl ResolveError {
    /// What to do about it.
    pub fn hint(&self) -> String {
        match self {
            Self::UnknownTarget { .. } | Self::UnknownTag { .. } => {
                "run `tunneler list` to see the tunnels and their tags".to_owned()
            }
            Self::AmbiguousTarget { name } => {
                format!("pass --tag {name} to use the tag")
            }
        }
    }
}

fn is_same_text(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
}

/// The tunnel with this exact name, ignoring case.
pub fn find_tunnel_by_name<'a>(
    name: &str,
    definitions: &'a [TunnelDefinitionDto],
) -> Result<&'a TunnelDefinitionDto, ResolveError> {
    named(name, definitions).ok_or_else(|| ResolveError::UnknownTarget {
        name: name.to_owned(),
    })
}

fn named<'a>(
    argument: &str,
    definitions: &'a [TunnelDefinitionDto],
) -> Option<&'a TunnelDefinitionDto> {
    definitions
        .iter()
        .find(|definition| is_same_text(&definition.spec.name, argument))
}

fn tagged<'a>(
    argument: &str,
    definitions: &'a [TunnelDefinitionDto],
) -> Vec<&'a TunnelDefinitionDto> {
    definitions
        .iter()
        .filter(|definition| {
            definition
                .spec
                .tags
                .iter()
                .any(|tag| is_same_text(tag, argument))
        })
        .collect()
}

fn resolve_argument<'a>(
    argument: &str,
    interpretation: TargetInterpretation,
    definitions: &'a [TunnelDefinitionDto],
) -> Result<Vec<&'a TunnelDefinitionDto>, ResolveError> {
    let carrying_tag = tagged(argument, definitions);
    if interpretation == TargetInterpretation::TagOnly {
        return non_empty_or(carrying_tag, || ResolveError::UnknownTag {
            tag: argument.to_owned(),
        });
    }
    match named(argument, definitions) {
        Some(tunnel) if carrying_tag.iter().all(|other| other.id == tunnel.id) => Ok(vec![tunnel]),
        Some(_) => Err(ResolveError::AmbiguousTarget {
            name: argument.to_owned(),
        }),
        None => non_empty_or(carrying_tag, || ResolveError::UnknownTarget {
            name: argument.to_owned(),
        }),
    }
}

fn non_empty_or(
    tunnels: Vec<&TunnelDefinitionDto>,
    error: impl FnOnce() -> ResolveError,
) -> Result<Vec<&TunnelDefinitionDto>, ResolveError> {
    if tunnels.is_empty() {
        return Err(error());
    }
    Ok(tunnels)
}

/// The tunnels the arguments designate, without duplicates, in the order they
/// are first designated.
///
/// An exact tunnel name (ignoring case) wins over a tag. An argument that is
/// the name of a tunnel and also the tag of other tunnels is ambiguous: it
/// must be disambiguated with [`TargetInterpretation::TagOnly`].
///
/// # Errors
///
/// Fails on the first argument that matches nothing or is ambiguous.
pub fn resolve_targets(
    arguments: &[String],
    interpretation: TargetInterpretation,
    definitions: &[TunnelDefinitionDto],
) -> Result<Vec<ResolvedTunnel>, ResolveError> {
    let mut resolved: Vec<ResolvedTunnel> = Vec::new();
    for argument in arguments {
        for tunnel in resolve_argument(argument, interpretation, definitions)? {
            if resolved.iter().all(|known| known.id != tunnel.id) {
                resolved.push(resolved_tunnel(tunnel));
            }
        }
    }
    Ok(resolved)
}

/// Every configured tunnel.
pub fn all_tunnels(definitions: &[TunnelDefinitionDto]) -> Vec<ResolvedTunnel> {
    definitions.iter().map(resolved_tunnel).collect()
}

fn resolved_tunnel(definition: &TunnelDefinitionDto) -> ResolvedTunnel {
    ResolvedTunnel {
        id: definition.id.clone(),
        name: definition.spec.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use ipc::protocol::{TunnelKindDto, TunnelSpecDto};
    use pretty_assertions::assert_eq;

    use super::*;

    fn definition(id: &str, name: &str, tags: &[&str]) -> TunnelDefinitionDto {
        TunnelDefinitionDto {
            id: id.to_owned(),
            spec: TunnelSpecDto {
                name: name.to_owned(),
                host: "nova".to_owned(),
                kind: TunnelKindDto::Local,
                listen_port: 3333,
                listen_address: None,
                target_host: None,
                target_port: None,
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            },
        }
    }

    fn catalog() -> Vec<TunnelDefinitionDto> {
        vec![
            definition("1", "Adonis", &["dev"]),
            definition("2", "Mail", &["dev", "mail"]),
            definition("3", "Staging", &[]),
        ]
    }

    fn arguments(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    fn resolved_ids(
        texts: &[&str],
        interpretation: TargetInterpretation,
    ) -> Result<Vec<String>, ResolveError> {
        resolve_targets(&arguments(texts), interpretation, &catalog())
            .map(|tunnels| tunnels.into_iter().map(|tunnel| tunnel.id).collect())
    }

    #[test]
    fn should_resolve_a_tunnel_when_the_name_matches_exactly() {
        assert_eq!(
            resolved_ids(&["Adonis"], TargetInterpretation::NameOrTag),
            Ok(vec!["1".to_owned()])
        );
    }

    #[test]
    fn should_resolve_a_tunnel_when_the_name_differs_only_by_case() {
        assert_eq!(
            resolved_ids(&["sTaGiNg"], TargetInterpretation::NameOrTag),
            Ok(vec!["3".to_owned()])
        );
    }

    #[test]
    fn should_resolve_every_tunnel_of_a_tag_when_the_argument_is_a_tag() {
        assert_eq!(
            resolved_ids(&["dev"], TargetInterpretation::NameOrTag),
            Ok(vec!["1".to_owned(), "2".to_owned()])
        );
    }

    #[test]
    fn should_resolve_a_tag_when_the_tag_flag_is_set() {
        assert_eq!(
            resolved_ids(&["mail"], TargetInterpretation::TagOnly),
            Ok(vec!["2".to_owned()])
        );
    }

    #[test]
    fn should_not_resolve_a_name_when_the_tag_flag_is_set() {
        assert_eq!(
            resolved_ids(&["Adonis"], TargetInterpretation::TagOnly),
            Err(ResolveError::UnknownTag {
                tag: "Adonis".to_owned()
            })
        );
    }

    #[test]
    fn should_list_a_tunnel_once_when_several_arguments_designate_it() {
        assert_eq!(
            resolved_ids(&["Adonis", "dev"], TargetInterpretation::NameOrTag),
            Ok(vec!["1".to_owned(), "2".to_owned()])
        );
    }

    #[test]
    fn should_report_an_unknown_target_when_nothing_matches() {
        assert_eq!(
            resolved_ids(&["ghost"], TargetInterpretation::NameOrTag),
            Err(ResolveError::UnknownTarget {
                name: "ghost".to_owned()
            })
        );
    }

    #[test]
    fn should_report_an_ambiguous_target_when_a_name_is_also_a_tag_of_other_tunnels() {
        let mut definitions = catalog();
        definitions.push(definition("4", "dev", &[]));

        let outcome = resolve_targets(
            &arguments(&["dev"]),
            TargetInterpretation::NameOrTag,
            &definitions,
        );

        assert_eq!(
            outcome,
            Err(ResolveError::AmbiguousTarget {
                name: "dev".to_owned()
            })
        );
    }

    #[test]
    fn should_resolve_the_tag_when_the_ambiguous_argument_comes_with_the_tag_flag() {
        let mut definitions = catalog();
        definitions.push(definition("4", "dev", &[]));

        let outcome = resolve_targets(
            &arguments(&["dev"]),
            TargetInterpretation::TagOnly,
            &definitions,
        );

        assert_eq!(outcome.map(|tunnels| tunnels.len()), Ok(2));
    }

    #[test]
    fn should_resolve_the_tunnel_when_its_name_is_also_its_only_tag() {
        let definitions = vec![definition("1", "Adonis", &["adonis"])];

        let outcome = resolve_targets(
            &arguments(&["adonis"]),
            TargetInterpretation::NameOrTag,
            &definitions,
        );

        assert_eq!(outcome.map(|tunnels| tunnels.len()), Ok(1));
    }

    #[test]
    fn should_suggest_listing_the_tunnels_when_the_target_is_unknown() {
        let error = ResolveError::UnknownTarget {
            name: "ghost".to_owned(),
        };

        assert!(error.hint().contains("tunneler list"));
    }

    #[test]
    fn should_suggest_the_tag_flag_when_the_target_is_ambiguous() {
        let error = ResolveError::AmbiguousTarget {
            name: "dev".to_owned(),
        };

        assert!(error.hint().contains("--tag dev"));
    }

    #[test]
    fn should_find_a_tunnel_by_name_ignoring_case() {
        let definitions = catalog();

        assert_eq!(
            find_tunnel_by_name("MAIL", &definitions).map(|tunnel| tunnel.id.as_str()),
            Ok("2")
        );
    }

    #[test]
    fn should_not_find_a_tunnel_by_tag_when_looking_for_a_name() {
        assert!(find_tunnel_by_name("dev", &catalog()).is_err());
    }

    #[test]
    fn should_list_every_tunnel_when_resolving_all() {
        assert_eq!(all_tunnels(&catalog()).len(), 3);
    }
}
