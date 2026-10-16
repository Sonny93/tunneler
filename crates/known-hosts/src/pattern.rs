use crate::error::WarningKind;
use crate::hashing::HashedHost;

const NEGATION_MARKER: u8 = b'!';
const LIST_SEPARATOR: char = ',';
const ANY_SEQUENCE: u8 = b'*';
const ANY_CHARACTER: u8 = b'?';
const HASHED_FIELD_PREFIX: char = '|';

/// The host field of a `known_hosts` line: a hashed name or a pattern list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HostPatterns {
    Hashed(HashedHost),
    List(Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pattern {
    glob: Vec<u8>,
    is_negated: bool,
}

impl HostPatterns {
    pub(crate) fn parse(field: &str) -> Result<Self, WarningKind> {
        if field.starts_with(HASHED_FIELD_PREFIX) {
            return HashedHost::parse(field)
                .map(Self::Hashed)
                .ok_or(WarningKind::InvalidHostHash);
        }
        Ok(Self::List(
            field.split(LIST_SEPARATOR).map(Pattern::parse).collect(),
        ))
    }

    /// Whether `name`, already lowercased, is covered by these patterns.
    pub(crate) fn matches(&self, name: &str) -> bool {
        match self {
            Self::Hashed(hashed) => hashed.matches(name),
            Self::List(patterns) => matches_list(patterns, name.as_bytes()),
        }
    }
}

impl Pattern {
    fn parse(text: &str) -> Self {
        let bytes = text.as_bytes();
        let is_negated = bytes.first() == Some(&NEGATION_MARKER);
        let glob = if is_negated { &bytes[1..] } else { bytes };
        Self {
            glob: glob.to_ascii_lowercase(),
            is_negated,
        }
    }
}

fn matches_list(patterns: &[Pattern], name: &[u8]) -> bool {
    let mut has_positive_match = false;
    for pattern in patterns
        .iter()
        .filter(|pattern| matches_glob(name, &pattern.glob))
    {
        if pattern.is_negated {
            return false;
        }
        has_positive_match = true;
    }
    has_positive_match
}

/// OpenSSH `match_pattern`: `*` and `?` wildcards over bytes.
pub(crate) fn matches_glob(text: &[u8], pattern: &[u8]) -> bool {
    let mut text_index = 0;
    let mut pattern_index = 0;
    let mut last_star: Option<(usize, usize)> = None;
    loop {
        let pattern_byte = pattern.get(pattern_index).copied();
        let text_byte = text.get(text_index).copied();
        match (pattern_byte, text_byte) {
            (Some(ANY_SEQUENCE), _) => {
                last_star = Some((pattern_index, text_index));
                pattern_index += 1;
            }
            (None, None) => return true,
            (Some(expected), Some(actual)) if expected == ANY_CHARACTER || expected == actual => {
                pattern_index += 1;
                text_index += 1;
            }
            _ => match last_star {
                Some((star_index, star_text_index)) if star_text_index < text.len() => {
                    last_star = Some((star_index, star_text_index + 1));
                    pattern_index = star_index + 1;
                    text_index = star_text_index + 1;
                }
                _ => return false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns(field: &str) -> HostPatterns {
        HostPatterns::parse(field).unwrap()
    }

    #[test]
    fn should_match_when_plain_name_equals_lookup_name() {
        assert!(patterns("example.com").matches("example.com"));
    }

    #[test]
    fn should_not_match_when_plain_name_differs() {
        assert!(!patterns("example.com").matches("example.org"));
    }

    #[test]
    fn should_match_when_pattern_has_uppercase_letters() {
        assert!(patterns("Example.COM").matches("example.com"));
    }

    #[test]
    fn should_match_any_listed_name_when_field_is_comma_separated() {
        assert!(patterns("a.example,b.example").matches("b.example"));
    }

    #[test]
    fn should_match_bracketed_name_when_port_is_part_of_lookup_name() {
        assert!(patterns("[host]:2222").matches("[host]:2222"));
    }

    #[test]
    fn should_not_match_bracketed_pattern_when_lookup_name_has_no_port() {
        assert!(!patterns("[host]:2222").matches("host"));
    }

    #[test]
    fn should_match_when_star_replaces_a_suffix() {
        assert!(patterns("*.example.com").matches("web.example.com"));
    }

    #[test]
    fn should_match_when_star_replaces_empty_text() {
        assert!(patterns("web*").matches("web"));
    }

    #[test]
    fn should_match_when_question_mark_replaces_one_character() {
        assert!(patterns("web?").matches("web1"));
    }

    #[test]
    fn should_not_match_when_question_mark_has_no_character_to_replace() {
        assert!(!patterns("web?").matches("web"));
    }

    #[test]
    fn should_match_when_several_stars_need_backtracking() {
        assert!(patterns("*a*b*c").matches("xxaxxbxxc"));
    }

    #[test]
    fn should_not_match_when_text_has_trailing_characters() {
        assert!(!patterns("*.com").matches("example.com.evil"));
    }

    #[test]
    fn should_not_match_when_a_negated_pattern_matches() {
        assert!(!patterns("*.example.com,!secret.example.com").matches("secret.example.com"));
    }

    #[test]
    fn should_not_match_when_negation_precedes_the_positive_pattern() {
        assert!(!patterns("!secret.example.com,*.example.com").matches("secret.example.com"));
    }

    #[test]
    fn should_match_when_negated_pattern_does_not_match() {
        assert!(patterns("*.example.com,!secret.example.com").matches("web.example.com"));
    }

    #[test]
    fn should_not_match_when_only_negated_patterns_are_present() {
        assert!(!patterns("!secret.example.com").matches("web.example.com"));
    }

    #[test]
    fn should_match_when_hashed_entry_was_generated_for_lookup_name() {
        let hashed = crate::hashing::HashedHost::with_salt("example.com", [1; 20]);

        assert!(patterns(&hashed.to_string()).matches("example.com"));
    }

    #[test]
    fn should_reject_field_when_hashed_entry_is_malformed() {
        assert_eq!(
            HostPatterns::parse("|1|broken"),
            Err(WarningKind::InvalidHostHash)
        );
    }
}
