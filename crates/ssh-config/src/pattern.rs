const NEGATION_MARKER: char = '!';
const LIST_SEPARATOR: char = ',';
const ANY_SEQUENCE: char = '*';
const ANY_CHARACTER: char = '?';
const SUBPATTERN_LENGTH_LIMIT: usize = 1023;

/// Outcome of matching a string against a comma-separated pattern list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListMatch {
    Positive,
    Negative,
    NoMatch,
}

impl ListMatch {
    pub(crate) fn is_positive(self) -> bool {
        self == Self::Positive
    }
}

/// OpenSSH `match_pattern`: `*` and `?` wildcards, case-sensitive.
pub(crate) fn matches_wildcard(text: &str, pattern: &str) -> bool {
    let text_characters: Vec<char> = text.chars().collect();
    let pattern_characters: Vec<char> = pattern.chars().collect();
    matches_wildcard_characters(&text_characters, &pattern_characters)
}

fn matches_wildcard_characters(text: &[char], pattern: &[char]) -> bool {
    let Some((&pattern_head, pattern_tail)) = pattern.split_first() else {
        return text.is_empty();
    };
    if pattern_head == ANY_SEQUENCE {
        return matches_any_sequence(text, pattern_tail);
    }
    let Some((&text_head, text_tail)) = text.split_first() else {
        return false;
    };
    let is_compatible = pattern_head == ANY_CHARACTER || pattern_head == text_head;
    is_compatible && matches_wildcard_characters(text_tail, pattern_tail)
}

fn matches_any_sequence(text: &[char], pattern_after_star: &[char]) -> bool {
    let remaining_pattern = skip_stars(pattern_after_star);
    if remaining_pattern.is_empty() {
        return true;
    }
    (0..=text.len()).any(|skipped| matches_wildcard_characters(&text[skipped..], remaining_pattern))
}

fn skip_stars(pattern: &[char]) -> &[char] {
    let star_count = pattern
        .iter()
        .take_while(|&&character| character == ANY_SEQUENCE)
        .count();
    &pattern[star_count..]
}

/// OpenSSH `match_pattern_list` without lowercasing: comma-separated list of
/// patterns, each optionally negated with `!`.
pub(crate) fn match_pattern_list(text: &str, list: &str) -> ListMatch {
    match_list_with(text, list, str::to_owned)
}

/// OpenSSH `match_hostname`: both the host and the patterns are lowercased.
pub(crate) fn match_hostname_list(host: &str, list: &str) -> ListMatch {
    match_list_with(&host.to_ascii_lowercase(), list, str::to_ascii_lowercase)
}

fn match_list_with(text: &str, list: &str, normalize: impl Fn(&str) -> String) -> ListMatch {
    let mut has_positive_match = false;
    for entry in list_entries(list) {
        let (is_negated, subpattern) = split_negation(entry);
        if subpattern.len() >= SUBPATTERN_LENGTH_LIMIT {
            return ListMatch::NoMatch;
        }
        if !matches_wildcard(text, &normalize(subpattern)) {
            continue;
        }
        if is_negated {
            return ListMatch::Negative;
        }
        has_positive_match = true;
    }
    if has_positive_match {
        ListMatch::Positive
    } else {
        ListMatch::NoMatch
    }
}

fn list_entries(list: &str) -> impl Iterator<Item = &str> {
    let without_trailing_separator = list.strip_suffix(LIST_SEPARATOR).unwrap_or(list);
    let entries = (!list.is_empty()).then(|| without_trailing_separator.split(LIST_SEPARATOR));
    entries.into_iter().flatten()
}

fn split_negation(entry: &str) -> (bool, &str) {
    match entry.strip_prefix(NEGATION_MARKER) {
        Some(subpattern) => (true, subpattern),
        None => (false, entry),
    }
}

/// Whether a `Host` line with these whitespace-separated patterns applies to
/// `host`: at least one positive match and no matching negated pattern.
pub(crate) fn host_patterns_apply(patterns: &[String], host: &str) -> bool {
    let mut is_applicable = false;
    for pattern in patterns {
        let (is_negated, subpattern) = split_negation(pattern);
        if !matches_wildcard(host, subpattern) {
            continue;
        }
        if is_negated {
            return false;
        }
        is_applicable = true;
    }
    is_applicable
}

