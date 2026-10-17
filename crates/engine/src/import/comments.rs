//! Comment removal and line continuations.

const BLOCK_COMMENT_OPEN: &str = "<#";
const BLOCK_COMMENT_CLOSE: &str = "#>";
const LINE_COMMENT_START: char = '#';
const CONTINUATION_CHARACTERS: [char; 2] = ['`', '\\'];

/// A fragment of a physical line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Piece {
    /// Text that is not a comment.
    Code(String),
    /// The content of a comment, trimmed.
    Comment {
        text: String,
        /// Whether code precedes the comment on its line.
        follows_code: bool,
    },
}

/// One physical line, split into code and comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommentedLine {
    /// 1-based.
    pub number: u32,
    pub pieces: Vec<Piece>,
}

/// A piece of code with the line it was written on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodeSegment {
    pub number: u32,
    pub text: String,
}

struct OpenBlock {
    text: String,
    follows_code: bool,
}

/// Splits `text` into lines of code and comments.
///
/// Block comments (`<# ... #>`) may span lines; `#` starts a line comment at
/// the start of a line or after whitespace, outside quotes.
pub(super) fn strip_comments(text: &str) -> Vec<CommentedLine> {
    let mut open_block: Option<OpenBlock> = None;
    let mut lines = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let pieces = scan_line(line, &mut open_block);
        lines.push(CommentedLine { number, pieces });
    }
    lines
}

/// Groups the code of the lines into logical lines: a line ending with a
/// backtick (PowerShell) or a backslash (shell) continues on the next one.
pub(super) fn logical_lines(lines: &[CommentedLine]) -> Vec<Vec<CodeSegment>> {
    let mut logical = Vec::new();
    let mut current: Vec<CodeSegment> = Vec::new();
    for line in lines {
        let mut is_continued = false;
        for piece in &line.pieces {
            let Piece::Code(code) = piece else {
                continue;
            };
            let (without_marker, has_marker) = strip_continuation(code);
            is_continued = has_marker;
            current.push(CodeSegment {
                number: line.number,
                text: without_marker.to_owned(),
            });
        }
        if !is_continued && !current.is_empty() {
            logical.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        logical.push(current);
    }
    logical
}

fn strip_continuation(code: &str) -> (&str, bool) {
    let trimmed = code.trim_end();
    match trimmed.strip_suffix(CONTINUATION_CHARACTERS) {
        Some(without_marker) => (without_marker, true),
        None => (code, false),
    }
}

fn scan_line(line: &str, open_block: &mut Option<OpenBlock>) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut code = String::new();
    let mut quote: Option<char> = None;
    let mut rest = line;
    let mut previous: Option<char> = None;
    loop {
        if let Some(block) = open_block.as_mut() {
            let Some(end) = rest.find(BLOCK_COMMENT_CLOSE) else {
                block.text.push_str(rest);
                block.text.push(' ');
                break;
            };
            block.text.push_str(&rest[..end]);
            pieces.push(finish_block(open_block.take()));
            rest = &rest[end + BLOCK_COMMENT_CLOSE.len()..];
            previous = None;
            continue;
        }
        let Some(character) = rest.chars().next() else {
            break;
        };
        if quote.is_none() && rest.starts_with(BLOCK_COMMENT_OPEN) {
            flush_code(&mut code, &mut pieces);
            let follows_code = has_code(&pieces);
            *open_block = Some(OpenBlock {
                text: String::new(),
                follows_code,
            });
            rest = &rest[BLOCK_COMMENT_OPEN.len()..];
            continue;
        }
        let starts_comment = character == LINE_COMMENT_START
            && quote.is_none()
            && previous.is_none_or(char::is_whitespace);
        if starts_comment {
            flush_code(&mut code, &mut pieces);
            let follows_code = has_code(&pieces);
            pieces.push(comment(&rest[character.len_utf8()..], follows_code));
            break;
        }
        quote = next_quote_state(quote, character);
        code.push(character);
        previous = Some(character);
        rest = &rest[character.len_utf8()..];
    }
    flush_code(&mut code, &mut pieces);
    pieces
}

