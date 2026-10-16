use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, TableLike, Value};

use super::migration::CURRENT_VERSION;
use super::model::{
    HostOverrides, Settings, StoreDocument, TunnelDefinition, TunnelId, TunnelKind, X11Launcher,
};

const VERSION_KEY: &str = "version";
const SETTINGS_KEY: &str = "settings";
const TUNNELS_KEY: &str = "tunnels";
const HOSTS_KEY: &str = "hosts";
const LAUNCHERS_KEY: &str = "x11_launchers";
const X11_DISPLAY_KEY: &str = "x11_display";
const SECTION_SEPARATOR: &str = "\n";
const FIRST_SECTION_POSITION: isize = -1;

/// Brings `toml` in line with `document`, touching only what differs so that
/// comments and formatting around untouched values survive.
pub(crate) fn apply(toml: &mut DocumentMut, document: &StoreDocument) {
    let root = toml.as_table_mut();
    set_value(root, VERSION_KEY, Value::from(i64::from(CURRENT_VERSION)));
    update_settings(root, &document.settings);
    update_tunnels(root, &document.tunnels);
    update_hosts(root, document);
    update_launchers(root, &document.x11_launchers);
}

fn update_settings(root: &mut Table, settings: &Settings) {
    let section = root
        .entry(SETTINGS_KEY)
        .or_insert_with(|| Item::Table(new_settings_section()));
    if section.as_table_like().is_none() {
        *section = Item::Table(new_settings_section());
    }
    if let Some(table) = section.as_table_like_mut() {
        for (key, entry) in settings_entries(settings) {
            set_value(table, key, entry);
        }
    }
}

fn settings_entries(settings: &Settings) -> Vec<(&'static str, Value)> {
    vec![
        (
            "default_bind_address",
            Value::from(settings.default_bind_address.as_str()),
        ),
        (
            "default_remote_bind_address",
            Value::from(settings.default_remote_bind_address.as_str()),
        ),
        (
            "keepalive_interval_s",
            integer(u128::from(settings.keepalive_interval.as_secs())),
        ),
        ("keepalive_max", integer(u128::from(settings.keepalive_max))),
        (
            "session_idle_grace_s",
            integer(u128::from(settings.session_idle_grace.as_secs())),
        ),
        (
            "reconnect_initial_delay_ms",
            integer(settings.reconnect_initial_delay.as_millis()),
        ),
        (
            "reconnect_max_delay_ms",
            integer(settings.reconnect_max_delay.as_millis()),
        ),
        ("health_probe", Value::from(settings.health_probe)),
        ("restore_on_launch", Value::from(settings.restore_on_launch)),
        (
            "start_on_launch_tags",
            string_array(&settings.start_on_launch_tags),
        ),
        ("launch_at_login", Value::from(settings.launch_at_login)),
        (
            X11_DISPLAY_KEY,
            Value::from(settings.x11_display.as_deref().unwrap_or_default()),
        ),
    ]
}

fn integer(number: u128) -> Value {
    Value::from(i64::try_from(number).unwrap_or(i64::MAX))
}

fn string_array(items: &[String]) -> Value {
    Value::Array(items.iter().map(String::as_str).collect::<Array>())
}

fn update_tunnels(root: &mut Table, tunnels: &[TunnelDefinition]) {
    update_array_of_tables(root, TUNNELS_KEY, tunnels, assign_tunnels, apply_tunnel);
}

fn assign_tunnels(array: &ArrayOfTables, tunnels: &[TunnelDefinition]) -> Vec<Option<usize>> {
    let mut claimed = vec![false; tunnels.len()];
    let mut assignments: Vec<Option<usize>> = array
        .iter()
        .map(|table| claim_tunnel_by_id(table, tunnels, &mut claimed))
        .collect();
    for (table, assignment) in array.iter().zip(assignments.iter_mut()) {
        if assignment.is_none() && !table.contains_key("id") {
            *assignment = claim_tunnel_by_name(table, tunnels, &mut claimed);
        }
    }
    assignments
}

