use std::fs;
use std::path::{Path, PathBuf};

use crate::criteria::{Criterion, parse_criteria};
use crate::error::{Error, Origin, SyntaxError};
use crate::lexer::{LexedLine, lex_line};

/// A parsed configuration file: block headers and directives in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFile {
    /// Where the file was read from.
    pub path: PathBuf,
    /// The meaningful lines; blank lines and comments are dropped.
    pub lines: Vec<ConfigLine>,
}

/// A meaningful line of a configuration file with its 1-based line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigLine {
    /// 1-based line number in the file.
    pub number: usize,
    /// What the line says.
    pub content: LineContent,
}

/// The kind of a configuration line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineContent {
    /// A `Host` block header with its whitespace-separated patterns.
    Host {
        /// The patterns, possibly negated with `!`.
        patterns: Vec<String>,
    },
    /// A `Match` block header with its parsed criteria.
    Match {
        /// The criteria, all of which must match.
        criteria: Vec<Criterion>,
    },
    /// Any other directive.
    Directive(DirectiveLine),
}

/// A directive as written: lowercased keyword and its split arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveLine {
    /// The keyword, lowercased.
    pub keyword: String,
    /// The arguments after quote and escape processing, without any trailing comment.
    pub arguments: Vec<String>,
    /// The text after the keyword, unsplit, with trailing comment.
    pub raw_value: String,
}

const HOST_KEYWORD: &str = "host";
const MATCH_KEYWORD: &str = "match";

/// Reads and parses one configuration file without following `Include`.
///
/// Only the line syntax and `Host`/`Match` headers are validated here; the
/// values of directives are validated when the configuration is resolved.
pub fn parse_file(path: &Path) -> Result<ConfigFile, Error> {
    let text = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_source(path, &text)
}

pub(crate) fn parse_source(path: &Path, text: &str) -> Result<ConfigFile, Error> {
    let mut lines = Vec::new();
    for (index, text_line) in text.lines().enumerate() {
        let number = index + 1;
        let origin = || Origin::new(path, number);
        let lexed = lex_line(text_line).map_err(|kind| Error::Syntax {
            origin: origin(),
            kind,
        })?;
        let Some(lexed) = lexed else {
            continue;
        };
        let content = build_content(lexed, &origin)?;
        lines.push(ConfigLine { number, content });
    }
    Ok(ConfigFile {
        path: path.to_path_buf(),
        lines,
    })
}

fn build_content(lexed: LexedLine, origin: &dyn Fn() -> Origin) -> Result<LineContent, Error> {
    match lexed.keyword.as_str() {
        HOST_KEYWORD => build_host(lexed.arguments, origin),
        MATCH_KEYWORD => build_match(&lexed.arguments, origin),
        _ => Ok(LineContent::Directive(DirectiveLine {
            keyword: lexed.keyword,
            arguments: lexed.arguments,
            raw_value: lexed.raw_value,
        })),
    }
}

fn build_host(patterns: Vec<String>, origin: &dyn Fn() -> Origin) -> Result<LineContent, Error> {
    if patterns.iter().any(String::is_empty) {
        return Err(Error::Syntax {
            origin: origin(),
            kind: SyntaxError::EmptyHostPattern,
        });
    }
    Ok(LineContent::Host { patterns })
}

fn build_match(arguments: &[String], origin: &dyn Fn() -> Origin) -> Result<LineContent, Error> {
    let criteria = parse_criteria(arguments).map_err(|kind| Error::Criteria {
        origin: origin(),
        kind,
    })?;
    Ok(LineContent::Match { criteria })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<ConfigFile, Error> {
        parse_source(Path::new("config"), text)
    }

    #[test]
    fn should_keep_line_numbers_when_blank_lines_and_comments_are_skipped() {
        let file = parse("# comment\n\nHost web\n  Port 22\n").unwrap();
        let numbers: Vec<usize> = file.lines.iter().map(|line| line.number).collect();
        assert_eq!(numbers, vec![3, 4]);
    }

    #[test]
    fn should_parse_host_header_when_line_starts_with_host() {
        let file = parse("Host a b\n").unwrap();
        assert_eq!(
            file.lines[0].content,
            LineContent::Host {
                patterns: vec!["a".to_owned(), "b".to_owned()]
            }
        );
    }

    #[test]
    fn should_parse_host_header_when_keyword_is_uppercase_with_equals() {
        let file = parse("HOST=a\n").unwrap();
        assert!(matches!(file.lines[0].content, LineContent::Host { .. }));
    }

    #[test]
    fn should_parse_match_header_when_line_starts_with_match() {
        let file = parse("Match host a\n").unwrap();
        assert!(matches!(file.lines[0].content, LineContent::Match { .. }));
    }

    #[test]
    fn should_keep_directive_when_keyword_is_not_a_block_header() {
        let file = parse("Port 22\n").unwrap();
        assert_eq!(
            file.lines[0].content,
            LineContent::Directive(DirectiveLine {
                keyword: "port".to_owned(),
                arguments: vec!["22".to_owned()],
                raw_value: "22".to_owned()
            })
        );
    }

    #[test]
    fn should_report_path_and_line_when_syntax_is_invalid() {
        let error = parse("Port 22\nHost \"a\n").unwrap_err();
        assert_eq!(error.to_string(), "config line 2: invalid quotes");
    }

    #[test]
    fn should_report_path_and_line_when_match_is_invalid() {
        let error = parse("\nMatch host\n").unwrap_err();
        assert_eq!(
            error.to_string(),
            "config line 2: bad Match condition: missing argument for Match 'host'"
        );
    }

    #[test]
    fn should_fail_when_host_pattern_is_empty() {
        assert!(matches!(
            parse("Host \"\"\n"),
            Err(Error::Syntax {
                kind: SyntaxError::EmptyHostPattern,
                ..
            })
        ));
    }

    #[test]
    fn should_fail_with_io_error_when_file_does_not_exist() {
        assert!(matches!(
            parse_file(Path::new("/nonexistent/ssh_config")),
            Err(Error::Io { .. })
        ));
    }
}
