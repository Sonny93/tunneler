//! The tag that groups the saved tunnels of one devslot project.

/// The tag carried by every devslot tunnel.
pub(crate) const DEVSLOT_TAG: &str = "devslot";

/// What every project tag starts with.
pub(crate) const PROJECT_TAG_PREFIX: &str = "ds-";

/// The longest a project tag may be, in bytes.
pub(crate) const MAX_PROJECT_TAG_BYTES: usize = 64;

/// The tag of a devslot project: `ds-` and its name, each character outside
/// `[A-Za-z0-9._-]` becoming `-`, cut so that the whole tag is at most 64
/// bytes. There is none when the project name is empty.
pub fn devslot_project_tag(project: &str) -> Option<String> {
    if project.is_empty() {
        return None;
    }
    let budget = MAX_PROJECT_TAG_BYTES - PROJECT_TAG_PREFIX.len();
    let sanitized: String = project
        .chars()
        .map(|character| {
            if is_tag_character(character) {
                character
            } else {
                '-'
            }
        })
        .take(budget)
        .collect();
    Some(format!("{PROJECT_TAG_PREFIX}{sanitized}"))
}

fn is_tag_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_prefix_the_project_when_its_name_is_plain() {
        assert_eq!(
            devslot_project_tag("web-shop"),
            Some("ds-web-shop".to_owned())
        );
    }

    #[test]
    fn should_replace_a_space_with_a_dash() {
        assert_eq!(
            devslot_project_tag("my project"),
            Some("ds-my-project".to_owned())
        );
    }

    #[test]
    fn should_give_a_tag_that_is_not_only_digits_when_the_project_is_a_number() {
        assert_eq!(devslot_project_tag("2024"), Some("ds-2024".to_owned()));
    }

    #[test]
    fn should_replace_a_colon_with_a_dash() {
        assert_eq!(devslot_project_tag("a:b"), Some("ds-a-b".to_owned()));
    }

    #[test]
    fn should_replace_each_unicode_character_with_one_dash() {
        assert_eq!(
            devslot_project_tag("café日本"),
            Some("ds-caf---".to_owned())
        );
    }

    #[test]
    fn should_give_no_tag_when_the_project_is_empty() {
        assert_eq!(devslot_project_tag(""), None);
    }

    #[test]
    fn should_cut_the_tag_to_64_bytes_when_the_project_is_very_long() {
        let tag = devslot_project_tag(&"a".repeat(500)).unwrap();

        assert_eq!(tag.len(), MAX_PROJECT_TAG_BYTES);
    }

    #[test]
    fn should_keep_dots_and_underscores() {
        assert_eq!(devslot_project_tag("a.b_c"), Some("ds-a.b_c".to_owned()));
    }

    #[test]
    fn should_give_a_tag_the_store_accepts_when_the_project_is_odd() {
        let tag = devslot_project_tag("my project: 2024 / café").unwrap();

        assert_eq!(crate::store::find_tag_problem(&tag), None);
    }
}
