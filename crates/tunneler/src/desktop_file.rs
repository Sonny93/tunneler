//! What the desktop entries Tunneler writes on XDG desktops have in common:
//! the `Exec` quoting of the Desktop Entry specification, the ownership line
//! that tells a file Tunneler wrote from someone else's, and an atomic write.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::file_change::FileChange;

const EXEC_RESERVED_CHARACTERS: &str = " \t\n\"'\\><~|&;$*?#()`";

/// Whether `content` carries `ownership_line` on a line of its own: the mark
/// of a file Tunneler wrote.
pub(crate) fn has_ownership_line(content: &str, ownership_line: &str) -> bool {
    content.lines().any(|line| line == ownership_line)
}

/// Reads a file as text, `None` when it does not exist.
pub(crate) fn read_if_exists(path: &Path) -> io::Result<Option<String>> {
    match fs::read(path) {
        Ok(content) => Ok(Some(String::from_utf8_lossy(&content).into_owned())),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(source),
    }
}

/// Writes `content` to `path` so that a reader sees the old file or the new
/// one, never half of it, creating the parent directory when needed.
pub(crate) fn write_atomically(path: &Path, content: &[u8]) -> io::Result<()> {
    let directory = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(directory)?;
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let temporary = directory.join(format!(".{file_name}.{}.tmp", std::process::id()));
    write_synced(&temporary, content)?;
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

fn write_synced(path: &Path, content: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content)?;
    file.sync_all()
}

/// Writes `content` to `path` like [`write_atomically`], unless the file
/// already holds exactly that, in which case it is not touched.
pub(crate) fn write_if_changed(path: &Path, content: &[u8]) -> io::Result<FileChange> {
    match fs::read(path) {
        Ok(existing) if existing == content => return Ok(FileChange::Unchanged),
        Ok(_) => {}
        Err(source) if source.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(source),
    }
    write_atomically(path, content)?;
    Ok(FileChange::Written)
}

/// Quotes one argument of an `Exec` key: arguments with reserved characters
/// go between double quotes, `"`, `` ` ``, `$` and `\` are escaped inside them,
/// every backslash is doubled again by the string syntax of the file, and `%`
/// is doubled so that it is not read as a field code.
pub(crate) fn quote_exec_argument(argument: &str) -> String {
    let escaped: String = argument.chars().map(escape_exec_character).collect();
    if argument.contains(|character| EXEC_RESERVED_CHARACTERS.contains(character)) {
        return format!("\"{escaped}\"");
    }
    escaped
}

fn escape_exec_character(character: char) -> String {
    match character {
        '"' | '`' | '$' => format!("\\\\{character}"),
        '\\' => "\\\\\\\\".to_owned(),
        '%' => "%%".to_owned(),
        other => other.to_string(),
    }
}

/// Escapes a value of type `string` (`TryExec`, `Name`...): the backslash and
/// the control characters that would end the line.
pub(crate) fn escape_string_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\\' => "\\\\".to_owned(),
            '\n' => "\\n".to_owned(),
            '\r' => "\\r".to_owned(),
            '\t' => "\\t".to_owned(),
            other => other.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn should_not_quote_a_plain_path() {
        assert_eq!(
            quote_exec_argument("/usr/bin/tunneler"),
            "/usr/bin/tunneler"
        );
    }

    #[test]
    fn should_quote_a_path_with_spaces() {
        assert_eq!(
            quote_exec_argument("/opt/my apps/tunneler"),
            "\"/opt/my apps/tunneler\""
        );
    }

    #[test]
    fn should_escape_double_quotes_in_a_quoted_path() {
        assert_eq!(quote_exec_argument("/a\"b"), "\"/a\\\\\"b\"");
    }

    #[test]
    fn should_quote_a_path_with_single_quotes() {
        assert_eq!(quote_exec_argument("/it's"), "\"/it's\"");
    }

    #[test]
    fn should_escape_dollars_and_backticks_in_a_quoted_path() {
        assert_eq!(
            quote_exec_argument("/$HOME/`x`"),
            "\"/\\\\$HOME/\\\\`x\\\\`\""
        );
    }

    #[test]
    fn should_quadruple_backslashes_in_a_quoted_path() {
        assert_eq!(quote_exec_argument("/a\\b"), "\"/a\\\\\\\\b\"");
    }

    #[test]
    fn should_double_percent_signs() {
        assert_eq!(quote_exec_argument("/100%/tunneler"), "/100%%/tunneler");
    }

    #[test]
    fn should_escape_backslashes_and_line_breaks_in_a_string_value() {
        assert_eq!(escape_string_value("/a\\b\nc"), "/a\\\\b\\nc");
    }

    #[test]
    fn should_recognise_the_ownership_line_when_it_stands_alone_on_a_line() {
        assert!(has_ownership_line(
            "[Desktop Entry]\nX-Mark=true\n",
            "X-Mark=true"
        ));
    }

    #[test]
    fn should_not_recognise_the_ownership_line_when_it_is_only_part_of_a_line() {
        assert!(!has_ownership_line("Comment=X-Mark=true\n", "X-Mark=true"));
    }

    #[test]
    fn should_leave_no_temporary_file_when_writing_atomically() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/entry.desktop");

        write_atomically(&path, b"content").unwrap();

        let names: Vec<_> = fs::read_dir(directory.path().join("nested"))
            .unwrap()
            .map(|file| file.unwrap().file_name())
            .collect();
        assert_eq!(names, vec!["entry.desktop"]);
    }

    #[test]
    fn should_report_the_file_as_written_when_it_is_missing_or_different() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("entry.desktop");

        let created = write_if_changed(&path, b"one").unwrap();
        let replaced = write_if_changed(&path, b"two").unwrap();

        assert_eq!(created, FileChange::Written);
        assert_eq!(replaced, FileChange::Written);
        assert_eq!(fs::read(&path).unwrap(), b"two");
    }

    #[test]
    fn should_report_the_file_as_unchanged_when_it_already_has_the_content() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("entry.desktop");
        write_if_changed(&path, b"one").unwrap();

        assert_eq!(
            write_if_changed(&path, b"one").unwrap(),
            FileChange::Unchanged
        );
    }
}