/// A `Host` pattern naming exactly one alias: no wildcard and no negation.
pub(crate) fn is_concrete_host_pattern(pattern: &str) -> bool {
    !pattern.is_empty()
        && !pattern.starts_with(NEGATION_MARKER)
        && !pattern.contains([ANY_SEQUENCE, ANY_CHARACTER])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn should_match_when_pattern_equals_text() {
        assert!(matches_wildcard("web", "web"));
    }

    #[test]
    fn should_not_match_when_pattern_differs_in_case() {
        assert!(!matches_wildcard("Web", "web"));
    }

    #[test]
    fn should_match_when_star_replaces_a_suffix() {
        assert!(matches_wildcard("web01.example.com", "web*"));
    }

    #[test]
    fn should_match_when_star_replaces_empty_text() {
        assert!(matches_wildcard("web", "web*"));
    }

    #[test]
    fn should_match_when_star_is_in_the_middle() {
        assert!(matches_wildcard("a-long-name", "a*name"));
    }

    #[test]
    fn should_match_when_consecutive_stars_are_used() {
        assert!(matches_wildcard("abc", "a**c"));
    }

    #[test]
    fn should_not_match_when_text_continues_after_literal_end() {
        assert!(!matches_wildcard("webx", "web"));
    }

    #[test]
    fn should_match_when_question_mark_replaces_one_character() {
        assert!(matches_wildcard("web1", "web?"));
    }

    #[test]
    fn should_not_match_when_question_mark_has_no_character_left() {
        assert!(!matches_wildcard("web", "web?"));
    }

    #[test]
    fn should_match_everything_when_pattern_is_a_single_star() {
        assert!(matches_wildcard("anything", "*"));
    }

    #[test]
    fn should_match_empty_text_when_pattern_is_empty() {
        assert!(matches_wildcard("", ""));
    }

    #[test]
    fn should_backtrack_when_star_first_candidate_fails() {
        assert!(matches_wildcard("aXbXc", "*Xc"));
    }

    #[test]
    fn should_return_positive_when_list_entry_matches() {
        assert_eq!(match_pattern_list("bob", "alice,bob"), ListMatch::Positive);
    }

    #[test]
    fn should_return_no_match_when_no_list_entry_matches() {
        assert_eq!(match_pattern_list("eve", "alice,bob"), ListMatch::NoMatch);
    }

    #[test]
    fn should_return_negative_when_negated_entry_matches() {
        assert_eq!(match_pattern_list("bob", "*,!bob"), ListMatch::Negative);
    }

    #[test]
    fn should_return_negative_when_negated_entry_precedes_positive_one() {
        assert_eq!(match_pattern_list("bob", "!bob,*"), ListMatch::Negative);
    }

    #[test]
    fn should_not_match_when_only_a_negated_entry_is_present_and_text_differs() {
        assert_eq!(match_pattern_list("eve", "!bob"), ListMatch::NoMatch);
    }

    #[test]
    fn should_be_case_sensitive_when_matching_a_plain_list() {
        assert_eq!(match_pattern_list("Bob", "bob"), ListMatch::NoMatch);
    }

    #[test]
    fn should_ignore_case_when_matching_hostnames() {
        assert_eq!(
            match_hostname_list("WEB.Example.COM", "web.example.com"),
            ListMatch::Positive
        );
    }

    #[test]
    fn should_lowercase_patterns_when_matching_hostnames() {
        assert_eq!(
            match_hostname_list("web.example.com", "WEB.*"),
            ListMatch::Positive
        );
    }

    #[test]
    fn should_return_no_match_when_list_is_empty() {
        assert_eq!(match_pattern_list("", ""), ListMatch::NoMatch);
    }

    #[test]
    fn should_ignore_trailing_separator_when_list_ends_with_a_comma() {
        assert_eq!(match_pattern_list("", "a,"), ListMatch::NoMatch);
    }

    #[test]
    fn should_return_no_match_when_subpattern_is_too_long() {
        let long_pattern = "a".repeat(SUBPATTERN_LENGTH_LIMIT);
        assert_eq!(match_pattern_list("a", &long_pattern), ListMatch::NoMatch);
    }

    #[test]
    fn should_apply_host_line_when_one_pattern_matches() {
        assert!(host_patterns_apply(&patterns(&["a", "web"]), "web"));
    }

    #[test]
    fn should_not_apply_host_line_when_nothing_matches() {
        assert!(!host_patterns_apply(&patterns(&["a", "b"]), "web"));
    }

    #[test]
    fn should_not_apply_host_line_when_negated_pattern_matches() {
        assert!(!host_patterns_apply(&patterns(&["*", "!web"]), "web"));
    }

    #[test]
    fn should_not_apply_host_line_when_negated_pattern_precedes_the_wildcard() {
        assert!(!host_patterns_apply(&patterns(&["!web", "*"]), "web"));
    }

    #[test]
    fn should_not_apply_host_line_when_it_only_has_negated_patterns() {
        assert!(!host_patterns_apply(&patterns(&["!web"]), "other"));
    }

    #[test]
    fn should_not_treat_comma_as_separator_when_matching_host_patterns() {
        assert!(!host_patterns_apply(&patterns(&["a,b"]), "a"));
    }

    #[test]
    fn should_be_case_sensitive_when_matching_host_patterns() {
        assert!(!host_patterns_apply(&patterns(&["Web"]), "web"));
    }

    #[test]
    fn should_report_concrete_when_pattern_has_no_wildcard_or_negation() {
        assert!(is_concrete_host_pattern("web01"));
    }

    #[test]
    fn should_not_report_concrete_when_pattern_has_a_star() {
        assert!(!is_concrete_host_pattern("web*"));
    }

    #[test]
    fn should_not_report_concrete_when_pattern_has_a_question_mark() {
        assert!(!is_concrete_host_pattern("web?"));
    }

    #[test]
    fn should_not_report_concrete_when_pattern_is_negated() {
        assert!(!is_concrete_host_pattern("!web"));
    }
}
