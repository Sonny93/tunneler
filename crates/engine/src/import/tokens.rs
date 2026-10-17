//! Splits the code of a logical line into shell-like words.

use super::comments::CodeSegment;

const SEPARATOR_CHARACTERS: [char; 3] = [';', '|', '&'];
const OPTION_PREFIX: char = '-';

/// A word of a logical line, or a statement separator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Token {
    /// The word without its quotes.
    pub text: String,
    /// The 1-based line the word was written on.
    pub line: u32,
    /// `;`, `|` or `&`: ends the arguments of a command.
    pub is_separator: bool,
}

#[derive(Default)]
struct Word {
    text: String,
    is_quoted: bool,
}

/// Splits the segments into words, honoring single and double quotes.
///
/// A quoted word that starts with `-` and holds spaces (`"-L 1:a:1"`) is
/// split again, as PowerShell passes it on as separate arguments.
pub(super) fn tokenize(segments: &[CodeSegment]) -> Vec<Token> {
    segments.iter().flat_map(tokenize_segment).collect()
}

fn tokenize_segment(segment: &CodeSegment) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut word = Word::default();
    let mut quote: Option<char> = None;
    for character in segment.text.chars() {
        match (quote, character) {
            (Some(open), closing) if open == closing => quote = None,
            (Some(_), inside) => word.text.push(inside),
            (None, '"' | '\'') => {
                quote = Some(character);
                word.is_quoted = true;
            }
            (None, separator) if SEPARATOR_CHARACTERS.contains(&separator) => {
                flush_word(&mut word, segment.number, &mut tokens);
                tokens.push(separator_token(separator, segment.number));
            }
            (None, space) if space.is_whitespace() => {
                flush_word(&mut word, segment.number, &mut tokens);
            }
            (None, other) => word.text.push(other),
        }
    }
    flush_word(&mut word, segment.number, &mut tokens);
    tokens
}

fn separator_token(separator: char, line: u32) -> Token {
    Token {
        text: separator.to_string(),
        line,
        is_separator: true,
    }
}

fn flush_word(word: &mut Word, line: u32, tokens: &mut Vec<Token>) {
    let finished = std::mem::take(word);
    if finished.text.is_empty() {
        return;
    }
    let is_option_string = finished.is_quoted && finished.text.starts_with(OPTION_PREFIX);
    let texts: Vec<&str> = if is_option_string {
        finished.text.split_whitespace().collect()
    } else {
        vec![finished.text.as_str()]
    };
    tokens.extend(texts.into_iter().map(|text| Token {
        text: text.to_owned(),
        line,
        is_separator: false,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(line: &str) -> Vec<String> {
        let segment = CodeSegment {
            number: 1,
            text: line.to_owned(),
        };
        tokenize(&[segment])
            .into_iter()
            .map(|token| token.text)
            .collect()
    }

    #[test]
    fn should_split_on_whitespace_when_words_are_unquoted() {
        assert_eq!(texts("ssh -N nova"), vec!["ssh", "-N", "nova"]);
    }

    #[test]
    fn should_keep_quoted_words_together_when_they_hold_spaces() {
        assert_eq!(
            texts("ssh -o \"ProxyCommand foo\" nova"),
            vec!["ssh", "-o", "ProxyCommand foo", "nova"]
        );
    }

    #[test]
    fn should_split_quoted_option_string_when_it_starts_with_a_dash() {
        assert_eq!(texts("{ \"-L 1:a:1\" }"), vec!["{", "-L", "1:a:1", "}"]);
    }

    #[test]
    fn should_emit_separator_when_statements_are_chained() {
        let segment = CodeSegment {
            number: 1,
            text: "ssh a && ssh b".to_owned(),
        };

        let separators = tokenize(&[segment])
            .iter()
            .filter(|token| token.is_separator)
            .count();

        assert_eq!(separators, 2);
    }

    #[test]
    fn should_not_emit_separator_when_it_is_quoted() {
        assert_eq!(texts("echo \"a;b\""), vec!["echo", "a;b"]);
    }
}