fn claim_tunnel_by_id(
    table: &Table,
    tunnels: &[TunnelDefinition],
    claimed: &mut [bool],
) -> Option<usize> {
    let id: TunnelId = table.get("id")?.as_str()?.parse().ok()?;
    claim(tunnels, claimed, |tunnel| tunnel.id == id)
}

fn claim_tunnel_by_name(
    table: &Table,
    tunnels: &[TunnelDefinition],
    claimed: &mut [bool],
) -> Option<usize> {
    let name = table.get("name")?.as_str()?.trim().to_lowercase();
    claim(tunnels, claimed, |tunnel| {
        tunnel.name.trim().to_lowercase() == name
    })
}

fn claim<Definition>(
    definitions: &[Definition],
    claimed: &mut [bool],
    is_match: impl Fn(&Definition) -> bool,
) -> Option<usize> {
    let index = definitions
        .iter()
        .enumerate()
        .position(|(position, definition)| !claimed[position] && is_match(definition))?;
    claimed[index] = true;
    Some(index)
}

fn apply_tunnel(table: &mut Table, tunnel: &TunnelDefinition) {
    set_value(table, "id", Value::from(tunnel.id.to_string()));
    set_value(table, "name", Value::from(tunnel.name.as_str()));
    set_value(table, "host", Value::from(tunnel.host.as_str()));
    set_value(table, "kind", Value::from(kind_name(tunnel.kind)));
    set_value(
        table,
        "listen_port",
        Value::from(i64::from(tunnel.listen_port)),
    );
    set_optional(
        table,
        "listen_address",
        tunnel.listen_address.as_deref().map(Value::from),
    );
    set_optional(
        table,
        "target_host",
        tunnel.target_host.as_deref().map(Value::from),
    );
    set_optional(
        table,
        "target_port",
        tunnel.target_port.map(|port| Value::from(i64::from(port))),
    );
    set_value(table, "tags", string_array(&tunnel.tags));
}

fn kind_name(kind: TunnelKind) -> &'static str {
    match kind {
        TunnelKind::Local => "local",
        TunnelKind::Remote => "remote",
        TunnelKind::Dynamic => "dynamic",
    }
}

fn update_hosts(root: &mut Table, document: &StoreDocument) {
    if document.hosts.is_empty() && !root.contains_key(HOSTS_KEY) {
        return;
    }
    let section = root.entry(HOSTS_KEY).or_insert_with(|| {
        let mut table = Table::new();
        table.set_implicit(true);
        Item::Table(table)
    });
    if let Some(hosts) = section.as_table_like_mut() {
        remove_unknown_hosts(hosts, document);
        for (alias, overrides) in &document.hosts {
            update_host(hosts, alias, overrides);
        }
    }
}

fn remove_unknown_hosts(hosts: &mut dyn TableLike, document: &StoreDocument) {
    let removed: Vec<String> = hosts
        .iter()
        .map(|(alias, _)| alias.to_owned())
        .filter(|alias| !document.hosts.contains_key(alias))
        .collect();
    for alias in removed {
        hosts.remove(&alias);
    }
}

fn update_host(hosts: &mut dyn TableLike, alias: &str, overrides: &HostOverrides) {
    if let Some(table) = hosts.get_mut(alias).and_then(Item::as_table_like_mut) {
        apply_host(table, overrides);
        return;
    }
    let mut table = new_section();
    apply_host(&mut table, overrides);
    hosts.insert(alias, Item::Table(table));
}

fn apply_host(table: &mut dyn TableLike, overrides: &HostOverrides) {
    set_optional(
        table,
        X11_DISPLAY_KEY,
        overrides.x11_display.as_deref().map(Value::from),
    );
}

