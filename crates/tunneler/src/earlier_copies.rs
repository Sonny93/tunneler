//! Earlier portable copies of the program, left next to the running one by an
//! installer that extracts every version into its own folder.
//!
//! The search and the decisions are pure: the callers gather the facts and act
//! on the answer.

use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};

use ipc::PORTABLE_MARKER_FILE_NAME;

/// The page that explains how to bring the tunnels of an earlier copy along.
pub const PORTABLE_MODE_DOCUMENTATION_URL: &str =
    "https://sonny93.github.io/tunneler/guide/updates-and-uninstall#portable-mode";

const TUNNELS_FILE_NAME: &str = "tunnels.toml";
#[cfg(unix)]
const GROUP_WRITE_BIT: u32 = 0o020;
#[cfg(unix)]
const OTHER_WRITE_BIT: u32 = 0o002;
#[cfg(unix)]
const STICKY_BIT: u32 = 0o1000;

/// The folders next to `executable_directory` that hold a portable copy with
/// tunnels: each is a real folder with both a `portable` file and a
/// `tunnels.toml`, none of them a symlink. On Unix the copy must also belong to
/// the user and be closed to others, inside a parent folder that others cannot
/// write to, so that nobody else can plant a copy. Sorted; empty when the
/// parent cannot be listed.
pub fn find_earlier_portable_copies(executable_directory: &Path) -> Vec<PathBuf> {
    let Some(parent) = executable_directory.parent() else {
        return Vec::new();
    };
    if !is_closed_parent(parent) {
        return Vec::new();
    }
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
    is_trusted_entry(directory, is_trusted_directory)
        && is_trusted_entry(&directory.join(PORTABLE_MARKER_FILE_NAME), is_trusted_file)
        && is_trusted_entry(&directory.join(TUNNELS_FILE_NAME), is_trusted_file)
}

fn is_trusted_entry(path: &Path, is_trusted: fn(&Metadata) -> bool) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| is_trusted(&metadata))
}

#[cfg(unix)]
fn is_closed_parent(parent: &Path) -> bool {
    fs::metadata(parent).is_ok_and(|metadata| is_closed_to_others(&Access::of(&metadata), true))
}

#[cfg(not(unix))]
fn is_closed_parent(_parent: &Path) -> bool {
    true
}

/// Whether the metadata is of a real directory (not a symlink) that belongs
/// to the user and that others cannot write to.
#[cfg(unix)]
pub fn is_trusted_directory(metadata: &Metadata) -> bool {
    metadata.is_dir() && is_owned_and_closed(metadata)
}

#[cfg(not(unix))]
pub fn is_trusted_directory(metadata: &Metadata) -> bool {
    metadata.is_dir()
}

/// Whether the metadata is of a regular file (not a symlink) that belongs to
/// the user and that others cannot write to.
#[cfg(unix)]
pub fn is_trusted_file(metadata: &Metadata) -> bool {
    metadata.is_file() && is_owned_and_closed(metadata)
}

#[cfg(not(unix))]
pub fn is_trusted_file(metadata: &Metadata) -> bool {
    metadata.is_file()
}

#[cfg(unix)]
fn is_owned_and_closed(metadata: &Metadata) -> bool {
    let access = Access::of(metadata);
    access.uid == access.euid && is_closed_to_others(&access, false)
}

/// Who owns an entry and who may write to it, as the permission decision
/// needs it.
#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
struct Access {
    mode: u32,
    uid: u32,
    gid: u32,
    euid: u32,
    egid: u32,
}

#[cfg(unix)]
impl Access {
    fn of(metadata: &Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;

        Self {
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            euid: rustix::process::geteuid().as_raw(),
            egid: rustix::process::getegid().as_raw(),
        }
    }
}

