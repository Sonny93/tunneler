//! Where a temporary tunnel comes from, when it comes from devslot.

use super::project_tag::{DEVSLOT_TAG, MAX_PROJECT_TAG_BYTES, PROJECT_TAG_PREFIX};
use crate::store::find_tag_problem;

/// Marks a temporary tunnel as started from a devslot listing.
///
/// It lives only in the daemon, next to the tunnel: it is never written to
/// the store nor to the state file. It is how a resync recognises the
/// temporary tunnels it may remove, and what saving the tunnel turns into
/// tags.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DevslotOrigin {
    /// The tag of the project of the instance, see
    /// [`devslot_project_tag`](crate::devslot_project_tag).
    pub project_tag: Option<String>,
}

impl DevslotOrigin {
    /// Whether the project tag, when there is one, is a valid tag that
    /// starts with `ds-` and fits in 64 bytes.
    pub(crate) fn is_valid(&self) -> bool {
        self.project_tag.as_deref().is_none_or(|tag| {
            tag.starts_with(PROJECT_TAG_PREFIX)
                && tag.len() <= MAX_PROJECT_TAG_BYTES
                && find_tag_problem(tag).is_none()
        })
    }

    /// The tags a saved tunnel of this origin carries: `devslot`, then the
    /// project tag.
    pub(crate) fn tags(&self) -> Vec<String> {
        std::iter::once(DEVSLOT_TAG.to_owned())
            .chain(self.project_tag.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(project_tag: Option<&str>) -> DevslotOrigin {
        DevslotOrigin {
            project_tag: project_tag.map(str::to_owned),
        }
    }

    #[test]
    fn should_tag_with_devslot_and_the_project_when_there_is_one() {
        assert_eq!(
            origin(Some("ds-web-shop")).tags(),
            ["devslot", "ds-web-shop"]
        );
    }

    #[test]
    fn should_tag_with_devslot_alone_when_there_is_no_project() {
        assert_eq!(origin(None).tags(), ["devslot"]);
    }

    #[test]
    fn should_be_valid_when_there_is_no_project_tag() {
        assert!(origin(None).is_valid());
    }

    #[test]
    fn should_be_valid_when_the_project_tag_starts_with_the_prefix() {
        assert!(origin(Some("ds-web-shop")).is_valid());
    }

    #[test]
    fn should_be_invalid_when_the_project_tag_lacks_the_prefix() {
        assert!(!origin(Some("web-shop")).is_valid());
    }

    #[test]
    fn should_be_invalid_when_the_project_tag_is_not_a_tag() {
        assert!(!origin(Some("ds-web shop")).is_valid());
    }

    #[test]
    fn should_be_invalid_when_the_project_tag_is_longer_than_64_bytes() {
        assert!(!origin(Some(&format!("ds-{}", "a".repeat(62)))).is_valid());
    }
}
