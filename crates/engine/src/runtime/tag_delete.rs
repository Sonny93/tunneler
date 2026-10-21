use crate::store::TunnelDefinition;

/// The tunnels and the launch tags once a tag was deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagDeletion {
    pub(crate) tunnels: Vec<TunnelDefinition>,
    pub(crate) start_on_launch_tags: Vec<String>,
    pub(crate) removed_count: usize,
}

/// Removes `tag` from every tunnel and from the launch tags. The tunnels
/// themselves are kept. `removed_count` counts the tunnels that carried `tag`.
pub(crate) fn delete_tag(
    tunnels: &[TunnelDefinition],
    start_on_launch_tags: &[String],
    tag: &str,
) -> TagDeletion {
    let removed_count = tunnels
        .iter()
        .filter(|tunnel| tunnel.tags.iter().any(|existing| existing == tag))
        .count();
    let remaining_tunnels = tunnels
        .iter()
        .map(|tunnel| TunnelDefinition {
            tags: without(&tunnel.tags, tag),
            ..tunnel.clone()
        })
        .collect();
    TagDeletion {
        tunnels: remaining_tunnels,
        start_on_launch_tags: without(start_on_launch_tags, tag),
        removed_count,
    }
}

fn without(tags: &[String], tag: &str) -> Vec<String> {
    tags.iter()
        .filter(|existing| *existing != tag)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{TunnelId, TunnelKind};

    fn tunnel(listen_port: u16, tags: &[&str]) -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            host: "host".to_owned(),
            kind: TunnelKind::Local,
            listen_port,
            listen_address: None,
            target_host: None,
            target_port: None,
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            open: crate::store::OpenOptions::default(),
            standby: None,
        }
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn should_remove_the_tag_and_keep_the_other_tags_when_a_tunnel_carries_it() {
        let tunnels = [tunnel(8080, &["dev", "web"])];

        let deletion = delete_tag(&tunnels, &[], "dev");

        assert_eq!(deletion.tunnels[0].tags, strings(&["web"]));
    }

    #[test]
    fn should_keep_the_tunnel_when_its_only_tag_is_deleted() {
        let tunnels = [tunnel(8080, &["dev"])];

        let deletion = delete_tag(&tunnels, &[], "dev");

        assert_eq!(
            (deletion.tunnels.len(), deletion.tunnels[0].tags.is_empty()),
            (1, true)
        );
    }

    #[test]
    fn should_count_only_the_tunnels_that_carried_the_tag() {
        let tunnels = [tunnel(8080, &["dev"]), tunnel(8081, &["web"])];

        let deletion = delete_tag(&tunnels, &[], "dev");

        assert_eq!(deletion.removed_count, 1);
    }

    #[test]
    fn should_leave_the_other_tunnels_untouched() {
        let tunnels = [tunnel(8080, &["dev"]), tunnel(8081, &["web"])];

        let deletion = delete_tag(&tunnels, &[], "dev");

        assert_eq!(deletion.tunnels[1], tunnels[1]);
    }

    #[test]
    fn should_remove_the_tag_from_the_launch_tags() {
        let deletion = delete_tag(&[], &strings(&["dev", "web"]), "dev");

        assert_eq!(deletion.start_on_launch_tags, strings(&["web"]));
    }
}
