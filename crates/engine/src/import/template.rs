//! Placeholders (`$_`, `${port}`, `$port`) of a forward template.

const VARIABLE_PREFIX: char = '$';
const BRACE_OPEN: char = '{';
const BRACE_CLOSE: char = '}';

/// Whether `text` refers to a variable.
pub(super) fn has_placeholder(text: &str) -> bool {
    text.contains(VARIABLE_PREFIX)
}

/// `text` with every variable reference replaced by `port`.
pub(super) fn substitute(text: &str, port: u16) -> String {
    let mut substituted = String::new();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character != VARIABLE_PREFIX {
            substituted.push(character);
            continue;
        }
        skip_variable_name(&mut characters);
        substituted.push_str(&port.to_string());
    }
    substituted
}

fn skip_variable_name(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    if characters.peek() == Some(&BRACE_OPEN) {
        for character in characters.by_ref() {
            if character == BRACE_CLOSE {
                break;
            }
        }
        return;
    }
    while characters
        .next_if(|next| next.is_alphanumeric() || *next == '_')
        .is_some()
    {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_replace_braced_variable_when_text_has_one() {
        assert_eq!(
            substitute("${_}:127.0.0.1:${_}", 3333),
            "3333:127.0.0.1:3333"
        );
    }

    #[test]
    fn should_replace_plain_variable_when_text_has_one() {
        assert_eq!(substitute("$p:localhost:$port", 80), "80:localhost:80");
    }

    #[test]
    fn should_replace_underscore_variable_when_text_has_one() {
        assert_eq!(substitute("$_:db:5432", 1), "1:db:5432");
    }

    #[test]
    fn should_report_placeholder_only_when_text_has_a_dollar() {
        assert!(has_placeholder("${port}:a:1"));
        assert!(!has_placeholder("3333:a:1"));
    }
}
