//! Earlier portable copies of the program, left next to the running one by an
//! installer that extracts every version into its own folder.
//!
//! The search and the decisions are pure: the callers gather the facts and act
//! on the answer.

use std::fs;
use std::path::{Path, PathBuf};

use ipc::PORTABLE_MARKER_FILE_NAME;

/// The page that explains how to bring the tunnels of an earlier copy along.
pub const PORTABLE_MODE_DOCUMENTATION_URL: &str =
    "https://sonny93.github.io/tunneler/guide/updates-and-uninstall#portable-mode";

const TUNNELS_FILE_NAME: &str = "tunnels.toml";

/// The folders next to `executable_directory` that hold a portable copy with
/// tunnels: each has both a `portable` file and a `tunnels.toml`. Sorted;
/// empty when the parent cannot be listed.
pub fn find_earlier_portable_copies(executable_directory: &Path) -> Vec<PathBuf> {
    let Some(parent) = executable_directory.parent() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut copies: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path != executable_directory)
        .filter(|path| holds_a_portable_copy(path))
        .collect();
    copies.sort();
    copies
}

fn holds_a_portable_copy(directory: &Path) -> bool {
    directory.join(PORTABLE_MARKER_FILE_NAME).is_file()
        && directory.join(TUNNELS_FILE_NAME).is_file()
}

/// What is known when the daemon starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationFacts<'a> {
    /// Whether the marker sits next to the executable.
    pub is_portable: bool,
    /// Whether the tunnels or the state path differs from the default layout
    /// (`--config`, `--state` or their environment variables).
    pub has_location_override: bool,
    /// Whether the default tunnels file already exists.
    pub is_default_tunnels_file_present: bool,
    /// The result of [`find_earlier_portable_copies`].
    pub candidates: &'a [PathBuf],
}

/// What the daemon does about earlier portable copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationDecision {
    /// Leave everything alone.
    Nothing,
    /// Copy the files of this copy to the default locations.
    Copy {
        /// The folder of the earlier copy.
        source: PathBuf,
    },
    /// Several copies qualify: copy nothing, tell the user.
    Ambiguous {
        /// The folders of the earlier copies.
        candidates: Vec<PathBuf>,
    },
}

/// Decides what to do at daemon start.
pub fn decide_migration(facts: &MigrationFacts) -> MigrationDecision {
    if facts.is_portable || facts.has_location_override || facts.is_default_tunnels_file_present {
        return MigrationDecision::Nothing;
    }
    match facts.candidates {
        [] => MigrationDecision::Nothing,
        [source] => MigrationDecision::Copy {
            source: source.clone(),
        },
        candidates => MigrationDecision::Ambiguous {
            candidates: candidates.to_vec(),
        },
    }
}

/// Something the user should know about where the configuration comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigurationNote {
    /// The configuration file in use does not exist but earlier portable
    /// copies do.
    EarlierCopies(Vec<PathBuf>),
    /// The program is portable and the user's tunnels file exists but is not
    /// read.
    UserFileIgnored(PathBuf),
}

/// The user's tunnels file when the program is portable and the file exists:
/// the file that is there but is not read.
pub fn find_ignored_user_file(is_portable: bool, user_tunnels_file: &Path) -> Option<PathBuf> {
    (is_portable && user_tunnels_file.is_file()).then(|| user_tunnels_file.to_path_buf())
}