fn update_launchers(root: &mut Table, launchers: &[X11Launcher]) {
    update_array_of_tables(
        root,
        LAUNCHERS_KEY,
        launchers,
        assign_launchers,
        apply_launcher,
    );
}

fn assign_launchers(array: &ArrayOfTables, launchers: &[X11Launcher]) -> Vec<Option<usize>> {
    let mut claimed = vec![false; launchers.len()];
    array
        .iter()
        .map(|table| {
            let host = table.get("host")?.as_str()?;
            let name = table.get("name")?.as_str()?;
            claim(launchers, &mut claimed, |launcher| {
                launcher.host == host && launcher.name == name
            })
        })
        .collect()
}

fn apply_launcher(table: &mut Table, launcher: &X11Launcher) {
    set_value(table, "host", Value::from(launcher.host.as_str()));
    set_value(table, "name", Value::from(launcher.name.as_str()));
    set_value(table, "command", Value::from(launcher.command.as_str()));
}

fn update_array_of_tables<Definition>(
    root: &mut Table,
    key: &str,
    definitions: &[Definition],
    assign: impl Fn(&ArrayOfTables, &[Definition]) -> Vec<Option<usize>>,
    apply: impl Fn(&mut Table, &Definition),
) {
    let item = root
        .entry(key)
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));
    if !item.is_array_of_tables() {
        *item = Item::ArrayOfTables(ArrayOfTables::new());
    }
    let Some(array) = item.as_array_of_tables_mut() else {
        return;
    };
    let assignments = assign(array, definitions);
    reconcile(array, definitions, &assignments, apply);
}

fn reconcile<Definition>(
    array: &mut ArrayOfTables,
    definitions: &[Definition],
    assignments: &[Option<usize>],
    apply: impl Fn(&mut Table, &Definition),
) {
    for index in (0..assignments.len()).rev() {
        if assignments[index].is_none() {
            array.remove(index);
        }
    }
    let kept: Vec<usize> = assignments.iter().flatten().copied().collect();
    for (table, definition_index) in array.iter_mut().zip(&kept) {
        apply(table, &definitions[*definition_index]);
    }
    for (index, definition) in definitions.iter().enumerate() {
        if !kept.contains(&index) {
            let mut table = new_section();
            apply(&mut table, definition);
            array.push(table);
        }
    }
}

fn new_section() -> Table {
    let mut table = Table::new();
    table.decor_mut().set_prefix(SECTION_SEPARATOR);
    table
}

fn new_settings_section() -> Table {
    let mut table = new_section();
    table.set_position(Some(FIRST_SECTION_POSITION));
    table
}

fn set_optional(table: &mut dyn TableLike, key: &str, new_value: Option<Value>) {
    match new_value {
        Some(new_value) => set_value(table, key, new_value),
        None => {
            table.remove(key);
        }
    }
}

fn set_value(table: &mut dyn TableLike, key: &str, new_value: Value) {
    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(existing) if has_same_content(existing, &new_value) => {}
        Some(existing) => {
            let decor = existing.decor().clone();
            *existing = new_value;
            *existing.decor_mut() = decor;
        }
        None => {
            table.insert(key, Item::Value(new_value));
        }
    }
}

