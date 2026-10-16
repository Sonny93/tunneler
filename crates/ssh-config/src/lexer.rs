use crate::error::SyntaxError;

const WHITESPACE: [char; 4] = [' ', '\t', '\r', '\n'];
const TRAILING_WHITESPACE: [char; 5] = [' ', '\t', '\r', '\n', '\x0c'];
const KEYWORD_DELIMITERS: [char; 6] = [' ', '\t', '\r', '\n', '"', '='];
const ARGUMENT_SEPARATORS: [char; 2] = [' ', '\t'];
const COMMENT_MARKER: char = '#';
const EQUALS_SIGN: char = '=';
const DOUBLE_QUOTE: char = '"';
const SINGLE_QUOTE: char = '\'';
const BACKSLASH: char = '\\';
const SPACE: char = ' ';

/// A configuration line split into its keyword and arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexedLine {
    pub keyword: String,
    pub raw_value: String,
    pub arguments: Vec<String>,
}

struct DelimitedToken<'text> {
    token: String,
    remainder: Option<&'text str>,
}

/// Splits one line the way OpenSSH `process_config_line` does.
///
/// Returns `Ok(None)` for blank lines and comments.
pub(crate) fn lex_line(text: &str) -> Result<Option<LexedLine>, SyntaxError> {
    let trimmed = text.trim_end_matches(TRAILING_WHITESPACE);
    let Some(first) = split_token(trimmed) else {
        return Ok(None);
    };
    let Some(keyword_token) = skip_leading_whitespace_token(first) else {
        return Ok(None);
    };
    if keyword_token.token.is_empty() || keyword_token.token.starts_with(COMMENT_MARKER) {
        return Ok(None);
    }
    let keyword = keyword_token.token.to_ascii_lowercase();
    let raw_value = keyword_token
        .remainder
        .map(|remainder| remainder.trim_start_matches(WHITESPACE))
        .unwrap_or_default();
    if raw_value.is_empty() {
        return Err(SyntaxError::MissingArguments { keyword });
    }
    let arguments = split_arguments(raw_value)?;
    Ok(Some(LexedLine {
        keyword,
        raw_value: raw_value.to_owned(),
        arguments,
    }))
}

fn skip_leading_whitespace_token(first: DelimitedToken<'_>) -> Option<DelimitedToken<'_>> {
    if !first.token.is_empty() {
        return Some(first);
    }
    split_token(first.remainder?)
}

fn split_token(text: &str) -> Option<DelimitedToken<'_>> {
    let Some(delimiter_index) = text.find(KEYWORD_DELIMITERS) else {
        return Some(DelimitedToken {
            token: text.to_owned(),
            remainder: None,
        });
    };
    let delimiter = text[delimiter_index..].chars().next()?;
    if delimiter == DOUBLE_QUOTE {
        return split_quoted_token(text, delimiter_index);
    }
    let mut remainder = text[delimiter_index + 1..].trim_start_matches(WHITESPACE);
    if delimiter != EQUALS_SIGN && remainder.starts_with(EQUALS_SIGN) {
        remainder = remainder[1..].trim_start_matches(WHITESPACE);
    }
    Some(DelimitedToken {
        token: text[..delimiter_index].to_owned(),
        remainder: Some(remainder),
    })
}

fn split_quoted_token(text: &str, quote_index: usize) -> Option<DelimitedToken<'_>> {
    let inside_start = quote_index + 1;
    let closing_offset = text[inside_start..].find(DOUBLE_QUOTE)?;
    let inside_end = inside_start + closing_offset;
    let token = format!(
        "{}{}",
        &text[..quote_index],
        &text[inside_start..inside_end]
    );
    let remainder = text[inside_end + 1..].trim_start_matches(WHITESPACE);
    Some(DelimitedToken {
        token,
        remainder: Some(remainder),
    })
}

/// Splits arguments like OpenSSH `argv_split` with comment termination.
pub(crate) fn split_arguments(text: &str) -> Result<Vec<String>, SyntaxError> {
    let characters: Vec<char> = text.chars().collect();
    let mut arguments = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if ARGUMENT_SEPARATORS.contains(&characters[index]) {
            index += 1;
            continue;
        }
        if characters[index] == COMMENT_MARKER {
            break;
        }
        arguments.push(read_argument(&characters, &mut index)?);
    }
    Ok(arguments)
}

fn read_argument(characters: &[char], index: &mut usize) -> Result<String, SyntaxError> {
    let mut argument = String::new();
    let mut open_quote: Option<char> = None;
    while let Some(&current) = characters.get(*index) {
        let next = characters.get(*index + 1).copied();
        if current == BACKSLASH {
            if is_escapable(next, open_quote) {
                *index += 1;
                argument.extend(next);
            } else {
                argument.push(BACKSLASH);
            }
        } else if open_quote.is_none() && ARGUMENT_SEPARATORS.contains(&current) {
            break;
        } else if open_quote.is_none() && is_quote(current) {
            open_quote = Some(current);
        } else if open_quote == Some(current) {
            open_quote = None;
        } else {
            argument.push(current);
        }
        *index += 1;
    }
    if open_quote.is_some() {
        return Err(SyntaxError::UnbalancedQuotes);
    }
    Ok(argument)
}

fn is_quote(character: char) -> bool {
    character == DOUBLE_QUOTE || character == SINGLE_QUOTE
}

