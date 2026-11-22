//! The pure parts of the export and import file commands: the bounded read
//! and the file name proposed in the save dialog.

use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::Path;

use ipc::protocol::MAX_CONFIG_IMPORT_BYTES;

use crate::bridge_error::BridgeError;

/// Largest import file accepted: the same cap as the daemon's.
pub const MAX_IMPORT_FILE_BYTES: u64 = MAX_CONFIG_IMPORT_BYTES as u64;

#[cfg(unix)]
const OWNER_ONLY_FILE_MODE: u32 = 0o600;
const DEFAULT_EXPORT_FILE_NAME: &str = "tunneler-export.toml";
const EXPORT_FILE_EXTENSION: &str = ".toml";
const FORBIDDEN_FILE_NAME_CHARACTERS: &[char] = &['/', '\\', '<', '>', ':', '"', '|', '?', '*'];

/// Reads at most `limit` bytes of UTF-8 text. One byte more is read to tell a
/// file of exactly `limit` bytes from a longer one, so an unbounded file or
/// stream is never loaded into memory.
pub fn read_bounded_text(reader: impl Read, limit: u64) -> Result<String, BridgeError> {
    let mut bytes = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| BridgeError::internal(&error.to_string()))?;
    if bytes.len() as u64 > limit {
        return Err(BridgeError::import_file_too_large(limit));
    }
    let text = String::from_utf8(bytes).map_err(|_| BridgeError::import_file_not_text())?;
    if text.chars().any(is_unsafe_control_character) {
        return Err(BridgeError::import_file_unsafe_characters());
    }
    Ok(text)
}

/// A C0 control character other than tab, line feed and carriage return, or DEL.
fn is_unsafe_control_character(character: char) -> bool {
    let is_allowed_whitespace = matches!(character, '\t' | '\n' | '\r');
    character.is_ascii_control() && !is_allowed_whitespace
}

/// Writes `contents` to `path`, replacing the file. A file created here is
/// readable by its owner only on Unix, like the one the command line writes; an
/// existing file keeps its mode, and Windows uses its default access rules.
pub fn write_private_file(path: &Path, contents: &str) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    restrict_to_owner(&mut options);
    options.open(path)?.write_all(contents.as_bytes())
}

#[cfg(unix)]
fn restrict_to_owner(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(OWNER_ONLY_FILE_MODE);
}

#[cfg(not(unix))]
fn restrict_to_owner(_options: &mut OpenOptions) {}

/// The name proposed in the save dialog: the front end only suggests a name,
/// never a location, so anything that looks like a path or a control character
/// is dropped and the extension is forced to `.toml`.
pub fn export_file_name(suggested: &str) -> String {
    let cleaned: String = suggested
        .chars()
        .filter(|character| {
            !character.is_control() && !FORBIDDEN_FILE_NAME_CHARACTERS.contains(character)
        })
        .collect();
    let trimmed = cleaned.trim().trim_start_matches('.');
    if trimmed.is_empty() {
        return DEFAULT_EXPORT_FILE_NAME.to_owned();
    }
    if trimmed.to_lowercase().ends_with(EXPORT_FILE_EXTENSION) {
        return trimmed.to_owned();
    }
    format!("{trimmed}{EXPORT_FILE_EXTENSION}")
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn should_read_the_text_when_it_is_exactly_at_the_limit() {
        let text = read_bounded_text(Cursor::new(b"abcd".to_vec()), 4).unwrap();

        assert_eq!(text, "abcd");
    }

    #[test]
    fn should_reject_the_file_when_it_is_one_byte_over_the_limit() {
        let error = read_bounded_text(Cursor::new(b"abcde".to_vec()), 4).unwrap_err();

        assert_eq!(error, BridgeError::import_file_too_large(4));
    }

    #[test]
    fn should_not_read_past_the_limit_plus_one_byte() {
        let mut reader = Cursor::new(vec![b'a'; 100]);

        let error = read_bounded_text(&mut reader, 10).unwrap_err();

        assert_eq!(error, BridgeError::import_file_too_large(10));
        assert_eq!(reader.position(), 11);
    }

    #[test]
    fn should_reject_the_file_when_it_is_not_utf8() {
        let error = read_bounded_text(Cursor::new(vec![0xff, 0xfe]), 10).unwrap_err();

        assert_eq!(error, BridgeError::import_file_not_text());
    }

    #[test]
    fn should_reject_the_file_when_it_holds_a_control_character() {
        for unsafe_text in ["a\u{0}b", "a\u{1b}[31m", "a\u{7f}b", "a\u{8}b"] {
            let error =
                read_bounded_text(Cursor::new(unsafe_text.as_bytes().to_vec()), 100).unwrap_err();

            assert_eq!(error, BridgeError::import_file_unsafe_characters());
        }
    }

    #[test]
    fn should_keep_tabs_and_line_breaks() {
        let text = read_bounded_text(Cursor::new(b"a\tb\r\nc\n".to_vec()), 100).unwrap();

        assert_eq!(text, "a\tb\r\nc\n");
    }

    #[cfg(unix)]
    #[test]
    fn should_create_the_export_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let directory =
            std::env::temp_dir().join(format!("tunneler-export-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("export.toml");
        let _ = std::fs::remove_file(&path);

        write_private_file(&path, "format = \"tunneler-export\"\n").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "format = \"tunneler-export\"\n"
        );
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn should_replace_the_content_of_an_existing_file() {
        let directory =
            std::env::temp_dir().join(format!("tunneler-replace-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("export.toml");
        std::fs::write(&path, "a much longer previous content").unwrap();

        write_private_file(&path, "new").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn should_keep_a_plain_toml_name() {
        assert_eq!(export_file_name("tunnels.toml"), "tunnels.toml");
    }

    #[test]
    fn should_add_the_extension_when_it_is_missing() {
        assert_eq!(export_file_name("tunnels"), "tunnels.toml");
    }

    #[test]
    fn should_drop_path_separators_and_control_characters() {
        assert_eq!(export_file_name("../etc/pass\nwd"), "etcpasswd.toml");
        assert_eq!(export_file_name("C:\\Users\\me\\x.toml"), "CUsersmex.toml");
    }

    #[test]
    fn should_not_propose_a_hidden_file() {
        assert_eq!(export_file_name("..bashrc"), "bashrc.toml");
    }

    #[test]
    fn should_fall_back_to_the_default_name_when_nothing_is_left() {
        assert_eq!(export_file_name(" /\\ "), "tunneler-export.toml");
        assert_eq!(export_file_name(""), "tunneler-export.toml");
    }
}