fn has_same_content(existing: &Value, replacement: &Value) -> bool {
    match (existing, replacement) {
        (Value::String(left), Value::String(right)) => left.value() == right.value(),
        (Value::Integer(left), Value::Integer(right)) => left.value() == right.value(),
        (Value::Boolean(left), Value::Boolean(right)) => left.value() == right.value(),
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| has_same_content(left, right))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::format::{parse_document, render_document};
    use crate::store::test_support::named_tunnel;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    const COMMENTED_FILE: &str = r#"version = 1

[settings]
# bind on loopback only
default_bind_address = "127.0.0.1"
keepalive_max = 3  # drop after three misses

# my favourite tunnel
[[tunnels]]
id = "01JA0000000000000000000000"
name = "Adonis"
host = "nova"
kind = "local"
listen_port = 3333
tags = ["dev"]

# the database
[[tunnels]]
id = "01JA0000000000000000000001"
name = "Database"  # postgres
host = "nova"
kind = "local"
listen_port = 5432
tags = []
"#;

    fn path() -> &'static Path {
        Path::new("tunnels.toml")
    }

    fn load(text: &str) -> StoreDocument {
        parse_document(path(), text).unwrap().document
    }

    fn update(text: &str, document: &StoreDocument) -> String {
        render_document(path(), Some(text), document).unwrap()
    }

    #[test]
    fn should_keep_comment_above_a_tunnel_when_another_tunnel_is_updated() {
        let mut document = load(COMMENTED_FILE);
        document.tunnels[1].listen_port = 5433;

        let updated = update(COMMENTED_FILE, &document);

        assert!(
            updated.contains("# my favourite tunnel\n[[tunnels]]"),
            "{updated}"
        );
    }

    #[test]
    fn should_apply_the_change_when_a_tunnel_is_updated() {
        let mut document = load(COMMENTED_FILE);
        document.tunnels[1].listen_port = 5433;

        let updated = update(COMMENTED_FILE, &document);

        assert_eq!(load(&updated), document);
    }

    #[test]
    fn should_keep_comments_inside_settings_when_a_setting_changes() {
        let mut document = load(COMMENTED_FILE);
        document.settings.keepalive_max = 5;

        let updated = update(COMMENTED_FILE, &document);

        assert!(
            updated.contains("# bind on loopback only\ndefault_bind_address"),
            "{updated}"
        );
    }

    #[test]
    fn should_keep_trailing_comment_when_the_commented_value_changes() {
        let mut document = load(COMMENTED_FILE);
        document.settings.keepalive_max = 5;

        let updated = update(COMMENTED_FILE, &document);

        assert!(
            updated.contains("keepalive_max = 5  # drop after three misses"),
            "{updated}"
        );
    }

    #[test]
    fn should_keep_trailing_comment_when_a_tunnel_value_changes() {
        let mut document = load(COMMENTED_FILE);
        document.tunnels[1].name = "Postgres".to_owned();

        let updated = update(COMMENTED_FILE, &document);

        assert!(
            updated.contains("name = \"Postgres\"  # postgres"),
            "{updated}"
        );
    }

    #[test]
    fn should_add_missing_setting_keys_when_the_file_only_has_some() {
        let document = load(COMMENTED_FILE);

        let updated = update(COMMENTED_FILE, &document);

        assert!(
            updated.contains("reconnect_max_delay_ms = 60000"),
            "{updated}"
        );
    }

    #[test]
    fn should_append_a_tunnel_when_the_document_has_a_new_one() {
        let mut document = load(COMMENTED_FILE);
        document
            .tunnels
            .push(named_tunnel(TunnelKind::Dynamic, "Socks", 1080));

        let updated = update(COMMENTED_FILE, &document);

        assert_eq!(load(&updated), document);
    }

    #[test]
    fn should_keep_existing_text_untouched_when_a_tunnel_is_appended() {
        let mut document = load(COMMENTED_FILE);
        document
            .tunnels
            .push(named_tunnel(TunnelKind::Dynamic, "Socks", 1080));

        let updated = update(COMMENTED_FILE, &document);

        assert!(updated.contains("# the database\n[[tunnels]]"), "{updated}");
    }

    #[test]
    fn should_remove_a_tunnel_and_its_comment_when_the_document_drops_it() {
        let mut document = load(COMMENTED_FILE);
        document.tunnels.remove(0);

        let updated = update(COMMENTED_FILE, &document);

        assert!(!updated.contains("my favourite tunnel"), "{updated}");
    }

    #[test]
    fn should_keep_remaining_tunnel_when_the_first_is_removed() {
        let mut document = load(COMMENTED_FILE);
        document.tunnels.remove(0);

        let updated = update(COMMENTED_FILE, &document);

        assert_eq!(load(&updated), document);
    }

    #[test]
    fn should_remove_optional_key_when_the_value_becomes_none() {
        let text = "[[tunnels]]\nid = \"01JA0000000000000000000000\"\nname = \"A\"\nhost = \"h\"\nkind = \"local\"\nlisten_port = 1\nlisten_address = \"0.0.0.0\"\ntags = []\n";
        let mut document = load(text);
        document.tunnels[0].listen_address = None;

        let updated = update(text, &document);

        assert!(!updated.contains("listen_address"), "{updated}");
    }

    #[test]
    fn should_match_an_id_less_tunnel_by_name_and_keep_its_comment() {
        let text = "# by hand\n[[tunnels]]\nname = \"A\"\nhost = \"h\"\nkind = \"local\"\nlisten_port = 1\n";
        let document = load(text);

        let updated = update(text, &document);

        assert!(
            updated.contains("\n# by hand\n[[tunnels]]\nname = \"A\"")
                && updated.contains("id = \""),
            "{updated}"
        );
    }

    #[test]
    fn should_write_the_generated_id_when_saving_an_id_less_tunnel() {
        let text = "[[tunnels]]\nname = \"A\"\nhost = \"h\"\nkind = \"local\"\nlisten_port = 1\n";
        let document = load(text);

        let updated = update(text, &document);

        assert_eq!(load(&updated), document);
    }

    #[test]
    fn should_update_a_launcher_in_place_when_host_and_name_match() {
        let text =
            "[[x11_launchers]]\n# editor\nhost = \"nova\"\nname = \"Code\"\ncommand = \"code\"\n";
        let mut document = load(text);
        document.x11_launchers[0].command = "code --wait".to_owned();

        let updated = update(text, &document);

        assert!(updated.contains("# editor\nhost = \"nova\""), "{updated}");
        assert_eq!(load(&updated), document);
    }

    #[test]
    fn should_add_and_remove_launchers_when_the_document_changes() {
        let text = "[[x11_launchers]]\nhost = \"nova\"\nname = \"Code\"\ncommand = \"code\"\n";
        let mut document = load(text);
        document.x11_launchers = vec![X11Launcher {
            host: "nova".to_owned(),
            name: "Terminal".to_owned(),
            command: "xterm".to_owned(),
        }];

        let updated = update(text, &document);

        assert_eq!(load(&updated), document);
    }

    #[test]
    fn should_add_update_and_remove_host_overrides_when_the_document_changes() {
        let text = "[hosts.old]\nx11_display = \":1\"\n\n[hosts.kept]\nx11_display = \":2\"\n";
        let mut document = load(text);
        document.hosts.remove("old");
        document.hosts.get_mut("kept").unwrap().x11_display = Some(":3".to_owned());
        document
            .hosts
            .insert("new".to_owned(), HostOverrides::default());

        let updated = update(text, &document);

        assert_eq!(load(&updated), document);
    }

    #[test]
    fn should_rewrite_settings_when_they_were_written_as_an_inline_table() {
        let text = "settings = { keepalive_max = 9 }\n";
        let mut document = load(text);
        document.settings.keepalive_max = 4;

        let updated = update(text, &document);

        assert_eq!(load(&updated), document);
    }

    #[test]
    fn should_place_settings_before_tunnels_when_the_file_had_none() {
        let text = "[[tunnels]]\nname = \"A\"\nhost = \"h\"\nkind = \"local\"\nlisten_port = 1\n";
        let updated = update(text, &load(text));

        assert!(
            updated.find("[settings]") < updated.find("[[tunnels]]"),
            "{updated}"
        );
    }

    #[test]
    fn should_stamp_the_current_version_when_the_file_has_none() {
        let updated = update("", &StoreDocument::default());

        assert!(updated.starts_with("version = 1\n"), "{updated}");
    }
}
