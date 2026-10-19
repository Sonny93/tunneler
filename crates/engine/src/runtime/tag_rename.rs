use crate::store::TunnelDefinition;

/// The tunnels and the launch tags once a tag was renamed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagRename {
    pub(crate) tunnels: Vec<TunnelDefinition>,
    pub(crate) start_on_launch_tags: Vec<String>,
    pub(crate) renamed_count: usize,
}

/// Renames `from` to `to` in every tunnel and in the launch tags. A list that
/// already carried `to` keeps it once. `renamed_count` counts the tunnels that
/// carried `from`.
pub(crate) fn rename_tag(
    tunnels: &[TunnelDefinition],
    start_on_launch_tags: &[String],
    from: &str,
    to: &str,
) -> TagRename {
    let renamed_count = tunnels
        .iter()
        .filter(|tunnel| tunnel.tags.iter().any(|tag| tag == from))
        .count();
    let renamed_tunnels = tunnels
        .iter()
        .map(|tunnel| TunnelDefinition {
            tags: rename_in(&tunnel.tags, from, to),
            ..tunnel.clone()
        })
        .collect();
    TagRename {
        tunnels: renamed_tunnels,
        start_on_launch_tags: rename_in(start_on_launch_tags, from, to),
        renamed_count,
    }
}

fn rename_in(tags: &[String], from: &str, to: &str) -> Vec<String> {
    let mut renamed: Vec<String> = Vec::with_capacity(tags.len());
    for tag in tags {
        let next = if tag == from { to } else { tag.as_str() };
        if !renamed.iter().any(|existing| existing == next) {
            renamed.push(next.to_owned());
        }
    }
    renamed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{TunnelId, TunnelKind};

    fn tunnel(tags: &[&str]) -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            host: "host".to_owned(),
            kind: TunnelKind::Local,
            listen_port: 8080,
            listen_address: None,
            target_host: None,
            target_port: None,
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        }
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn should_rename_the_tag_in_place_when_a_tunnel_carries_it() {
        let tunnels = [tunnel(&["dev", "web"])];

        let renamed = rename_tag(&tunnels, &[], "dev", "staging");

        assert_eq!(renamed.tunnels[0].tags, strings(&["staging", "web"]));
    }

    #[test]
    fn should_count_only_the_tunnels_that_carried_the_tag() {
        let tunnels = [tunnel(&["dev"]), tunnel(&["web"])];

        let renamed = rename_tag(&tunnels, &[], "dev", "staging");

        assert_eq!(renamed.renamed_count, 1);
    }

    #[test]
    fn should_keep_the_target_once_when_a_tunnel_already_carries_it() {
        let tunnels = [tunnel(&["dev", "web"])];

        let renamed = rename_tag(&tunnels, &[], "dev", "web");

        assert_eq!(renamed.tunnels[0].tags, strings(&["web"]));
    }

    #[test]
    fn should_leave_the_other_tunnels_untouched() {
        let tunnels = [tunnel(&["dev"]), tunnel(&["web"])];

        let renamed = rename_tag(&tunnels, &[], "dev", "staging");

        assert_eq!(renamed.tunnels[1], tunnels[1]);
    }

    #[test]
    fn should_rename_the_tag_in_the_launch_tags() {
        let renamed = rename_tag(&[], &strings(&["dev", "web"]), "dev", "staging");

        assert_eq!(renamed.start_on_launch_tags, strings(&["staging", "web"]));
    }

    #[test]
    fn should_keep_the_target_once_in_the_launch_tags() {
        let renamed = rename_tag(&[], &strings(&["web", "dev"]), "dev", "web");

        assert_eq!(renamed.start_on_launch_tags, strings(&["web"]));
    }
}
