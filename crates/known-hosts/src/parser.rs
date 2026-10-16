use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;

use crate::entry::{Entry, parse_line};
use crate::error::{Error, Warning};

const LINE_FEED: u8 = b'\n';
const CARRIAGE_RETURN: char = '\r';

/// The valid entries of a file and the warnings for the lines skipped.
#[derive(Debug, Default)]
pub(crate) struct ParsedFile {
    pub(crate) entries: Vec<Entry>,
    pub(crate) warnings: Vec<Warning>,
}

/// Reads and parses a file; a missing file reads as empty.
pub(crate) fn read_file(path: &Path) -> Result<ParsedFile, Error> {
    match fs::read(path) {
        Ok(content) => Ok(parse_content(path, &content)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(ParsedFile::default()),
        Err(source) => Err(Error::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

pub(crate) fn parse_content(path: &Path, content: &[u8]) -> ParsedFile {
    let shared_path: Arc<Path> = Arc::from(path);
    let mut parsed = ParsedFile::default();
    for (index, raw_line) in content.split(|byte| *byte == LINE_FEED).enumerate() {
        let line_number = index + 1;
        let text = String::from_utf8_lossy(raw_line);
        match parse_line(text.trim_end_matches(CARRIAGE_RETURN)) {
            Ok(None) => {}
            Ok(Some(parsed_line)) => parsed.entries.push(Entry {
                kind: parsed_line.kind,
                patterns: parsed_line.patterns,
                key: parsed_line.key,
                path: Arc::clone(&shared_path),
                line: line_number,
            }),
            Err(kind) => parsed.warnings.push(Warning {
                path: path.to_path_buf(),
                line: line_number,
                kind,
            }),
        }
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::WarningKind;
    use crate::key::tests::key_for;
    use pretty_assertions::assert_eq;

    fn valid_line(host: &str) -> String {
        format!(
            "{host} ssh-ed25519 {}",
            key_for("ssh-ed25519", b"k").to_base64()
        )
    }

    #[test]
    fn should_number_entries_by_line_when_file_has_comments_and_blank_lines() {
        let content = format!("# header\n\n{}\n", valid_line("host"));

        let parsed = parse_content(Path::new("known_hosts"), content.as_bytes());

        assert_eq!(parsed.entries[0].line, 3);
    }

    #[test]
    fn should_keep_valid_lines_when_another_line_is_malformed() {
        let content = format!("garbage\n{}\n", valid_line("host"));

        let parsed = parse_content(Path::new("known_hosts"), content.as_bytes());

        assert_eq!(parsed.entries.len(), 1);
    }

    #[test]
    fn should_warn_with_path_and_line_when_a_line_is_malformed() {
        let content = format!("{}\nhost ssh-ed25519\n", valid_line("host"));

        let parsed = parse_content(Path::new("known_hosts"), content.as_bytes());

        assert_eq!(
            parsed.warnings,
            vec![Warning {
                path: "known_hosts".into(),
                line: 2,
                kind: WarningKind::MissingField { field: "key" },
            }]
        );
    }

    #[test]
    fn should_parse_every_line_when_file_uses_crlf() {
        let content = format!("{}\r\n{}\r\n", valid_line("a"), valid_line("b"));

        let parsed = parse_content(Path::new("known_hosts"), content.as_bytes());

        assert_eq!(parsed.entries.len(), 2);
    }

    #[test]
    fn should_parse_last_line_when_file_has_no_trailing_newline() {
        let parsed = parse_content(Path::new("known_hosts"), valid_line("host").as_bytes());

        assert_eq!(parsed.entries.len(), 1);
    }

    #[test]
    fn should_not_fail_when_a_comment_is_not_utf8() {
        let mut content = b"# \xff\xfe\n".to_vec();
        content.extend_from_slice(valid_line("host").as_bytes());

        let parsed = parse_content(Path::new("known_hosts"), &content);

        assert_eq!(parsed.entries.len(), 1);
    }

    #[test]
    fn should_read_as_empty_when_file_is_missing() {
        let directory = tempfile::tempdir().unwrap();

        let parsed = read_file(&directory.path().join("absent")).unwrap();

        assert_eq!(parsed.entries.len(), 0);
    }

    #[test]
    fn should_report_io_error_with_path_when_path_is_a_directory() {
        let directory = tempfile::tempdir().unwrap();

        let error = read_file(directory.path()).unwrap_err();

        assert!(matches!(error, Error::Io { path, .. } if path == directory.path()));
    }
}
