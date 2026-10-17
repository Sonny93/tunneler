//! Port lists: PowerShell and shell arrays, and lines made of numbers only.

use super::comments::{CommentedLine, Piece};

const ARRAY_OPENERS: [char; 2] = ['@', '='];
const ARRAY_OPEN: char = '(';
const ARRAY_CLOSE: char = ')';
const LIST_SEPARATOR: char = ',';

/// One port of a list, with the comment written next to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PortEntry {
    pub port: u16,
    pub label: Option<String>,
    /// 1-based.
    pub line: u32,
}

/// The ports of every array (`@( ... )`, `name=( ... )`) and of every line
/// that holds only numbers, in text order.
pub(super) fn extract_port_list(lines: &[CommentedLine]) -> Vec<PortEntry> {
    let mut scanner = PortScanner::default();
    for line in lines {
        scanner.scan_line(line);
    }
    scanner.entries
}

#[derive(Default)]
struct PortScanner {
    entries: Vec<PortEntry>,
    is_inside_array: bool,
    labelable_entry: Option<usize>,
}

impl PortScanner {
    fn scan_line(&mut self, line: &CommentedLine) {
        let is_numeric = is_numeric_line(line);
        for piece in &line.pieces {
            match piece {
                Piece::Code(text) => self.scan_code(text, line.number, is_numeric),
                Piece::Comment { text, follows_code } => self.attach_label(text, *follows_code),
            }
        }
    }

    fn scan_code(&mut self, text: &str, line: u32, is_numeric: bool) {
        let mut word = String::new();
        let mut previous: Option<char> = None;
        for character in text.chars() {
            if character.is_whitespace() || character == LIST_SEPARATOR {
                self.finish_word(&mut word, line, is_numeric);
            } else if character == ARRAY_OPEN || character == ARRAY_CLOSE {
                self.finish_word(&mut word, line, is_numeric);
                self.track_array(character, previous);
            } else {
                word.push(character);
            }
            previous = Some(character);
        }
        self.finish_word(&mut word, line, is_numeric);
    }

    fn track_array(&mut self, parenthesis: char, previous: Option<char>) {
        if parenthesis == ARRAY_CLOSE {
            self.is_inside_array = false;
        } else if previous.is_some_and(|character| ARRAY_OPENERS.contains(&character)) {
            self.is_inside_array = true;
        }
    }

    fn finish_word(&mut self, word: &mut String, line: u32, is_numeric: bool) {
        let finished = std::mem::take(word);
        if finished.is_empty() {
            return;
        }
        match parse_port(&finished).filter(|_| self.is_inside_array || is_numeric) {
            Some(port) => {
                self.entries.push(PortEntry {
                    port,
                    label: None,
                    line,
                });
                self.labelable_entry = Some(self.entries.len() - 1);
            }
            None => self.labelable_entry = None,
        }
    }

    fn attach_label(&mut self, text: &str, follows_code: bool) {
        if !follows_code || text.is_empty() {
            return;
        }
        if let Some(entry) = self
            .labelable_entry
            .take()
            .and_then(|index| self.entries.get_mut(index))
        {
            entry.label.get_or_insert_with(|| text.to_owned());
        }
    }
}

fn parse_port(word: &str) -> Option<u16> {
    word.parse::<u16>().ok().filter(|port| *port != 0)
}

fn is_numeric_line(line: &CommentedLine) -> bool {
    let code: String = line
        .pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Code(text) => Some(text.as_str()),
            Piece::Comment { .. } => None,
        })
        .collect();
    code.chars().any(|character| character.is_ascii_digit())
        && code.chars().all(|character| {
            character.is_ascii_digit() || character.is_whitespace() || character == LIST_SEPARATOR
        })
}

#[cfg(test)]
mod tests {
    use super::super::comments::strip_comments;
    use super::*;

    fn ports(text: &str) -> Vec<u16> {
        extract_port_list(&strip_comments(text))
            .iter()
            .map(|entry| entry.port)
            .collect()
    }

    #[test]
    fn should_read_ports_when_powershell_array_spans_lines() {
        assert_eq!(ports("$p = @(\n  3333,\n  3334\n)"), vec![3333, 3334]);
    }

    #[test]
    fn should_read_ports_when_shell_array_is_used() {
        assert_eq!(ports("ports=(3333 3334)"), vec![3333, 3334]);
    }

    #[test]
    fn should_read_ports_when_line_holds_only_numbers() {
        assert_eq!(ports("3333 5432\n8080,9090"), vec![3333, 5432, 8080, 9090]);
    }

    #[test]
    fn should_ignore_numbers_when_they_are_outside_arrays_and_mixed_with_words() {
        assert_eq!(ports("ssh -p 22 nova"), Vec::<u16>::new());
    }

    #[test]
    fn should_ignore_numbers_when_they_are_out_of_range() {
        assert_eq!(ports("0 70000 22"), vec![22]);
    }

    #[test]
    fn should_label_port_when_block_comment_follows_it() {
        let entries = extract_port_list(&strip_comments("@(\n3333 <# adonis #>,\n)"));

        assert_eq!(entries[0].label.as_deref(), Some("adonis"));
    }

    #[test]
    fn should_label_port_when_line_comment_follows_it() {
        let entries = extract_port_list(&strip_comments("3333 # adonis"));

        assert_eq!(entries[0].label.as_deref(), Some("adonis"));
    }

    #[test]
    fn should_not_label_port_when_comment_is_on_the_next_line() {
        let entries = extract_port_list(&strip_comments("3333\n# unrelated"));

        assert_eq!(entries[0].label, None);
    }

    #[test]
    fn should_record_the_line_of_each_port() {
        let entries = extract_port_list(&strip_comments("@(\n3333,\n\n3334)"));

        assert_eq!(
            entries.iter().map(|entry| entry.line).collect::<Vec<_>>(),
            vec![2, 4]
        );
    }
}
