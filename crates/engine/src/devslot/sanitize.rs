//! Cleaning of the strings a remote host sends: they are untrusted, and end
//! up in front of the user.

use std::ops::RangeInclusive;

/// The longest a name, variable, kind, service or URL a host sends may be.
pub(crate) const MAX_TEXT_BYTES: usize = 256;

/// The longest a path a host sends may be.
pub(crate) const MAX_PATH_BYTES: usize = 1024;

/// The most of the standard error shown to the user.
pub(crate) const MAX_STDERR_BYTES: usize = 1024;

const BIDI_CONTROLS: [char; 3] = ['\u{061C}', '\u{200E}', '\u{200F}'];
const BIDI_EMBEDDING_RANGE: RangeInclusive<char> = '\u{202A}'..='\u{202E}';
const BIDI_ISOLATE_RANGE: RangeInclusive<char> = '\u{2066}'..='\u{2069}';

fn is_forbidden(character: char) -> bool {
    character.is_control()
        || BIDI_CONTROLS.contains(&character)
        || BIDI_EMBEDDING_RANGE.contains(&character)
        || BIDI_ISOLATE_RANGE.contains(&character)
}

/// Whether `text` holds a control or bidirectional control character.
pub(crate) fn contains_forbidden_character(text: &str) -> bool {
    text.chars().any(is_forbidden)
}

/// Removes the control and bidirectional control characters of `text`, then
/// keeps its first `max_bytes` bytes, cut at a character boundary.
pub(crate) fn sanitize_text(text: &str, max_bytes: usize) -> String {
    let mut cleaned = String::new();
    for character in text.chars().filter(|character| !is_forbidden(*character)) {
        if cleaned.len() + character.len_utf8() > max_bytes {
            break;
        }
        cleaned.push(character);
    }
    cleaned
}

/// Turns the tail of a standard error into one clean line: line breaks and
/// tabs become spaces, the other forbidden characters are removed, and the
/// last [`MAX_STDERR_BYTES`] bytes are kept.
pub(crate) fn sanitize_stderr(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let spaced: String = text
        .chars()
        .map(|character| {
            if character.is_whitespace() && character.is_control() {
                ' '
            } else {
                character
            }
        })
        .filter(|character| !is_forbidden(*character))
        .collect();
    keep_tail(spaced.trim(), MAX_STDERR_BYTES)
}

fn keep_tail(text: &str, max_bytes: usize) -> String {
    let mut start = text.len().saturating_sub(max_bytes);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_strip_control_characters_when_sanitizing_text() {
        assert_eq!(
            sanitize_text("my\u{0}-\u{7}app\n", MAX_TEXT_BYTES),
            "my-app"
        );
    }

    #[test]
    fn should_strip_bidirectional_controls_when_sanitizing_text() {
        let text = "a\u{061C}b\u{200E}c\u{200F}d\u{202A}e\u{202E}f\u{2066}g\u{2069}h";

        assert_eq!(sanitize_text(text, MAX_TEXT_BYTES), "abcdefgh");
    }

    #[test]
    fn should_cap_the_length_when_the_text_is_longer_than_the_limit() {
        let text = "a".repeat(MAX_TEXT_BYTES + 10);

        assert_eq!(sanitize_text(&text, MAX_TEXT_BYTES).len(), MAX_TEXT_BYTES);
    }

    #[test]
    fn should_cut_at_a_character_boundary_when_the_limit_falls_inside_a_character() {
        let text = "é".repeat(200);

        let cleaned = sanitize_text(&text, MAX_TEXT_BYTES);

        assert_eq!(cleaned.len(), MAX_TEXT_BYTES);
        assert_eq!(cleaned.chars().count(), 128);
    }

    #[test]
    fn should_keep_ordinary_unicode_when_sanitizing_text() {
        assert_eq!(sanitize_text("café-日本", MAX_TEXT_BYTES), "café-日本");
    }

    #[test]
    fn should_join_the_lines_when_sanitizing_the_standard_error() {
        assert_eq!(
            sanitize_stderr(b"bash: ds: command\nnot found\n"),
            "bash: ds: command not found"
        );
    }

    #[test]
    fn should_keep_the_end_when_the_standard_error_is_longer_than_the_limit() {
        let mut bytes = vec![b'a'; MAX_STDERR_BYTES * 2];
        bytes.extend_from_slice(b"the end");

        let cleaned = sanitize_stderr(&bytes);

        assert_eq!(cleaned.len(), MAX_STDERR_BYTES);
        assert!(cleaned.ends_with("the end"));
    }

    #[test]
    fn should_strip_control_characters_when_sanitizing_the_standard_error() {
        assert_eq!(sanitize_stderr(b"\x1b[31mred\x07"), "[31mred");
    }
}
