use std::ops::Range;
use std::str::FromStr;

use serde::Deserialize;
use toml_edit::{DocumentMut, Item, Table, Value};

use super::error::ImportFileError;
use super::plan_types::ConfigImportSource;
use super::{EXPORT_FORMAT, EXPORT_VERSION, MAX_IMPORT_FILE_BYTES, MAX_IMPORT_TUNNELS};
use crate::devslot::{MAX_TEXT_BYTES, contains_forbidden_character, sanitize_text};
use crate::runtime::NewTunnel;
use crate::store::{CURRENT_VERSION, StoredTunnel, locate, migrate};

const FORMAT_KEY: &str = "format";
const VERSION_KEY: &str = "version";
const IMPLICIT_STORE_VERSION: i64 = 1;
const MAX_KEY_SEGMENTS: usize = 16;
/// The deepest nesting of tables, inline tables and arrays a file may have:
/// a `tunnels.toml` reaches 5 (root, `[[tunnels]]`, one tunnel, `[tunnels.open]`,
/// a value).
const MAX_DOCUMENT_DEPTH: usize = 16;
/// The stack of the thread that parses a file. Reserved lazily by the system.
const PARSER_STACK_BYTES: usize = 32 << 20;

/// The tunnels of an import file, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedImportFile {
    pub(super) source: ConfigImportSource,
    pub(super) tunnels: Vec<NewTunnel>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredExport {
    #[serde(rename = "format")]
    _format: String,
    #[serde(rename = "version")]
    _version: u32,
    #[serde(default)]
    tunnels: Vec<StoredTunnel>,
}

#[derive(Deserialize)]
struct StoredTunnelsOnly {
    #[serde(default)]
    tunnels: Vec<StoredTunnel>,
}

/// Refuses a file larger than [`MAX_IMPORT_FILE_BYTES`].
fn check_import_size(contents: &str) -> Result<(), ImportFileError> {
    if contents.len() > MAX_IMPORT_FILE_BYTES {
        return Err(ImportFileError::TooLarge {
            found: contents.len(),
            limit: MAX_IMPORT_FILE_BYTES,
        });
    }
    Ok(())
}

/// Reads the tunnels of an export, or of a raw `tunnels.toml`.
///
/// The `format` key tells the two apart: absent is a store, `tunneler-export`
/// is an export, anything else is not ours. The ids, if any, are dropped.
///
/// The work runs on a thread of its own with a large stack, and the TOML
/// document is dropped there, so that a deeply nested file cannot overflow the
/// stack of the caller. Blocking: call it from `run_blocking` in async code.
pub(crate) fn parse_import_file(contents: &str) -> Result<ParsedImportFile, ImportFileError> {
    check_import_size(contents)?;
    std::thread::scope(|scope| {
        let spawned = std::thread::Builder::new()
            .stack_size(PARSER_STACK_BYTES)
            .spawn_scoped(scope, || parse_on_this_thread(contents));
        match spawned {
            Ok(handle) => handle
                .join()
                .unwrap_or_else(|payload| std::panic::resume_unwind(payload)),
            Err(error) => Err(syntax_error(
                contents,
                None,
                &format!("cannot start the parser: {error}"),
            )),
        }
    })
}

fn parse_on_this_thread(contents: &str) -> Result<ParsedImportFile, ImportFileError> {
    reject_deep_keys(contents)?;
    let document = DocumentMut::from_str(contents)
        .map_err(|error| syntax_error(contents, error.span(), error.message()))?;
    reject_deep_document(&document)?;
    let source = detect_source(&document)?;
    let stored = match source {
        ConfigImportSource::Export => read_export(&document, contents)?,
        ConfigImportSource::Store => read_store(document, contents)?,
    };
    check_entry_count(stored.len())?;
    reject_unsafe_characters(&stored)?;
    Ok(ParsedImportFile {
        source,
        tunnels: stored.into_iter().map(into_new_tunnel).collect(),
    })
}

