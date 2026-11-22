//! Reading the file to import: the whole of it, once, and never more than the
//! daemon accepts.

use std::fs::File;
use std::io::{self, Read};

use ipc::protocol::MAX_CONFIG_IMPORT_BYTES;

use super::error::CliError;

const STANDARD_INPUT_ARGUMENT: &str = "-";
const STANDARD_INPUT_NAME: &str = "the standard input";

/// The text of the file named by the argument, or of the standard input for
/// `-`.
///
/// # Errors
///
/// Fails when the file cannot be read, holds more than
/// [`MAX_CONFIG_IMPORT_BYTES`] bytes, or is not UTF-8 text.
pub fn read_import_source(argument: &str) -> Result<String, CliError> {
    if argument == STANDARD_INPUT_ARGUMENT {
        return read_bounded(
            io::stdin().lock(),
            STANDARD_INPUT_NAME,
            MAX_CONFIG_IMPORT_BYTES,
        );
    }
    let file = File::open(argument).map_err(|source| CliError::ImportUnreadable {
        source_name: argument.to_owned(),
        source,
    })?;
    read_bounded(file, argument, MAX_CONFIG_IMPORT_BYTES)
}

/// Reads at most `limit_bytes` bytes as text; a source with one byte more is
/// refused without reading the rest.
fn read_bounded(
    reader: impl Read,
    source_name: &str,
    limit_bytes: usize,
) -> Result<String, CliError> {
    let read_cap = u64::try_from(limit_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .take(read_cap)
        .read_to_end(&mut bytes)
        .map_err(|source| CliError::ImportUnreadable {
            source_name: source_name.to_owned(),
            source,
        })?;
    if bytes.len() > limit_bytes {
        return Err(CliError::ImportTooLarge {
            source_name: source_name.to_owned(),
            limit_bytes,
        });
    }
    let text = String::from_utf8(bytes).map_err(|_| CliError::ImportNotText {
        source_name: source_name.to_owned(),
    })?;
    if text.chars().any(is_forbidden_control_character) {
        return Err(CliError::ImportHasControlCharacters {
            source_name: source_name.to_owned(),
        });
    }
    Ok(text)
}

/// A C0 control character other than tab, line feed and carriage return, or
/// DEL: TOML forbids them, and they would inflate when escaped in JSON.
fn is_forbidden_control_character(character: char) -> bool {
    character.is_control() && !matches!(character, '\t' | '\n' | '\r') && character <= '\u{7f}'
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    const SMALL_LIMIT: usize = 8;

    #[test]
    fn should_read_the_whole_text_when_it_is_exactly_at_the_limit() {
        let reader = Cursor::new(b"12345678".to_vec());

        assert_eq!(
            read_bounded(reader, "backup.toml", SMALL_LIMIT).unwrap(),
            "12345678"
        );
    }

    #[test]
    fn should_refuse_the_text_when_it_is_one_byte_over_the_limit() {
        let reader = Cursor::new(b"123456789".to_vec());

        let outcome = read_bounded(reader, "backup.toml", SMALL_LIMIT);

        assert!(matches!(
            outcome,
            Err(CliError::ImportTooLarge {
                limit_bytes: SMALL_LIMIT,
                ..
            })
        ));
    }

    #[test]
    fn should_refuse_an_endless_source_without_reading_it_all() {
        let outcome = read_bounded(io::repeat(b'a'), "the standard input", SMALL_LIMIT);

        assert!(matches!(outcome, Err(CliError::ImportTooLarge { .. })));
    }

    #[test]
    fn should_refuse_a_control_character_but_accept_tab_and_line_endings() {
        let refused = read_bounded(Cursor::new(b"a\x07b".to_vec()), "f", SMALL_LIMIT);
        let deleted = read_bounded(Cursor::new(b"a\x7fb".to_vec()), "f", SMALL_LIMIT);
        let allowed = read_bounded(Cursor::new(b"a\tb\r\n".to_vec()), "f", SMALL_LIMIT);

        assert!(matches!(
            refused,
            Err(CliError::ImportHasControlCharacters { .. })
        ));
        assert!(matches!(
            deleted,
            Err(CliError::ImportHasControlCharacters { .. })
        ));
        assert_eq!(allowed.unwrap(), "a\tb\r\n");
    }

    #[test]
    fn should_refuse_bytes_that_are_not_utf8_text() {
        let reader = Cursor::new(vec![0xff, 0xfe, 0x00]);

        let outcome = read_bounded(reader, "backup.toml", SMALL_LIMIT);

        assert!(matches!(outcome, Err(CliError::ImportNotText { .. })));
    }

    #[test]
    fn should_say_the_file_cannot_be_read_when_it_does_not_exist() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.toml");

        let outcome = read_import_source(&missing.display().to_string());

        assert!(matches!(outcome, Err(CliError::ImportUnreadable { .. })));
    }

    #[test]
    fn should_read_the_file_when_the_argument_names_one() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("backup.toml");
        std::fs::write(&path, "format = \"tunneler-export\"\n").unwrap();

        let text = read_import_source(&path.display().to_string()).unwrap();

        assert_eq!(text, "format = \"tunneler-export\"\n");
    }

    #[test]
    fn should_refuse_a_file_over_the_cap_of_the_daemon() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("big.toml");
        std::fs::write(&path, "a".repeat(MAX_CONFIG_IMPORT_BYTES + 1)).unwrap();

        let outcome = read_import_source(&path.display().to_string());

        assert!(matches!(outcome, Err(CliError::ImportTooLarge { .. })));
    }
}
