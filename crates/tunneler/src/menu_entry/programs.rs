//! Which programs the menu entry may point at.

use std::path::{Path, PathBuf};

const UI_PROGRAM_NAME: &str = "tunneler-ui";
const PACKAGE_MANAGED_ROOT: &str = "/usr";
const MANUAL_INSTALL_ROOT: &str = "/usr/local";

/// The `tunneler-ui` program that sits next to `executable`, when there is one.
pub(super) fn find_ui_program(executable: &Path) -> Option<PathBuf> {
    let candidate = executable.parent()?.join(UI_PROGRAM_NAME);
    candidate.is_file().then_some(candidate)
}

/// The file `find_ui_program` looks for, for the error message.
pub(super) fn expected_ui_program(executable: &Path) -> PathBuf {
    executable
        .parent()
        .unwrap_or(Path::new("."))
        .join(UI_PROGRAM_NAME)
}

/// Whether the executable lives where a package manager puts programs: the
/// package ships its own menu entry and owns that path. `/usr/local` is where
/// people install by hand, so it does not count.
pub(super) fn is_package_managed(executable: &Path) -> bool {
    executable.starts_with(PACKAGE_MANAGED_ROOT) && !executable.starts_with(MANUAL_INSTALL_ROOT)
}

/// Whether the path has a character that could end the line of the entry and
/// start another key.
pub(super) fn has_control_character(path: &Path) -> bool {
    path.to_string_lossy().chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_find_the_ui_program_when_it_sits_next_to_the_executable() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("tunneler-ui"), "").unwrap();

        let found = find_ui_program(&directory.path().join("tunneler"));

        assert_eq!(found, Some(directory.path().join("tunneler-ui")));
    }

    #[test]
    fn should_not_find_the_ui_program_when_it_is_missing() {
        let directory = tempfile::tempdir().unwrap();

        assert_eq!(find_ui_program(&directory.path().join("tunneler")), None);
    }

    #[test]
    fn should_see_a_path_under_usr_as_managed_by_a_package() {
        assert!(is_package_managed(Path::new("/usr/bin/tunneler")));
    }

    #[test]
    fn should_not_see_a_path_under_usr_local_as_managed_by_a_package() {
        assert!(!is_package_managed(Path::new("/usr/local/bin/tunneler")));
    }

    #[test]
    fn should_see_a_newline_or_a_carriage_return_as_a_control_character() {
        assert!(has_control_character(Path::new("/a\nb/tunneler-ui")));
        assert!(has_control_character(Path::new("/a\rb/tunneler-ui")));
    }

    #[test]
    fn should_not_see_a_space_as_a_control_character() {
        assert!(!has_control_character(Path::new("/my apps/tunneler-ui")));
    }

    #[test]
    fn should_not_see_a_path_in_the_home_directory_as_managed_by_a_package() {
        assert!(!is_package_managed(Path::new(
            "/home/me/.local/share/mise/installs/tunneler/0.5.0/tunneler"
        )));
    }

    #[test]
    fn should_not_see_a_sibling_of_usr_as_managed_by_a_package() {
        assert!(!is_package_managed(Path::new("/usrlocal/tunneler")));
    }
}