fn is_escapable(next: Option<char>, open_quote: Option<char>) -> bool {
    match next {
        Some(DOUBLE_QUOTE | SINGLE_QUOTE | BACKSLASH) => true,
        Some(SPACE) => open_quote.is_none(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(text: &str) -> LexedLine {
        lex_line(text).unwrap().unwrap()
    }

    fn arguments_of(text: &str) -> Vec<String> {
        lex(text).arguments
    }

    #[test]
    fn should_skip_line_when_it_is_empty() {
        assert_eq!(lex_line("").unwrap(), None);
    }

    #[test]
    fn should_skip_line_when_it_only_contains_whitespace() {
        assert_eq!(lex_line("   \t ").unwrap(), None);
    }

    #[test]
    fn should_skip_line_when_it_is_a_comment() {
        assert_eq!(lex_line("# Host foo").unwrap(), None);
    }

    #[test]
    fn should_skip_line_when_comment_is_indented() {
        assert_eq!(lex_line("   # Host foo").unwrap(), None);
    }

    #[test]
    fn should_lowercase_keyword_when_it_is_mixed_case() {
        assert_eq!(lex("HoStNaMe example.com").keyword, "hostname");
    }

    #[test]
    fn should_strip_leading_whitespace_when_line_is_indented() {
        assert_eq!(lex("\t  Port 22").keyword, "port");
    }

    #[test]
    fn should_split_keyword_when_separated_by_equals() {
        assert_eq!(arguments_of("Port=2222"), vec!["2222"]);
    }

    #[test]
    fn should_split_keyword_when_equals_has_surrounding_whitespace() {
        assert_eq!(arguments_of("Port = 2222"), vec!["2222"]);
    }

    #[test]
    fn should_split_keyword_when_equals_follows_whitespace() {
        assert_eq!(arguments_of("Port =2222"), vec!["2222"]);
    }

    #[test]
    fn should_keep_second_equals_when_value_starts_with_equals() {
        assert_eq!(arguments_of("Port == 2222"), vec!["=", "2222"]);
    }

    #[test]
    fn should_keep_equals_inside_value_when_value_contains_equals() {
        assert_eq!(arguments_of("SetEnv A=b"), vec!["A=b"]);
    }

    #[test]
    fn should_split_arguments_when_separated_by_tabs_and_spaces() {
        assert_eq!(arguments_of("Host a\tb  c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn should_group_argument_when_double_quoted() {
        assert_eq!(arguments_of("IdentityFile \"/a b/key\""), vec!["/a b/key"]);
    }

    #[test]
    fn should_group_argument_when_single_quoted() {
        assert_eq!(arguments_of("IdentityFile '/a b/key'"), vec!["/a b/key"]);
    }

    #[test]
    fn should_join_segments_when_quote_starts_mid_argument() {
        assert_eq!(arguments_of("Host a\"b c\"d"), vec!["ab cd"]);
    }

    #[test]
    fn should_keep_other_quote_when_nested_in_quotes() {
        assert_eq!(arguments_of("Host \"it's\""), vec!["it's"]);
    }

    #[test]
    fn should_unescape_space_when_backslash_precedes_it_outside_quotes() {
        assert_eq!(arguments_of("Host a\\ b"), vec!["a b"]);
    }

    #[test]
    fn should_unescape_quote_when_backslash_precedes_it() {
        assert_eq!(arguments_of("Host \\\"a"), vec!["\"a"]);
    }

    #[test]
    fn should_unescape_backslash_when_doubled() {
        assert_eq!(arguments_of("Host a\\\\b"), vec!["a\\b"]);
    }

    #[test]
    fn should_keep_backslash_when_escape_is_unrecognised() {
        assert_eq!(arguments_of("Host a\\nb"), vec!["a\\nb"]);
    }

    #[test]
    fn should_keep_backslash_and_space_when_escaped_space_is_inside_quotes() {
        assert_eq!(arguments_of("Host \"a\\ b\""), vec!["a\\ b"]);
    }

    #[test]
    fn should_end_line_when_trailing_comment_starts_a_token() {
        assert_eq!(arguments_of("Port 22 # standard"), vec!["22"]);
    }

    #[test]
    fn should_keep_hash_when_it_is_inside_a_token() {
        assert_eq!(arguments_of("Host a#b"), vec!["a#b"]);
    }

    #[test]
    fn should_keep_hash_when_it_is_quoted() {
        assert_eq!(arguments_of("Host \"#a\""), vec!["#a"]);
    }

    #[test]
    fn should_yield_no_arguments_when_value_is_only_a_comment() {
        assert_eq!(arguments_of("Port # none"), Vec::<String>::new());
    }

    #[test]
    fn should_keep_raw_value_when_line_has_trailing_comment() {
        assert_eq!(lex("ProxyCommand nc %h %p # c").raw_value, "nc %h %p # c");
    }

    #[test]
    fn should_ignore_trailing_whitespace_when_line_ends_with_it() {
        assert_eq!(lex("Port 22 \t\r").raw_value, "22");
    }

    #[test]
    fn should_ignore_form_feed_when_it_ends_the_line() {
        assert_eq!(lex("Port 22\x0c").raw_value, "22");
    }

    #[test]
    fn should_fail_when_keyword_has_no_argument() {
        assert_eq!(
            lex_line("Port"),
            Err(SyntaxError::MissingArguments {
                keyword: "port".to_owned()
            })
        );
    }

    #[test]
    fn should_fail_when_equals_is_not_followed_by_a_value() {
        assert!(matches!(
            lex_line("Port ="),
            Err(SyntaxError::MissingArguments { .. })
        ));
    }

    #[test]
    fn should_fail_when_quote_is_not_closed() {
        assert_eq!(lex_line("Host \"a b"), Err(SyntaxError::UnbalancedQuotes));
    }

    #[test]
    fn should_accept_quoted_keyword_when_quotes_surround_it() {
        assert_eq!(lex("\"Port\" 22").keyword, "port");
    }

    #[test]
    fn should_skip_line_when_keyword_quote_is_not_closed() {
        assert_eq!(lex_line("\"Port 22").unwrap(), None);
    }
}