fn finish_block(block: Option<OpenBlock>) -> Piece {
    let block = block.unwrap_or(OpenBlock {
        text: String::new(),
        follows_code: false,
    });
    comment(&block.text, block.follows_code)
}

fn comment(text: &str, follows_code: bool) -> Piece {
    Piece::Comment {
        text: text
            .trim_start_matches(LINE_COMMENT_START)
            .trim()
            .to_owned(),
        follows_code,
    }
}

fn next_quote_state(quote: Option<char>, character: char) -> Option<char> {
    match (quote, character) {
        (Some(open), closing) if open == closing => None,
        (None, '"' | '\'') => Some(character),
        (current, _) => current,
    }
}

fn flush_code(code: &mut String, pieces: &mut Vec<Piece>) {
    if code.trim().is_empty() {
        code.clear();
        return;
    }
    pieces.push(Piece::Code(std::mem::take(code)));
}

fn has_code(pieces: &[Piece]) -> bool {
    pieces.iter().any(|piece| matches!(piece, Piece::Code(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(text: &str) -> Piece {
        Piece::Code(text.to_owned())
    }

    fn comment_after_code(text: &str) -> Piece {
        Piece::Comment {
            text: text.to_owned(),
            follows_code: true,
        }
    }

    #[test]
    fn should_extract_line_comment_when_hash_follows_whitespace() {
        let lines = strip_comments("3333 # adonis");

        assert_eq!(
            lines[0].pieces,
            vec![code("3333 "), comment_after_code("adonis")]
        );
    }

    #[test]
    fn should_keep_hash_when_it_is_inside_quotes() {
        let lines = strip_comments("echo \"a # b\"");

        assert_eq!(lines[0].pieces, vec![code("echo \"a # b\"")]);
    }

    #[test]
    fn should_keep_hash_when_it_is_not_preceded_by_whitespace() {
        let lines = strip_comments("echo ${#array}");

        assert_eq!(lines[0].pieces, vec![code("echo ${#array}")]);
    }

    #[test]
    fn should_extract_block_comment_when_it_sits_between_code() {
        let lines = strip_comments("3333 <# adonis #>,");

        assert_eq!(
            lines[0].pieces,
            vec![code("3333 "), comment_after_code("adonis"), code(",")]
        );
    }

    #[test]
    fn should_join_block_comment_text_when_it_spans_lines() {
        let lines = strip_comments("<# first\nsecond #> ssh");

        assert_eq!(
            lines[1].pieces,
            vec![
                Piece::Comment {
                    text: "first second".to_owned(),
                    follows_code: false
                },
                code(" ssh")
            ]
        );
    }

    #[test]
    fn should_ignore_apostrophe_when_it_is_inside_a_comment() {
        let lines = strip_comments("<# api's dev #>, 5");

        assert_eq!(lines[0].pieces[1], code(", 5"));
    }

    #[test]
    fn should_not_flag_comment_as_following_code_when_it_starts_the_line() {
        let lines = strip_comments("# standalone");

        assert_eq!(
            lines[0].pieces,
            vec![Piece::Comment {
                text: "standalone".to_owned(),
                follows_code: false
            }]
        );
    }

    #[test]
    fn should_join_lines_when_backslash_ends_the_line() {
        let logical = logical_lines(&strip_comments("ssh -N \\\n  -L 1:a:1 nova"));

        assert_eq!(logical.len(), 1);
        assert_eq!(logical[0].len(), 2);
    }

    #[test]
    fn should_join_lines_when_backtick_ends_the_line() {
        let logical = logical_lines(&strip_comments("ssh -N `\n  nova"));

        assert_eq!(logical.len(), 1);
    }

    #[test]
    fn should_keep_line_numbers_when_lines_are_joined() {
        let logical = logical_lines(&strip_comments("ssh \\\n nova"));

        assert_eq!(logical[0][1].number, 2);
    }
}