#[derive(Clone, Copy)]
enum Node<'document> {
    Item(&'document Item),
    Table(&'document Table),
    Value(&'document Value),
}

/// Refuses a document nested deeper than [`MAX_DOCUMENT_DEPTH`], walking it
/// with an explicit stack: nothing recurses on the shape of the file.
fn reject_deep_document(document: &DocumentMut) -> Result<(), ImportFileError> {
    let mut pending = vec![(Node::Table(document.as_table()), 1_usize)];
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_DOCUMENT_DEPTH {
            return Err(ImportFileError::Syntax {
                line: None,
                message: format!("the file is nested more than {MAX_DOCUMENT_DEPTH} levels deep"),
            });
        }
        push_children(node, depth + 1, &mut pending);
    }
    Ok(())
}

fn push_children<'document>(
    node: Node<'document>,
    depth: usize,
    pending: &mut Vec<(Node<'document>, usize)>,
) {
    match node {
        Node::Table(table) => {
            pending.extend(table.iter().map(|(_, item)| (Node::Item(item), depth)));
        }
        Node::Item(Item::Table(table)) => pending.push((Node::Table(table), depth)),
        Node::Item(Item::ArrayOfTables(tables)) => {
            pending.extend(tables.iter().map(|table| (Node::Table(table), depth)));
        }
        Node::Item(Item::Value(value)) => pending.push((Node::Value(value), depth)),
        Node::Value(Value::Array(array)) => {
            pending.extend(array.iter().map(|value| (Node::Value(value), depth)));
        }
        Node::Value(Value::InlineTable(table)) => {
            pending.extend(table.iter().map(|(_, value)| (Node::Value(value), depth)));
        }
        Node::Item(Item::None) | Node::Value(_) => {}
    }
}

fn detect_source(document: &DocumentMut) -> Result<ConfigImportSource, ImportFileError> {
    match document.get(FORMAT_KEY) {
        None => Ok(ConfigImportSource::Store),
        Some(item) if item.as_str() == Some(EXPORT_FORMAT) => Ok(ConfigImportSource::Export),
        Some(_) => Err(ImportFileError::NotTunnelerFile),
    }
}

fn read_export(
    document: &DocumentMut,
    contents: &str,
) -> Result<Vec<StoredTunnel>, ImportFileError> {
    check_version(document, EXPORT_VERSION)?;
    let stored: StoredExport = deserialize(contents)?;
    Ok(stored.tunnels)
}

fn read_store(document: DocumentMut, contents: &str) -> Result<Vec<StoredTunnel>, ImportFileError> {
    check_version(&document, CURRENT_VERSION)?;
    let version = stored_version(&document);
    let migrated = migrate(document, u32::try_from(version).unwrap_or(CURRENT_VERSION))
        .map_err(|error| syntax_error(contents, None, &error.to_string()))?
        .to_string();
    let stored: StoredTunnelsOnly = deserialize(&migrated)?;
    Ok(stored.tunnels)
}

fn stored_version(document: &DocumentMut) -> i64 {
    document
        .get(VERSION_KEY)
        .and_then(toml_edit::Item::as_integer)
        .unwrap_or(IMPLICIT_STORE_VERSION)
}

fn check_version(document: &DocumentMut, supported: u32) -> Result<(), ImportFileError> {
    let found = stored_version(document);
    if found <= i64::from(supported) {
        return Ok(());
    }
    Err(ImportFileError::NewerVersion {
        found: u32::try_from(found).unwrap_or(u32::MAX),
        supported,
    })
}

fn deserialize<Parsed: for<'text> Deserialize<'text>>(
    text: &str,
) -> Result<Parsed, ImportFileError> {
    toml_edit::de::from_str(text).map_err(|error| syntax_error(text, error.span(), error.message()))
}

fn syntax_error(text: &str, span: Option<Range<usize>>, message: &str) -> ImportFileError {
    ImportFileError::Syntax {
        line: span.map(|span| locate(text, &span).line),
        message: sanitize_text(message, MAX_TEXT_BYTES),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lexical {
    Code,
    Comment,
    BasicString,
    LiteralString,
    MultilineBasic,
    MultilineLiteral,
}

/// Refuses a line holding more than [`MAX_KEY_SEGMENTS`] key segments, before
/// any TOML parsing: the parser and the deserializers recurse once per
/// segment of a dotted key or a table header, and a file within the size cap
/// can hold over a hundred thousand of them. Dots inside strings and comments
/// do not count; any other dot on a line does, so a line is refused when in
/// doubt.
fn reject_deep_keys(contents: &str) -> Result<(), ImportFileError> {
    let bytes = contents.as_bytes();
    let mut state = Lexical::Code;
    let mut line = 1;
    let mut dots = 0;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\n' {
            line += 1;
            dots = 0;
            if !matches!(state, Lexical::MultilineBasic | Lexical::MultilineLiteral) {
                state = Lexical::Code;
            }
            index += 1;
            continue;
        }
        let rest = &bytes[index..];
        match state {
            Lexical::Code => match byte {
                b'#' => state = Lexical::Comment,
                b'"' if rest.starts_with(b"\"\"\"") => {
                    state = Lexical::MultilineBasic;
                    index += 2;
                }
                b'"' => state = Lexical::BasicString,
                b'\'' if rest.starts_with(b"'''") => {
                    state = Lexical::MultilineLiteral;
                    index += 2;
                }
                b'\'' => state = Lexical::LiteralString,
                b'.' => {
                    dots += 1;
                    if dots >= MAX_KEY_SEGMENTS {
                        return Err(ImportFileError::Syntax {
                            line: Some(line),
                            message: format!(
                                "a key or table header has more than {MAX_KEY_SEGMENTS} segments"
                            ),
                        });
                    }
                }
                _ => {}
            },
            Lexical::BasicString | Lexical::MultilineBasic if byte == b'\\' => {
                if rest.get(1).is_some_and(|next| *next != b'\n') {
                    index += 1;
                }
            }
            Lexical::BasicString if byte == b'"' => state = Lexical::Code,
            Lexical::LiteralString if byte == b'\'' => state = Lexical::Code,
            Lexical::MultilineBasic if rest.starts_with(b"\"\"\"") => {
                state = Lexical::Code;
                index += 2 + extra_quotes(&rest[3..], b'"');
            }
            Lexical::MultilineLiteral if rest.starts_with(b"'''") => {
                state = Lexical::Code;
                index += 2 + extra_quotes(&rest[3..], b'\'');
            }
            _ => {}
        }
        index += 1;
    }
    Ok(())
}

/// How many of the up to two quotes right after a closing multiline delimiter
/// belong to the content: `'''a''''` holds `a'`.
fn extra_quotes(after_delimiter: &[u8], quote: u8) -> usize {
    after_delimiter
        .iter()
        .take(2)
        .take_while(|byte| **byte == quote)
        .count()
}

fn check_entry_count(found: usize) -> Result<(), ImportFileError> {
    if found > MAX_IMPORT_TUNNELS {
        return Err(ImportFileError::TooManyEntries {
            found,
            limit: MAX_IMPORT_TUNNELS,
        });
    }
    Ok(())
}

fn reject_unsafe_characters(stored: &[StoredTunnel]) -> Result<(), ImportFileError> {
    for (entry, tunnel) in stored.iter().enumerate() {
        let unsafe_field = tunnel
            .text_fields()
            .into_iter()
            .find(|(_, text)| contains_forbidden_character(text));
        if let Some((field, _)) = unsafe_field {
            return Err(ImportFileError::UnsafeCharacters { entry, field });
        }
    }
    Ok(())
}

fn into_new_tunnel(stored: StoredTunnel) -> NewTunnel {
    let definition = stored.into_definition();
    NewTunnel {
        host: definition.host,
        kind: definition.kind,
        listen_port: definition.listen_port,
        listen_address: definition.listen_address,
        target_host: definition.target_host,
        target_port: definition.target_port,
        tags: definition.tags,
        open: definition.open,
        standby: definition.standby,
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::store::TunnelKind;

    const EXPORT: &str = "format = \"tunneler-export\"\nversion = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3333\ntags = [\"dev\"]\n\n[tunnels.open]\npath = \"/admin\"\n";

    fn parse(contents: &str) -> Result<ParsedImportFile, ImportFileError> {
        parse_import_file(contents)
    }

    fn entry_with(field_line: &str) -> String {
        format!(
            "format = \"tunneler-export\"\nversion = 1\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3333\n{field_line}\n"
        )
    }

    #[test]
    fn should_read_the_tunnels_when_the_file_is_an_export() {
        let parsed = parse(EXPORT).unwrap();

        assert_eq!(parsed.source, ConfigImportSource::Export);
        assert_eq!(parsed.tunnels.len(), 1);
        assert_eq!(parsed.tunnels[0].host, "nova");
        assert_eq!(parsed.tunnels[0].kind, TunnelKind::Local);
        assert_eq!(parsed.tunnels[0].tags, vec!["dev".to_owned()]);
        assert_eq!(parsed.tunnels[0].open.path.as_deref(), Some("/admin"));
    }

    #[test]
    fn should_read_only_the_tunnels_when_the_file_is_a_raw_store() {
        let contents = "version = 1\n\n[settings]\ndefault_bind_address = \"0.0.0.0\"\nnot_a_setting = 1\n\n[hosts.nova]\nx11_display = \":1\"\n\n[[openers]]\nid = \"viewer\"\nname = \"Viewer\"\ncommand = \"viewer\"\nargs = []\n\n[[x11_launchers]]\nhost = \"nova\"\nname = \"term\"\ncommand = \"xterm\"\n\n[[tunnels]]\nid = \"01JABCDEFGHJKMNPQRSTVWXYZ0\"\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3333\n";

        let parsed = parse(contents).unwrap();

        assert_eq!(parsed.source, ConfigImportSource::Store);
        assert_eq!(parsed.tunnels.len(), 1);
        assert_eq!(parsed.tunnels[0].listen_port, 3333);
    }

    #[test]
    fn should_accept_a_file_without_tunnels() {
        let parsed = parse("format = \"tunneler-export\"\nversion = 1\n").unwrap();

        assert_eq!(parsed.tunnels, Vec::new());
    }

    #[test]
    fn should_ignore_the_id_of_a_tunnel_table_of_an_export() {
        let contents = entry_with("id = \"01JABCDEFGHJKMNPQRSTVWXYZ0\"");

        assert!(parse(&contents).is_ok());
    }

    #[test]
    fn should_report_the_line_when_the_toml_is_broken() {
        let outcome = parse("format = \"tunneler-export\"\nversion = \n");

        assert!(matches!(
            outcome,
            Err(ImportFileError::Syntax { line: Some(2), .. })
        ));
    }

    #[test]
    fn should_report_a_syntax_error_when_an_export_has_an_unknown_key() {
        let contents = entry_with("lisen_port = 1");

        assert!(matches!(
            parse(&contents),
            Err(ImportFileError::Syntax { .. })
        ));
    }

    #[test]
    fn should_report_a_syntax_error_when_the_export_has_an_unknown_top_level_key() {
        let contents = "format = \"tunneler-export\"\nversion = 1\n[settings]\nstandby = true\n";

        assert!(matches!(
            parse(contents),
            Err(ImportFileError::Syntax { .. })
        ));
    }

    #[test]
    fn should_report_a_syntax_error_when_the_export_has_no_version() {
        assert!(matches!(
            parse("format = \"tunneler-export\"\n"),
            Err(ImportFileError::Syntax { .. })
        ));
    }

    #[test]
    fn should_not_echo_control_characters_when_the_syntax_error_names_a_key() {
        let contents = "format = \"tunneler-export\"\nversion = 1\n[[tunnels]]\nhost = \"a\"\nkind = \"local\"\nlisten_port = 1\n\"k\\u001b[31m\" = 1\n";

        let Err(ImportFileError::Syntax { message, .. }) = parse(contents) else {
            panic!("expected a syntax error");
        };

        assert!(!message.contains('\u{1b}'));
    }

    #[test]
    fn should_refuse_the_file_when_the_format_is_another_value() {
        assert_eq!(
            parse("format = \"something-else\"\nversion = 1\n"),
            Err(ImportFileError::NotTunnelerFile)
        );
    }

    #[test]
    fn should_refuse_the_file_when_the_format_is_not_a_string() {
        assert_eq!(parse("format = 3\n"), Err(ImportFileError::NotTunnelerFile));
    }

    #[test]
    fn should_report_a_newer_version_when_the_export_is_newer() {
        assert_eq!(
            parse("format = \"tunneler-export\"\nversion = 2\nfuture = true\n"),
            Err(ImportFileError::NewerVersion {
                found: 2,
                supported: 1
            })
        );
    }

    #[test]
    fn should_report_a_newer_version_when_the_store_is_newer() {
        assert_eq!(
            parse("version = 7\n[[tunnels]]\nfuture = true\n"),
            Err(ImportFileError::NewerVersion {
                found: 7,
                supported: CURRENT_VERSION
            })
        );
    }

    #[test]
    fn should_refuse_the_whole_file_when_a_field_holds_a_control_character() {
        let contents = entry_with("target_host = \"db\\u0007\"");

        assert_eq!(
            parse(&contents),
            Err(ImportFileError::UnsafeCharacters {
                entry: 0,
                field: "target_host"
            })
        );
    }

    #[test]
    fn should_name_the_entry_and_the_field_when_a_tag_holds_a_newline() {
        let second = "\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 4444\ntags = [\"ok\", \"bad\\ntag\"]\n";
        let contents = format!("{}{second}", entry_with(""));

        assert_eq!(
            parse(&contents),
            Err(ImportFileError::UnsafeCharacters {
                entry: 1,
                field: "tags"
            })
        );
    }

    #[test]
    fn should_refuse_every_bidirectional_control_in_a_text_field() {
        let characters = [
            '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}',
            '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{061C}',
        ];

        for character in characters {
            let contents = entry_with(&format!("target_host = \"a{character}b\""));
            assert!(
                matches!(
                    parse(&contents),
                    Err(ImportFileError::UnsafeCharacters { .. })
                ),
                "U+{:04X} was accepted",
                u32::from(character)
            );
        }
    }

    #[test]
    fn should_refuse_a_tab_inside_a_value() {
        let contents = entry_with("open = { path = \"/a\\tb\" }");

        assert_eq!(
            parse(&contents),
            Err(ImportFileError::UnsafeCharacters {
                entry: 0,
                field: "open.path"
            })
        );
    }

    #[test]
    fn should_refuse_the_file_when_it_is_over_the_size_cap() {
        let contents = format!("# {}\n", "a".repeat(MAX_IMPORT_FILE_BYTES));

        assert_eq!(
            parse(&contents),
            Err(ImportFileError::TooLarge {
                found: contents.len(),
                limit: MAX_IMPORT_FILE_BYTES
            })
        );
    }

    #[test]
    fn should_accept_a_file_exactly_at_the_size_cap() {
        let header = "format = \"tunneler-export\"\nversion = 1\n# ";
        let contents = format!(
            "{header}{}",
            "a".repeat(MAX_IMPORT_FILE_BYTES - header.len())
        );

        assert_eq!(contents.len(), MAX_IMPORT_FILE_BYTES);
        assert!(parse(&contents).is_ok());
    }

    #[test]
    fn should_refuse_the_file_when_it_holds_more_tunnels_than_the_cap() {
        let table = "[[tunnels]]\nhost = \"n\"\nkind = \"local\"\nlisten_port = 1\n";
        let contents = format!(
            "format = \"tunneler-export\"\nversion = 1\n{}",
            table.repeat(MAX_IMPORT_TUNNELS + 1)
        );

        assert_eq!(
            parse(&contents),
            Err(ImportFileError::TooManyEntries {
                found: MAX_IMPORT_TUNNELS + 1,
                limit: MAX_IMPORT_TUNNELS
            })
        );
    }

    fn parse_on_a_small_stack(contents: String) -> Result<ParsedImportFile, ImportFileError> {
        std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || parse_import_file(&contents))
            .unwrap()
            .join()
            .unwrap()
    }

    fn deep_key(prefix: &str, segments: usize) -> String {
        format!("{prefix}{}", vec!["a"; segments].join("."))
    }

    #[test]
    fn should_refuse_a_deeply_dotted_key_without_overflowing_the_stack() {
        let contents = format!("{} = 1\n", deep_key("", 130_000));

        assert!(matches!(
            parse_on_a_small_stack(contents),
            Err(ImportFileError::Syntax { line: Some(1), .. })
        ));
    }

    #[test]
    fn should_refuse_a_deeply_dotted_key_of_a_tunnel_without_overflowing_the_stack() {
        let contents = format!(
            "format = \"tunneler-export\"\nversion = 1\n[[tunnels]]\n{} = 1\n",
            deep_key("name.", 125_000)
        );

        assert!(matches!(
            parse_on_a_small_stack(contents),
            Err(ImportFileError::Syntax { line: Some(4), .. })
        ));
    }

    #[test]
    fn should_refuse_a_deep_table_header_without_overflowing_the_stack() {
        let contents = format!("[{}]\n", deep_key("", 130_000));

        assert!(matches!(
            parse_on_a_small_stack(contents),
            Err(ImportFileError::Syntax { line: Some(1), .. })
        ));
    }

    #[test]
    fn should_accept_a_key_path_at_the_segment_limit_and_refuse_one_more() {
        assert_eq!(reject_deep_keys(&deep_key("", MAX_KEY_SEGMENTS)), Ok(()));
        assert!(reject_deep_keys(&deep_key("", MAX_KEY_SEGMENTS + 1)).is_err());
    }

    #[test]
    fn should_not_count_the_dots_of_strings_and_comments() {
        let dots = ".".repeat(100);
        let contents = format!(
            "a = \"{dots}\"\nb = '{dots}'\n\"{dots}\" = 1\n# {dots}\nc = \"\"\"\n{dots}\n\"\"\" # {dots}\nd = '''\n{dots}\n'''\ne = \"\\\"{dots}\"\n"
        );

        assert_eq!(reject_deep_keys(&contents), Ok(()));
    }

    #[test]
    fn should_count_dots_again_after_a_string_ends() {
        let contents = format!("a = \"x\" {}\n", ".".repeat(20));

        assert!(reject_deep_keys(&contents).is_err());
    }

    #[test]
    fn should_still_parse_a_normal_export_and_a_normal_store_file() {
        let store = "version = 1\n\n[settings]\ndefault_bind_address = \"127.0.0.1\"\n\n[[tunnels]]\nhost = \"db.example.com\"\nkind = \"local\"\nlisten_port = 3333\ntarget_host = \"10.0.0.1\"\n\n[tunnels.open]\npath = \"/a.b.c\"\n";

        assert!(parse_on_a_small_stack(EXPORT.to_owned()).is_ok());
        assert!(parse_on_a_small_stack(store.to_owned()).is_ok());
    }

    fn nested_inline(levels: usize, key_path: &str) -> String {
        let mut text = "1".to_owned();
        for _ in 0..levels {
            text = format!("{{ {key_path} = {text} }}");
        }
        text
    }

    fn assert_syntax_error_on_a_small_stack(contents: String) {
        assert!(matches!(
            parse_on_a_small_stack(contents),
            Err(ImportFileError::Syntax { .. })
        ));
    }

    #[test]
    fn should_not_let_extra_quotes_before_a_closing_delimiter_hide_the_dots() {
        let dots = vec!["a"; 17].join(".");

        assert!(reject_deep_keys(&format!("x = {{ s = '''a'''', {dots} = 1 }}\n")).is_err());
        assert!(reject_deep_keys(&format!("x = {{ s = '''a''''', {dots} = 1 }}\n")).is_err());
        assert!(reject_deep_keys(&format!("x = {{ s = \"\"\"a\"\"\"\", {dots} = 1 }}\n")).is_err());
        assert!(
            reject_deep_keys(&format!("x = {{ s = \"\"\"a\"\"\"\"\", {dots} = 1 }}\n")).is_err()
        );
    }

    #[test]
    fn should_refuse_the_nested_payload_that_hides_its_dots_behind_extra_quotes() {
        let path = vec!["a"; 70].join(".");
        let body = nested_inline(75, &path);
        let contents = format!("x = {{ s = '''a'''', {path} = {body} }}\n");

        assert_syntax_error_on_a_small_stack(contents);
    }

    #[test]
    fn should_refuse_the_same_payload_with_basic_multiline_strings() {
        let path = vec!["a"; 70].join(".");
        let body = nested_inline(75, &path);
        let contents = format!("x = {{ s = \"\"\"a\"\"\"\", {path} = {body} }}\n");

        assert_syntax_error_on_a_small_stack(contents);
    }

    #[test]
    fn should_refuse_deep_inline_nesting_alone_within_the_parser_limit() {
        assert_syntax_error_on_a_small_stack(format!("x = {}\n", nested_inline(70, "a")));
    }

    #[test]
    fn should_refuse_array_of_tables_headers_followed_by_inline_nesting() {
        let header = vec!["a"; 15].join(".");
        let contents = format!("[[{header}]]\nx = {}\n", nested_inline(60, "a"));

        assert_syntax_error_on_a_small_stack(contents);
    }

    #[test]
    fn should_refuse_deep_arrays() {
        let contents = format!("x = {}1{}\n", "[".repeat(70), "]".repeat(70));

        assert_syntax_error_on_a_small_stack(contents);
    }

    #[test]
    fn should_parse_a_store_with_settings_openers_and_open_options() {
        let store = "version = 1\n\n[settings]\nstart_on_launch_tags = [\"boot\"]\n\n[hosts.nova]\nx11_display = \":1\"\n\n[[openers]]\nid = \"viewer\"\nname = \"Viewer\"\ncommand = \"viewer\"\nargs = [\"{url}\", \"--x\"]\nports = [80, 443]\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3333\ntags = [\"a\", \"b\"]\n\n[tunnels.open]\npath = \"/admin\"\n\n[[tunnels]]\nhost = \"nova\"\nkind = \"local\"\nlisten_port = 3334\nopen = { path = \"/x\", with = \"viewer\" }\n";

        let parsed = parse_on_a_small_stack(store.to_owned()).unwrap();

        assert_eq!(parsed.tunnels.len(), 2);
        assert_eq!(parsed.tunnels[1].open.with.as_deref(), Some("viewer"));
    }

    #[test]
    fn should_accept_a_document_exactly_at_the_depth_limit() {
        let at_limit = format!("x = {}\n", nested_inline(MAX_DOCUMENT_DEPTH - 3, "a"));
        let over_limit = format!("x = {}\n", nested_inline(MAX_DOCUMENT_DEPTH, "a"));
        let depth_of = |text: &str| reject_deep_document(&DocumentMut::from_str(text).unwrap());

        assert_eq!(depth_of(&at_limit), Ok(()));
        assert!(depth_of(&over_limit).is_err());
    }

    #[test]
    fn should_accept_exactly_the_most_tunnels() {
        let table = "[[tunnels]]\nhost = \"n\"\nkind = \"local\"\nlisten_port = 1\n";
        let contents = format!(
            "format = \"tunneler-export\"\nversion = 1\n{}",
            table.repeat(MAX_IMPORT_TUNNELS)
        );

        assert_eq!(parse(&contents).unwrap().tunnels.len(), MAX_IMPORT_TUNNELS);
    }
}
