use toml_edit::{ArrayOfTables, DocumentMut, value};

use super::{EXPORT_FORMAT, EXPORT_VERSION};
use crate::store::{TunnelDefinition, export_tunnel_table};

const HEADER: &str = "# Tunneler export (format version 1)\n";
const FORMAT_KEY: &str = "format";
const VERSION_KEY: &str = "version";
const TUNNELS_KEY: &str = "tunnels";

/// Which saved tunnels to export: a tunnel is exported when its host is in
/// `hosts` or one of its tags is in `tags`. Both empty exports every tunnel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigExportFilter {
    /// The host aliases to export.
    pub hosts: Vec<String>,
    /// The tags to export.
    pub tags: Vec<String>,
}

impl ConfigExportFilter {
    fn selects(&self, tunnel: &TunnelDefinition) -> bool {
        if self.hosts.is_empty() && self.tags.is_empty() {
            return true;
        }
        self.hosts.contains(&tunnel.host) || tunnel.tags.iter().any(|tag| self.tags.contains(tag))
    }
}

/// A rendered export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigExport {
    /// The text of the file.
    pub contents: String,
    /// How many tunnels it holds.
    pub tunnel_count: usize,
}

/// Renders the tunnels the filter selects, in the given order, as the text of
/// an export file: no ids, and nothing but the tunnels (no settings, openers,
/// launchers or host overrides).
pub(crate) fn export_tunnels(
    tunnels: &[TunnelDefinition],
    filter: &ConfigExportFilter,
) -> ConfigExport {
    let selected: Vec<&TunnelDefinition> = tunnels
        .iter()
        .filter(|tunnel| filter.selects(tunnel))
        .collect();
    let mut document = DocumentMut::new();
    document.insert(FORMAT_KEY, value(EXPORT_FORMAT));
    document.insert(VERSION_KEY, value(i64::from(EXPORT_VERSION)));
    if !selected.is_empty() {
        let mut array = ArrayOfTables::new();
        for tunnel in &selected {
            array.push(export_tunnel_table(tunnel));
        }
        document.insert(TUNNELS_KEY, toml_edit::Item::ArrayOfTables(array));
    }
    ConfigExport {
        contents: format!("{HEADER}{document}"),
        tunnel_count: selected.len(),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::store::test_support::{tunnel, tunnel_on};
    use crate::store::{OpenOptions, OpenScheme, TunnelKind};

    fn on_host(host: &str, port: u16, tags: &[&str]) -> TunnelDefinition {
        TunnelDefinition {
            host: host.to_owned(),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            ..tunnel_on(TunnelKind::Local, port)
        }
    }

    fn ports_of(export: &ConfigExport) -> Vec<u16> {
        let parsed = crate::transfer::parse::parse_import_file(&export.contents).unwrap();
        parsed
            .tunnels
            .iter()
            .map(|tunnel| tunnel.listen_port)
            .collect()
    }

    fn catalog() -> Vec<TunnelDefinition> {
        vec![
            on_host("nova", 3001, &["dev"]),
            on_host("nova", 3002, &["mail"]),
            on_host("atlas", 3003, &["dev", "mail"]),
            on_host("atlas", 3004, &[]),
        ]
    }

    #[test]
    fn should_render_the_documented_layout_when_a_tunnel_has_tags_and_open_options() {
        let definition = TunnelDefinition {
            tags: vec!["dev".to_owned()],
            open: OpenOptions {
                path: Some("/admin".to_owned()),
                ..OpenOptions::default()
            },
            ..tunnel(TunnelKind::Local)
        };

        let export = export_tunnels(&[definition], &ConfigExportFilter::default());

        assert_eq!(
            export.contents,
            "# Tunneler export (format version 1)\nformat = \"tunneler-export\"\nversion = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3333\ntags = [\"dev\"]\n\n[tunnels.open]\npath = \"/admin\"\n"
        );
    }

    #[test]
    fn should_leave_out_the_id_and_the_optional_fields_when_they_are_absent() {
        let export = export_tunnels(
            &[tunnel(TunnelKind::Dynamic)],
            &ConfigExportFilter::default(),
        );

        assert_eq!(
            export.contents,
            "# Tunneler export (format version 1)\nformat = \"tunneler-export\"\nversion = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"dynamic\"\nlisten_port = 3333\n"
        );
    }

    #[test]
    fn should_write_every_optional_field_when_the_tunnel_sets_them() {
        let definition = TunnelDefinition {
            listen_address: Some("0.0.0.0".to_owned()),
            target_host: Some("db".to_owned()),
            target_port: Some(5432),
            standby: Some(true),
            open: OpenOptions {
                scheme: Some(OpenScheme::Https),
                with: Some("viewer".to_owned()),
                ..OpenOptions::default()
            },
            ..tunnel(TunnelKind::Local)
        };

        let export = export_tunnels(&[definition], &ConfigExportFilter::default());

        assert!(export.contents.contains("listen_address = \"0.0.0.0\""));
        assert!(export.contents.contains("target_host = \"db\""));
        assert!(export.contents.contains("target_port = 5432"));
        assert!(export.contents.contains("standby = true"));
        assert!(export.contents.contains("scheme = \"https\""));
        assert!(export.contents.contains("with = \"viewer\""));
        assert!(!export.contents.contains("id ="));
    }

    #[test]
    fn should_export_every_tunnel_when_there_is_no_filter() {
        let export = export_tunnels(&catalog(), &ConfigExportFilter::default());

        assert_eq!(export.tunnel_count, 4);
        assert_eq!(ports_of(&export), vec![3001, 3002, 3003, 3004]);
    }

    #[test]
    fn should_export_the_tunnels_of_the_hosts_when_a_host_filter_is_given() {
        let filter = ConfigExportFilter {
            hosts: vec!["atlas".to_owned()],
            tags: Vec::new(),
        };

        assert_eq!(
            ports_of(&export_tunnels(&catalog(), &filter)),
            vec![3003, 3004]
        );
    }

    #[test]
    fn should_export_the_tunnels_with_the_tags_when_a_tag_filter_is_given() {
        let filter = ConfigExportFilter {
            hosts: Vec::new(),
            tags: vec!["mail".to_owned()],
        };

        assert_eq!(
            ports_of(&export_tunnels(&catalog(), &filter)),
            vec![3002, 3003]
        );
    }

    #[test]
    fn should_export_the_union_when_both_filters_are_given() {
        let filter = ConfigExportFilter {
            hosts: vec!["atlas".to_owned()],
            tags: vec!["mail".to_owned()],
        };

        let export = export_tunnels(&catalog(), &filter);

        assert_eq!(export.tunnel_count, 3);
        assert_eq!(ports_of(&export), vec![3002, 3003, 3004]);
    }

    #[test]
    fn should_export_nothing_when_no_tunnel_matches() {
        let filter = ConfigExportFilter {
            hosts: vec!["ghost".to_owned()],
            tags: Vec::new(),
        };

        let export = export_tunnels(&catalog(), &filter);

        assert_eq!(export.tunnel_count, 0);
        assert_eq!(
            export.contents,
            "# Tunneler export (format version 1)\nformat = \"tunneler-export\"\nversion = 1\n"
        );
    }
}