/// Whether nobody but the user can write to the entry. Other-write is always
/// refused, and so is the sticky bit when `refuses_sticky` is set (a shared
/// folder such as `/tmp`). Group-write is refused too, except for a user
/// private group: many systems give each user a group of their own and a
/// umask of 002, so their folders are group-writable without anyone else in
/// the group. The group is taken as private only when the entry's group id,
/// its owner's id, the effective uid and the process's effective gid are all
/// the same number. A shared group whose id merely equals the user's id (such
/// as group 100 for the user 100) has a different effective gid and stays
/// refused.
#[cfg(unix)]
fn is_closed_to_others(access: &Access, refuses_sticky: bool) -> bool {
    let has_private_group =
        access.uid == access.euid && access.gid == access.uid && access.egid == access.euid;
    let mut refused_bits = OTHER_WRITE_BIT;
    if !has_private_group {
        refused_bits |= GROUP_WRITE_BIT;
    }
    if refuses_sticky {
        refused_bits |= STICKY_BIT;
    }
    access.mode & refused_bits == 0
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
        set_mode(&directory, 0o755);
        if has_marker {
            write_file(&directory.join(PORTABLE_MARKER_FILE_NAME), "");
        }
        if has_tunnels {
            write_file(&directory.join(TUNNELS_FILE_NAME), "version = 1\n");
        }
        directory
    }

    fn closed_tempdir() -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        set_mode(directory.path(), 0o700);
        directory
    }

    fn write_file(path: &Path, content: &str) {
        fs::write(path, content).unwrap();
        set_mode(path, 0o644);
    }

    #[cfg(unix)]
    fn set_mode(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[cfg(not(unix))]
    fn set_mode(_path: &Path, _mode: u32) {}

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
        let parent = closed_tempdir();
        let earlier = copy_in(parent.path(), "0.5.0", true, true);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(find_earlier_portable_copies(&current), vec![earlier]);
    }

    #[test]
    fn should_ignore_a_sibling_without_the_marker() {
        let parent = closed_tempdir();
        copy_in(parent.path(), "0.5.0", false, true);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn should_ignore_a_sibling_without_the_tunnels_file() {
        let parent = closed_tempdir();
        copy_in(parent.path(), "0.5.0", true, false);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn should_exclude_the_directory_of_the_executable() {
        let parent = closed_tempdir();
        let current = copy_in(parent.path(), "0.6.0", true, true);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn should_sort_the_copies_when_several_are_found() {
        let parent = closed_tempdir();
        let newer = copy_in(parent.path(), "0.5.0", true, true);
        let older = copy_in(parent.path(), "0.4.0", true, true);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(find_earlier_portable_copies(&current), vec![older, newer]);
    }

    #[cfg(unix)]
    #[test]
    fn should_ignore_a_symlink_to_a_copy_and_find_the_copy_once() {
        let parent = closed_tempdir();
        let earlier = copy_in(parent.path(), "0.5.0", true, true);
        std::os::unix::fs::symlink("./0.5.0", parent.path().join("0.5")).unwrap();
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(find_earlier_portable_copies(&current), vec![earlier]);
    }

    #[cfg(unix)]
    #[test]
    fn should_find_nothing_when_the_only_sibling_is_a_symlink_to_a_copy() {
        let elsewhere = closed_tempdir();
        let target = copy_in(elsewhere.path(), "0.5.0", true, true);
        let parent = closed_tempdir();
        std::os::unix::fs::symlink(&target, parent.path().join("0.5.0")).unwrap();
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
    }

    #[cfg(unix)]
    #[test]
    fn should_find_nothing_when_the_parent_is_writable_by_others() {
        let parent = closed_tempdir();
        copy_in(parent.path(), "0.5.0", true, true);
        let current = copy_in(parent.path(), "0.6.0", false, false);
        set_mode(parent.path(), 0o1777);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
    }

    #[cfg(unix)]
    fn access(mode: u32, uid: u32, gid: u32) -> Access {
        Access {
            mode,
            uid,
            gid,
            euid: 1000,
            egid: 1000,
        }
    }

    #[cfg(unix)]
    #[test]
    fn should_accept_a_group_writable_folder_when_the_group_is_the_private_group_of_the_user() {
        assert!(is_closed_to_others(&access(0o40775, 1000, 1000), false));
    }

    #[cfg(unix)]
    #[test]
    fn should_refuse_a_group_writable_folder_when_the_group_is_shared() {
        assert!(!is_closed_to_others(&access(0o40775, 1000, 100), false));
    }

    #[cfg(unix)]
    #[test]
    fn should_refuse_a_group_writable_folder_when_the_effective_group_of_the_user_differs() {
        let access = Access {
            egid: 100,
            ..access(0o40775, 1000, 1000)
        };

        assert!(!is_closed_to_others(&access, false));
    }

    #[cfg(unix)]
    #[test]
    fn should_refuse_a_group_writable_folder_when_the_owner_is_not_the_user() {
        assert!(!is_closed_to_others(&access(0o40775, 1001, 1001), false));
    }

    #[cfg(unix)]
    #[test]
    fn should_refuse_other_write_even_with_a_private_group() {
        assert!(!is_closed_to_others(&access(0o40757, 1000, 1000), false));
    }

    #[cfg(unix)]
    #[test]
    fn should_refuse_the_sticky_bit_only_for_the_parent() {
        let access = access(0o41755, 1000, 1000);

        assert_eq!(
            (
                is_closed_to_others(&access, true),
                is_closed_to_others(&access, false)
            ),
            (false, true)
        );
    }

    /// Runs only when the test user's primary group id equals its user id.
    #[cfg(unix)]
    #[test]
    fn should_find_a_sibling_folder_group_writable_when_the_user_has_a_private_group() {
        if rustix::process::getegid().as_raw() != rustix::process::geteuid().as_raw() {
            return;
        }
        let parent = closed_tempdir();
        let earlier = copy_in(parent.path(), "0.5.0", true, true);
        set_mode(&earlier, 0o775);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(find_earlier_portable_copies(&current), vec![earlier]);
    }

    #[cfg(unix)]
    #[test]
    fn should_ignore_a_sibling_folder_writable_by_others() {
        let parent = closed_tempdir();
        let earlier = copy_in(parent.path(), "0.5.0", true, true);
        set_mode(&earlier, 0o757);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
    }

    #[cfg(unix)]
    #[test]
    fn should_ignore_a_tunnels_file_writable_by_others() {
        let parent = closed_tempdir();
        let earlier = copy_in(parent.path(), "0.5.0", true, true);
        set_mode(&earlier.join(TUNNELS_FILE_NAME), 0o666);
        let current = copy_in(parent.path(), "0.6.0", false, false);

        assert_eq!(
            find_earlier_portable_copies(&current),
            Vec::<PathBuf>::new()
        );
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
        let directory = closed_tempdir();
        let user_file = directory.path().join(TUNNELS_FILE_NAME);
        fs::write(&user_file, "").unwrap();

        assert_eq!(
            find_ignored_user_file(true, &user_file),
            Some(user_file.clone())
        );
    }

    #[test]
    fn should_not_find_an_ignored_user_file_when_not_portable() {
        let directory = closed_tempdir();
        let user_file = directory.path().join(TUNNELS_FILE_NAME);
        fs::write(&user_file, "").unwrap();

        assert_eq!(find_ignored_user_file(false, &user_file), None);
    }
}
