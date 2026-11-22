use std::collections::BTreeMap;

use super::plan_types::ConfigImportEntryWarning;
use crate::runtime::NewTunnel;
use crate::store::{
    BROWSER_OPENER_ID, OpenOptions, TunnelDefinition, TunnelId, ValidationError,
    validate_tunnel_definition, validate_tunnel_host,
};
use ulid::Ulid;

/// An entry of the file once the host map and the opener check were applied
/// and its own rules were checked, before it is compared with anything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreparedEntry {
    pub(super) file_host: String,
    pub(super) tunnel: NewTunnel,
    pub(super) warnings: Vec<ConfigImportEntryWarning>,
    pub(super) problems: Vec<String>,
}

impl PreparedEntry {
    pub(super) fn is_valid(&self) -> bool {
        self.problems.is_empty()
    }

    /// The tunnel as a definition with a fixed placeholder id, to be compared
    /// or validated; the id never reaches the store.
    pub(super) fn definition(&self) -> TunnelDefinition {
        self.tunnel.clone().into_definition(placeholder_id())
    }
}

/// Applies the host map and drops an opener this machine does not know, then
/// collects what breaks the rules of a tunnel, the host alias rule included.
pub(super) fn prepare_entry(
    file_tunnel: NewTunnel,
    host_map: &BTreeMap<String, String>,
    opener_ids: &[String],
) -> PreparedEntry {
    let file_host = file_tunnel.host.clone();
    let host = host_map
        .get(&file_host)
        .cloned()
        .unwrap_or_else(|| file_host.clone());
    let (open, warnings) = drop_unknown_opener(file_tunnel.open, opener_ids);
    let tunnel = NewTunnel {
        host,
        open,
        ..file_tunnel
    };
    let mut prepared = PreparedEntry {
        file_host,
        tunnel,
        warnings,
        problems: Vec::new(),
    };
    prepared.problems = find_problems(&prepared.definition());
    prepared
}

fn drop_unknown_opener(
    open: OpenOptions,
    opener_ids: &[String],
) -> (OpenOptions, Vec<ConfigImportEntryWarning>) {
    let Some(opener) = open.with.as_deref() else {
        return (open, Vec::new());
    };
    let is_known = opener == BROWSER_OPENER_ID || opener_ids.iter().any(|id| id == opener);
    if is_known {
        return (open, Vec::new());
    }
    let warning = ConfigImportEntryWarning::OpenerDropped {
        opener: opener.to_owned(),
    };
    (OpenOptions { with: None, ..open }, vec![warning])
}

fn find_problems(definition: &TunnelDefinition) -> Vec<String> {
    let host_problems = validate_tunnel_host(definition).err().unwrap_or_default();
    let has_host_problem = !host_problems.is_empty();
    let rule_problems = validate_tunnel_definition(definition)
        .err()
        .unwrap_or_default()
        .into_iter()
        .filter(|problem| {
            !has_host_problem || !matches!(problem, ValidationError::EmptyHost { .. })
        });
    host_problems
        .into_iter()
        .chain(rule_problems)
        .map(|problem| problem.to_string().replace(&placeholder_suffix(), ""))
        .collect()
}

fn placeholder_suffix() -> String {
    format!(" ({})", placeholder_id())
}

fn placeholder_id() -> TunnelId {
    TunnelId::from_ulid(Ulid::nil())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::store::TunnelKind;

    fn file_tunnel(host: &str) -> NewTunnel {
        NewTunnel {
            host: host.to_owned(),
            kind: TunnelKind::Local,
            listen_port: 3333,
            listen_address: None,
            target_host: None,
            target_port: None,
            tags: Vec::new(),
            open: OpenOptions::default(),
            standby: None,
        }
    }

    fn prepare(tunnel: NewTunnel) -> PreparedEntry {
        prepare_entry(tunnel, &BTreeMap::new(), &["viewer".to_owned()])
    }

    #[test]
    fn should_be_valid_when_the_tunnel_breaks_no_rule() {
        assert!(prepare(file_tunnel("nova")).is_valid());
    }

    #[test]
    fn should_replace_the_host_when_the_map_renames_it() {
        let map = BTreeMap::from([("oldbox".to_owned(), "nova".to_owned())]);

        let prepared = prepare_entry(file_tunnel("oldbox"), &map, &[]);

        assert_eq!(prepared.tunnel.host, "nova");
        assert_eq!(prepared.file_host, "oldbox");
    }

    #[test]
    fn should_refuse_the_mapped_host_when_it_breaks_the_host_rule() {
        let map = BTreeMap::from([("oldbox".to_owned(), "-oProxyCommand=x".to_owned())]);

        let prepared = prepare_entry(file_tunnel("oldbox"), &map, &[]);

        assert_eq!(prepared.problems.len(), 1);
        assert!(prepared.problems[0].contains("starts with a dash"));
    }

    #[test]
    fn should_refuse_the_file_host_when_it_breaks_the_host_rule() {
        let prepared = prepare(file_tunnel("a b"));

        assert!(!prepared.is_valid());
    }

    #[test]
    fn should_report_one_problem_when_the_host_is_empty() {
        let prepared = prepare(file_tunnel(""));

        assert_eq!(prepared.problems.len(), 1);
    }

    #[test]
    fn should_report_the_store_rules_when_the_tunnel_breaks_them() {
        let tunnel = NewTunnel {
            listen_port: 0,
            tags: vec!["a b".to_owned()],
            ..file_tunnel("nova")
        };

        assert_eq!(prepare(tunnel).problems.len(), 2);
    }

    #[test]
    fn should_drop_the_opener_when_no_opener_has_that_id() {
        let tunnel = NewTunnel {
            open: OpenOptions {
                with: Some("ghost".to_owned()),
                path: Some("/x".to_owned()),
                ..OpenOptions::default()
            },
            ..file_tunnel("nova")
        };

        let prepared = prepare(tunnel);

        assert_eq!(prepared.tunnel.open.with, None);
        assert_eq!(prepared.tunnel.open.path.as_deref(), Some("/x"));
        assert_eq!(
            prepared.warnings,
            vec![ConfigImportEntryWarning::OpenerDropped {
                opener: "ghost".to_owned()
            }]
        );
    }

    #[test]
    fn should_keep_the_opener_when_it_is_the_browser_or_a_known_one() {
        for opener in [BROWSER_OPENER_ID, "viewer"] {
            let tunnel = NewTunnel {
                open: OpenOptions {
                    with: Some(opener.to_owned()),
                    ..OpenOptions::default()
                },
                ..file_tunnel("nova")
            };

            let prepared = prepare(tunnel);

            assert_eq!(prepared.tunnel.open.with.as_deref(), Some(opener));
            assert_eq!(prepared.warnings, Vec::new());
        }
    }
}
