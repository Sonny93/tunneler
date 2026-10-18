//! What to show in the file manager for the configuration file.

use std::path::{Path, PathBuf};

/// How to show the configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevealTarget {
    /// Select the file in its folder.
    File(PathBuf),
    /// The file does not exist yet: open the folder that will hold it.
    ParentFolder(PathBuf),
}

/// Picks the file when it exists, else its parent folder. The folder itself
/// is never created: a missing folder fails when opened.
pub fn reveal_target(config_file: &Path, exists: impl Fn(&Path) -> bool) -> RevealTarget {
    if exists(config_file) {
        return RevealTarget::File(config_file.to_path_buf());
    }
    match config_file.parent() {
        Some(parent) => RevealTarget::ParentFolder(parent.to_path_buf()),
        None => RevealTarget::File(config_file.to_path_buf()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG_FILE: &str = "/home/me/.config/tunneler/tunnels.toml";

    #[test]
    fn should_reveal_the_file_when_it_exists() {
        let target = reveal_target(Path::new(CONFIG_FILE), |_| true);

        assert_eq!(target, RevealTarget::File(PathBuf::from(CONFIG_FILE)));
    }

    #[test]
    fn should_open_the_parent_folder_when_the_file_does_not_exist() {
        let target = reveal_target(Path::new(CONFIG_FILE), |_| false);

        assert_eq!(
            target,
            RevealTarget::ParentFolder(PathBuf::from("/home/me/.config/tunneler"))
        );
    }
}