/// The notes `tunneler status` adds under the configuration line.
pub fn configuration_notes(
    is_configuration_file_present: bool,
    candidates: &[PathBuf],
    ignored_user_file: Option<PathBuf>,
) -> Vec<ConfigurationNote> {
    let earlier_copies = (!is_configuration_file_present && !candidates.is_empty())
        .then(|| ConfigurationNote::EarlierCopies(candidates.to_vec()));
    earlier_copies
        .into_iter()
        .chain(ignored_user_file.map(ConfigurationNote::UserFileIgnored))
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn copy_in(parent: &Path, name: &str, has_marker: bool, has_tunnels: bool) -> PathBuf {
        let directory = parent.join(name);
        fs::create_dir_all(&directory).unwrap();
        if has_marker {
            fs::write(directory.join(PORTABLE_MARKER_FILE_NAME), "").unwrap();
        }
        if has_tunnels {
            fs::write(directory.join(TUNNELS_FILE_NAME), "version = 1\n").unwrap();
        }
        directory
    }

    fn facts(candidates: &[PathBuf]) -> MigrationFacts<'_> {
        MigrationFacts {
            is_portable: false,
            has_location_override: false,
            is_default_tunnels_file_present: false,
            candidates,
        }
    }

    #[test]
    fn should_find_a_sibling_that_has_the_marker_and_the_tunnels_file() {
        let parent = tempfile::tempdir().unwrap();
        let earlier = copy_in(parent.path(), "0.5.0", true, true);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(find_earlier_portable_copies(&current), vec![earlier]);
    }

    #[test]
    fn should_ignore_a_sibling_without_the_marker() {
        let parent = tempfile::tempdir().unwrap();
        copy_in(parent.path(), "0.5.0", false, true);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn should_ignore_a_sibling_without_the_tunnels_file() {
        let parent = tempfile::tempdir().unwrap();
        copy_in(parent.path(), "0.5.0", true, false);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn should_exclude_the_directory_of_the_executable() {
        let parent = tempfile::tempdir().unwrap();
        let current = copy_in(parent.path(), "0.6.0", true, true);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn should_sort_the_copies_when_several_are_found() {
        let parent = tempfile::tempdir().unwrap();
        let newer = copy_in(parent.path(), "0.5.0", true, true);
        let older = copy_in(parent.path(), "0.4.0", true, true);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(find_earlier_portable_copies(&current), vec![older, newer]);
    }

    #[test]
    fn should_find_nothing_when_the_parent_cannot_be_listed() {
        assert_eq!(
            find_earlier_portable_copies(Path::new("/nonexistent-parent/current")),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn should_copy_nothing_when_no_copy_exists() {
        assert_eq!(decide_migration(&facts(&[])), MigrationDecision::Nothing);
    }

    #[test]
    fn should_copy_the_only_candidate_when_nothing_blocks_it() {
        let candidates = [PathBuf::from("/opt/0.5.0")];

        assert_eq!(
            decide_migration(&facts(&candidates)),
            MigrationDecision::Copy {
                source: PathBuf::from("/opt/0.5.0")
            }
        );
    }

    #[test]
    fn should_not_migrate_when_the_program_is_portable() {
        let candidates = [PathBuf::from("/opt/0.5.0")];

        let decision = decide_migration(&MigrationFacts {
            is_portable: true,
            ..facts(&candidates)
        });

        assert_eq!(decision, MigrationDecision::Nothing);
    }

    #[test]
    fn should_not_migrate_when_a_location_is_overridden() {
        let candidates = [PathBuf::from("/opt/0.5.0")];

        let decision = decide_migration(&MigrationFacts {
            has_location_override: true,
            ..facts(&candidates)
        });

        assert_eq!(decision, MigrationDecision::Nothing);
    }

    #[test]
    fn should_not_migrate_when_the_default_tunnels_file_exists() {
        let candidates = [PathBuf::from("/opt/0.5.0")];

        let decision = decide_migration(&MigrationFacts {
            is_default_tunnels_file_present: true,
            ..facts(&candidates)
        });

        assert_eq!(decision, MigrationDecision::Nothing);
    }

    #[test]
    fn should_report_the_candidates_when_several_exist() {
        let candidates = [PathBuf::from("/opt/0.4.0"), PathBuf::from("/opt/0.5.0")];

        assert_eq!(
            decide_migration(&facts(&candidates)),
            MigrationDecision::Ambiguous {
                candidates: candidates.to_vec()
            }
        );
    }

    #[test]
    fn should_note_the_earlier_copies_when_the_configuration_file_is_missing() {
        let candidates = [PathBuf::from("/opt/0.5.0")];

        assert_eq!(
            configuration_notes(false, &candidates, None),
            vec![ConfigurationNote::EarlierCopies(candidates.to_vec())]
        );
    }

    #[test]
    fn should_not_note_the_earlier_copies_when_the_configuration_file_exists() {
        let candidates = [PathBuf::from("/opt/0.5.0")];

        assert_eq!(configuration_notes(true, &candidates, None), Vec::new());
    }

    #[test]
    fn should_note_the_user_file_when_one_is_ignored() {
        let user_file = PathBuf::from("/home/me/.config/tunneler/tunnels.toml");

        assert_eq!(
            configuration_notes(true, &[], Some(user_file.clone())),
            vec![ConfigurationNote::UserFileIgnored(user_file)]
        );
    }

    #[test]
    fn should_find_the_user_file_when_portable_and_the_file_exists() {
        let directory = tempfile::tempdir().unwrap();
        let user_file = directory.path().join(TUNNELS_FILE_NAME);
        fs::write(&user_file, "").unwrap();

        assert_eq!(
            find_ignored_user_file(true, &user_file),
            Some(user_file.clone())
        );
    }

    #[test]
    fn should_not_find_an_ignored_user_file_when_not_portable() {
        let directory = tempfile::tempdir().unwrap();
        let user_file = directory.path().join(TUNNELS_FILE_NAME);
        fs::write(&user_file, "").unwrap();

        assert_eq!(find_ignored_user_file(false, &user_file), None);
    }
}
