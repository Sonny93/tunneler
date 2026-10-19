use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use serde::Deserialize;
use serde::de::IgnoredAny;
use toml_edit::DocumentMut;
use ulid::Ulid;

use super::document_update;
use super::error::{SourceLocation, StoreError};
use super::migration::{CURRENT_VERSION, migrate};
use super::model::{
    HostOverrides, Settings, StoreDocument, TunnelDefinition, TunnelId, TunnelKind, X11Launcher,
};
use super::validation::validate;

const IMPLICIT_VERSION: u32 = 1;

/// The result of reading the store file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOutcome {
    /// The parsed and validated content.
    pub document: StoreDocument,
    /// Whether reading had to fill gaps (ids generated for tunnels written
    /// by hand). The file on disk no longer matches the document until the
    /// caller saves it.
    pub was_normalized: bool,
}

#[derive(Deserialize)]
struct VersionProbe {
    version: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredDocument {
    #[serde(rename = "version")]
    _version: Option<u32>,
    #[serde(default)]
    settings: StoredSettings,
    #[serde(default)]
    tunnels: Vec<StoredTunnel>,
    #[serde(default)]
    hosts: BTreeMap<String, StoredHostOverrides>,
    #[serde(default)]
    x11_launchers: Vec<StoredLauncher>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSettings {
    default_bind_address: Option<String>,
    default_remote_bind_address: Option<String>,
    keepalive_interval_s: Option<u64>,
    keepalive_max: Option<u32>,
    session_idle_grace_s: Option<u64>,
    reconnect_initial_delay_ms: Option<u64>,
    reconnect_max_delay_ms: Option<u64>,
    health_probe: Option<bool>,
    restore_on_launch: Option<bool>,
    start_on_launch_tags: Option<Vec<String>>,
    launch_at_login: Option<bool>,
    use_os_keyring: Option<bool>,
    x11_display: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum StoredKind {
    Local,
    Remote,
    Dynamic,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredTunnel {
    id: Option<Ulid>,
    #[serde(default, rename = "name")]
    _legacy_name: Option<IgnoredAny>,
    host: String,
    kind: StoredKind,
    listen_port: u16,
    listen_address: Option<String>,
    target_host: Option<String>,
    target_port: Option<u16>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredHostOverrides {
    x11_display: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredLauncher {
    host: String,
    name: String,
    command: String,
}

/// Reads, migrates, validates and converts the text of a store file.
pub(crate) fn parse_document(path: &Path, text: &str) -> Result<LoadOutcome, StoreError> {
    let migrated = parse_migrated(path, text)?.to_string();
    let stored: StoredDocument =
        toml_edit::de::from_str(&migrated).map_err(|error| parse_error(path, &migrated, &error))?;
    let outcome = stored.into_outcome();
    validate(&outcome.document).map_err(StoreError::Validation)?;
    Ok(outcome)
}

/// Produces the text of the store file for `document`, updating
/// `existing_text` in place so comments and formatting survive.
pub(crate) fn render_document(
    path: &Path,
    existing_text: Option<&str>,
    document: &StoreDocument,
) -> Result<String, StoreError> {
    let mut toml = match existing_text {
        Some(text) => parse_migrated(path, text)?,
        None => DocumentMut::new(),
    };
    document_update::apply(&mut toml, document);
    Ok(toml.to_string())
}

fn parse_migrated(path: &Path, text: &str) -> Result<DocumentMut, StoreError> {
    let probe: VersionProbe =
        toml_edit::de::from_str(text).map_err(|error| parse_error(path, text, &error))?;
    let version = probe.version.unwrap_or(IMPLICIT_VERSION);
    if version > CURRENT_VERSION {
        return Err(StoreError::UnsupportedVersion {
            found: version,
            supported: CURRENT_VERSION,
        });
    }
    let toml = DocumentMut::from_str(text).map_err(|error| StoreError::Parse {
        path: path.to_path_buf(),
        location: error.span().map(|span| locate(text, &span)),
        message: error.message().to_owned(),
    })?;
    migrate(toml, version)
}

fn parse_error(path: &Path, text: &str, error: &toml_edit::de::Error) -> StoreError {
    StoreError::Parse {
        path: path.to_path_buf(),
        location: error.span().map(|span| locate(text, &span)),
        message: error.message().to_owned(),
    }
}

fn locate(text: &str, span: &Range<usize>) -> SourceLocation {
    let before = &text[..span.start.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    SourceLocation {
        line,
        column: before[line_start..].chars().count() + 1,
    }
}

impl StoredDocument {
    fn into_outcome(self) -> LoadOutcome {
        let mut was_normalized = false;
        let tunnels = self
            .tunnels
            .into_iter()
            .map(|stored| {
                was_normalized |= stored.id.is_none();
                stored.into_definition()
            })
            .collect();
        let document = StoreDocument {
            settings: self.settings.into_settings(),
            tunnels,
            hosts: self
                .hosts
                .into_iter()
                .map(|(alias, stored)| (alias, stored.into_overrides()))
                .collect(),
            x11_launchers: self
                .x11_launchers
                .into_iter()
                .map(StoredLauncher::into_launcher)
                .collect(),
        };
        LoadOutcome {
            document,
            was_normalized,
        }
    }
}

impl StoredSettings {
    fn into_settings(self) -> Settings {
        let defaults = Settings::default();
        Settings {
            default_bind_address: self
                .default_bind_address
                .unwrap_or(defaults.default_bind_address),
            default_remote_bind_address: self
                .default_remote_bind_address
                .unwrap_or(defaults.default_remote_bind_address),
            keepalive_interval: self
                .keepalive_interval_s
                .map_or(defaults.keepalive_interval, Duration::from_secs),
            keepalive_max: self.keepalive_max.unwrap_or(defaults.keepalive_max),
            session_idle_grace: self
                .session_idle_grace_s
                .map_or(defaults.session_idle_grace, Duration::from_secs),
            reconnect_initial_delay: self
                .reconnect_initial_delay_ms
                .map_or(defaults.reconnect_initial_delay, Duration::from_millis),
            reconnect_max_delay: self
                .reconnect_max_delay_ms
                .map_or(defaults.reconnect_max_delay, Duration::from_millis),
            health_probe: self.health_probe.unwrap_or(defaults.health_probe),
            restore_on_launch: self.restore_on_launch.unwrap_or(defaults.restore_on_launch),
            start_on_launch_tags: self
                .start_on_launch_tags
                .unwrap_or(defaults.start_on_launch_tags),
            launch_at_login: self.launch_at_login.unwrap_or(defaults.launch_at_login),
            use_os_keyring: self.use_os_keyring.unwrap_or(defaults.use_os_keyring),
            x11_display: self.x11_display.filter(|display| !display.is_empty()),
        }
    }
}

impl StoredTunnel {
    fn into_definition(self) -> TunnelDefinition {
        TunnelDefinition {
            id: self.id.map_or_else(TunnelId::generate, TunnelId::from_ulid),
            host: self.host,
            kind: self.kind.into(),
            listen_port: self.listen_port,
            listen_address: self.listen_address,
            target_host: self.target_host,
            target_port: self.target_port,
            tags: self.tags,
        }
    }
}

impl From<StoredKind> for TunnelKind {
    fn from(kind: StoredKind) -> Self {
        match kind {
            StoredKind::Local => Self::Local,
            StoredKind::Remote => Self::Remote,
            StoredKind::Dynamic => Self::Dynamic,
        }
    }
}

impl StoredHostOverrides {
    fn into_overrides(self) -> HostOverrides {
        HostOverrides {
            x11_display: self.x11_display.filter(|display| !display.is_empty()),
        }
    }
}

impl StoredLauncher {
    fn into_launcher(self) -> X11Launcher {
        X11Launcher {
            host: self.host,
            name: self.name,
            command: self.command,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::test_support::tunnel_on;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    const PLAN_EXAMPLE: &str = include_str!("fixtures/plan_example.toml");

    fn path() -> PathBuf {
        PathBuf::from("tunnels.toml")
    }

    fn parse(text: &str) -> Result<LoadOutcome, StoreError> {
        parse_document(&path(), text)
    }

    fn render(existing_text: Option<&str>, document: &StoreDocument) -> String {
        render_document(&path(), existing_text, document).unwrap()
    }

    fn parse_error_message(text: &str) -> String {
        parse(text).unwrap_err().to_string()
    }

    #[test]
    fn should_parse_every_field_when_reading_the_plan_example() {
        let outcome = parse(PLAN_EXAMPLE).unwrap();

        let tunnel = &outcome.document.tunnels[0];
        assert_eq!(
            (
                tunnel.host.as_str(),
                tunnel.kind,
                tunnel.listen_port,
                tunnel.tags.clone()
            ),
            (
                "nova",
                TunnelKind::Local,
                3333,
                vec!["dev".to_owned(), "adonis".to_owned()]
            )
        );
    }

    #[test]
    fn should_load_a_tunnel_when_the_file_still_carries_a_legacy_name_key() {
        let outcome = parse(
            "[[tunnels]]\nname = \"Adonis\"\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3333\n",
        );

        assert_eq!(outcome.unwrap().document.tunnels[0].listen_port, 3333);
    }

    #[test]
    fn should_drop_the_legacy_name_key_when_the_file_is_rewritten() {
        let text = "[[tunnels]]\nid = \"01JA0000000000000000000000\"\nname = \"Adonis\"\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3333\n";
        let document = parse(text).unwrap().document;

        assert!(!render(Some(text), &document).contains("name"));
    }

    #[test]
    fn should_parse_hosts_and_launchers_when_reading_the_plan_example() {
        let document = parse(PLAN_EXAMPLE).unwrap().document;

        assert_eq!(
            (
                document.hosts["nova"].x11_display.clone(),
                document.x11_launchers[0].command.clone()
            ),
            (
                Some("198.51.100.135:0.0".to_owned()),
                "code --wait".to_owned()
            )
        );
    }

    #[test]
    fn should_parse_default_settings_when_reading_the_plan_example() {
        let document = parse(PLAN_EXAMPLE).unwrap().document;

        assert_eq!(document.settings, Settings::default());
    }

    #[test]
    fn should_not_normalize_when_every_tunnel_has_an_id() {
        assert!(!parse(PLAN_EXAMPLE).unwrap().was_normalized);
    }

    #[test]
    fn should_round_trip_the_plan_example_through_a_fresh_render() {
        let document = parse(PLAN_EXAMPLE).unwrap().document;

        let rendered = render(None, &document);

        assert_eq!(parse(&rendered).unwrap().document, document);
    }

    #[test]
    fn should_leave_text_untouched_when_rendering_an_unchanged_document() {
        let document = parse(PLAN_EXAMPLE).unwrap().document;

        assert_eq!(render(Some(PLAN_EXAMPLE), &document), PLAN_EXAMPLE);
    }

    #[test]
    fn should_apply_defaults_when_settings_keys_are_omitted() {
        let outcome = parse("version = 1\n").unwrap();

        assert_eq!(outcome.document, StoreDocument::default());
    }

    #[test]
    fn should_apply_defaults_when_the_file_is_empty() {
        assert_eq!(parse("").unwrap().document, StoreDocument::default());
    }

    #[test]
    fn should_keep_other_defaults_when_only_one_setting_is_given() {
        let outcome = parse("[settings]\nkeepalive_max = 5\n").unwrap();

        assert_eq!(
            outcome.document.settings,
            Settings {
                keepalive_max: 5,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn should_read_durations_in_their_unit_when_settings_are_given() {
        let outcome =
            parse("[settings]\nkeepalive_interval_s = 20\nreconnect_initial_delay_ms = 250\n")
                .unwrap();

        assert_eq!(
            (
                outcome.document.settings.keepalive_interval,
                outcome.document.settings.reconnect_initial_delay
            ),
            (Duration::from_secs(20), Duration::from_millis(250))
        );
    }

    #[test]
    fn should_leave_the_os_keyring_off_when_the_setting_is_omitted() {
        assert!(
            !parse("version = 1\n")
                .unwrap()
                .document
                .settings
                .use_os_keyring
        );
    }

    #[test]
    fn should_read_the_os_keyring_setting_when_it_is_given() {
        let outcome = parse("[settings]\nuse_os_keyring = true\n").unwrap();

        assert!(outcome.document.settings.use_os_keyring);
    }

    #[test]
    fn should_write_the_os_keyring_setting_when_rendering() {
        let mut document = StoreDocument::default();
        document.settings.use_os_keyring = true;

        assert!(render(None, &document).contains("use_os_keyring = true"));
    }

    #[test]
    fn should_report_the_line_when_a_key_is_unknown() {
        let message = parse_error_message(
            "version = 1\n\n[[tunnels]]\nhost = \"h\"\nkind = \"local\"\nlisen_port = 1\n",
        );

        assert!(message.starts_with("tunnels.toml:6:1:"), "{message}");
    }

    #[test]
    fn should_name_the_key_when_it_is_unknown() {
        let message = parse_error_message("[settings]\nkeepalive_intervall_s = 3\n");

        assert!(message.contains("keepalive_intervall_s"), "{message}");
    }

    #[test]
    fn should_reject_unknown_top_level_key() {
        assert!(matches!(
            parse("colour = 1\n"),
            Err(StoreError::Parse { .. })
        ));
    }

    #[test]
    fn should_report_the_position_when_toml_syntax_is_broken() {
        let message = parse_error_message("version = 1\n[[tunnels]\n");

        assert!(message.starts_with("tunnels.toml:2:"), "{message}");
    }

    #[test]
    fn should_reject_invalid_tunnel_id_with_its_position() {
        let message = parse_error_message(
            "[[tunnels]]\nid = \"nope\"\nhost = \"h\"\nkind = \"local\"\nlisten_port = 1\n",
        );

        assert!(message.starts_with("tunnels.toml:2:"), "{message}");
    }

    #[test]
    fn should_reject_unknown_tunnel_kind() {
        assert!(matches!(
            parse("[[tunnels]]\nhost = \"h\"\nkind = \"sideways\"\nlisten_port = 1\n"),
            Err(StoreError::Parse { .. })
        ));
    }

    #[test]
    fn should_reject_port_out_of_range() {
        assert!(matches!(
            parse("[[tunnels]]\nhost = \"h\"\nkind = \"local\"\nlisten_port = 70000\n"),
            Err(StoreError::Parse { .. })
        ));
    }

    #[test]
    fn should_generate_an_id_and_flag_normalization_when_a_tunnel_has_none() {
        let outcome =
            parse("[[tunnels]]\nhost = \"h\"\nkind = \"local\"\nlisten_port = 1\n").unwrap();

        assert!(outcome.was_normalized);
    }

    #[test]
    fn should_generate_distinct_ids_when_several_tunnels_have_none() {
        let outcome = parse(concat!(
            "[[tunnels]]\nhost = \"h\"\nkind = \"local\"\nlisten_port = 1\n",
            "[[tunnels]]\nhost = \"h\"\nkind = \"local\"\nlisten_port = 2\n",
        ))
        .unwrap();

        assert_ne!(
            outcome.document.tunnels[0].id,
            outcome.document.tunnels[1].id
        );
    }

    #[test]
    fn should_treat_missing_version_as_the_first_version() {
        assert!(parse("[settings]\n").is_ok());
    }

    #[test]
    fn should_refuse_file_when_its_version_is_newer() {
        let outcome = parse("version = 2\nfuture_key = true\n");

        assert!(matches!(
            outcome,
            Err(StoreError::UnsupportedVersion {
                found: 2,
                supported: 1
            })
        ));
    }

    #[test]
    fn should_refuse_to_render_over_a_file_written_by_a_newer_version() {
        let outcome = render_document(&path(), Some("version = 9\n"), &StoreDocument::default());

        assert!(matches!(
            outcome,
            Err(StoreError::UnsupportedVersion { found: 9, .. })
        ));
    }

    #[test]
    fn should_refuse_to_render_over_a_file_with_broken_syntax() {
        let outcome = render_document(&path(), Some("[[tunnels]\n"), &StoreDocument::default());

        assert!(matches!(outcome, Err(StoreError::Parse { .. })));
    }

    #[test]
    fn should_report_validation_errors_when_the_content_breaks_a_rule() {
        let outcome = parse("[[tunnels]]\nhost = \"h\"\nkind = \"local\"\nlisten_port = 0\n");

        assert!(matches!(outcome, Err(StoreError::Validation(_))));
    }

    #[test]
    fn should_write_version_and_sections_in_the_plan_layout_when_the_file_is_new() {
        let document = StoreDocument {
            tunnels: vec![TunnelDefinition {
                id: "01JA0000000000000000000000".parse().unwrap(),
                tags: vec!["dev".to_owned()],
                ..tunnel_on(TunnelKind::Local, 3333)
            }],
            ..StoreDocument::default()
        };

        let rendered = render(None, &document);

        assert!(
            rendered.starts_with("version = 1\n\n[settings]\n"),
            "{rendered}"
        );
        assert!(
            rendered.ends_with(
                "\n[[tunnels]]\nid = \"01JA0000000000000000000000\"\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3333\ntags = [\"dev\"]\n"
            ),
            "{rendered}"
        );
    }

    #[test]
    fn should_omit_optional_tunnel_fields_when_they_are_unset() {
        let document = StoreDocument {
            tunnels: vec![tunnel_on(TunnelKind::Local, 3333)],
            ..StoreDocument::default()
        };

        let rendered = render(None, &document);

        assert!(!rendered.contains("listen_address"), "{rendered}");
    }

    #[test]
    fn should_write_tags_when_the_tunnel_has_none() {
        let document = StoreDocument {
            tunnels: vec![tunnel_on(TunnelKind::Local, 3333)],
            ..StoreDocument::default()
        };

        assert!(render(None, &document).contains("tags = []"));
    }

    #[test]
    fn should_write_optional_fields_when_they_are_set() {
        let definition = TunnelDefinition {
            listen_address: Some("0.0.0.0".to_owned()),
            target_host: Some("db".to_owned()),
            target_port: Some(5432),
            ..tunnel_on(TunnelKind::Remote, 3333)
        };
        let document = StoreDocument {
            tunnels: vec![definition],
            ..StoreDocument::default()
        };

        let rendered = render(None, &document);

        assert_eq!(parse(&rendered).unwrap().document, document);
    }

    #[test]
    fn should_round_trip_host_aliases_that_need_quoting() {
        let mut document = StoreDocument::default();
        document.hosts.insert(
            "build.example.com".to_owned(),
            HostOverrides {
                x11_display: Some(":1".to_owned()),
            },
        );

        let rendered = render(None, &document);

        assert_eq!(parse(&rendered).unwrap().document, document);
    }

    #[test]
    fn should_locate_the_offset_in_lines_and_characters() {
        let location = locate("ab\ncdé\nfg", &(7..8));

        assert_eq!(location, SourceLocation { line: 2, column: 4 });
    }
}
