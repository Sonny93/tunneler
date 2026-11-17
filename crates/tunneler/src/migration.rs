//! Brings the tunnels of an earlier portable copy to the user's directories
//! when the daemon starts, and tells the user when it cannot.
//!
//! Nothing here moves, deletes or overwrites a file: the earlier copy stays
//! as it was, and an existing destination is left alone.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use engine::default_store_path;
use ipc::{PortableLayout, detect_portable_layout};
use thiserror::Error;

use crate::daemon::DaemonOptions;
use crate::earlier_copies::{
    ConfigurationNote, MigrationDecision, MigrationFacts, PORTABLE_MODE_DOCUMENTATION_URL,
    configuration_notes, decide_migration, find_earlier_portable_copies, find_ignored_user_file,
    is_trusted_file,
};
use crate::layout::DataLayout;

/// Why the files of an earlier copy could not be copied.
#[derive(Debug, Error)]
#[error("cannot copy {} to {}: {source}", .from.display(), .to.display())]
pub struct MigrationError {
    from: PathBuf,
    to: PathBuf,
    source: io::Error,
}

#[cfg(unix)]
const NO_FOLLOW_FLAG: i32 = rustix::fs::OFlags::NOFOLLOW.bits().cast_signed();
#[cfg(unix)]
const PRIVATE_FILE_MODE: u32 = 0o600;
#[cfg(unix)]
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyOutcome {
    Copied,
    DestinationExists,
    SourceMissing,
}

/// Runs before the daemon opens its files: copies the files of the only earlier
/// portable copy when nothing exists yet, warns when it cannot choose, and
/// warns when a portable copy ignores the user's tunnels file. Never fails
/// the start.
pub fn prepare_configuration(options: &DaemonOptions) {
    let Some(executable_directory) = running_executable_directory() else {
        return;
    };
    if detect_portable_layout().is_some() {
        warn_about_ignored_user_file(options);
        return;
    }
    let default_layout = match DataLayout::resolve() {
        Ok(layout) => layout,
        Err(error) => {
            tracing::warn!(%error, "cannot look for earlier portable copies");
            return;
        }
    };
    let is_migration_possible = options.store_path == default_layout.tunnels_file
        && options.state_path == default_layout.state_file
        && !options.store_path.exists();
    if !is_migration_possible {
        return;
    }
    let candidates = find_earlier_portable_copies(&executable_directory);
    let decision = decide_migration(&MigrationFacts {
        is_portable: false,
        has_location_override: false,
        is_default_tunnels_file_present: false,
        candidates: &candidates,
    });
    apply_decision(decision, options);
}

/// The notes to print under the configuration line of `tunneler status`.
pub fn status_notes(configuration_file: &Path) -> Vec<ConfigurationNote> {
    let candidates = if configuration_file.exists() {
        Vec::new()
    } else {
        running_executable_directory()
            .map_or_default(|directory| find_earlier_portable_copies(&directory))
    };
    let ignored_user_file = default_store_path().ok().and_then(|user_file| {
        find_ignored_user_file(detect_portable_layout().is_some(), &user_file)
    });
    configuration_notes(configuration_file.exists(), &candidates, ignored_user_file)
}

fn running_executable_directory() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    Some(executable.parent()?.to_path_buf())
}

fn warn_about_ignored_user_file(options: &DaemonOptions) {
    let Some(user_file) = default_store_path()
        .ok()
        .and_then(|user_file| find_ignored_user_file(true, &user_file))
    else {
        return;
    };
    tracing::warn!(
        read = %options.store_path.display(),
        ignored = %user_file.display(),
        "this copy is portable: it reads the tunnels file next to the program, not the one in the user's directory"
    );
}

fn apply_decision(decision: MigrationDecision, options: &DaemonOptions) {
    match decision {
        MigrationDecision::Nothing => {}
        MigrationDecision::Copy { source } => copy_and_report(&source, options),
        MigrationDecision::Ambiguous { candidates } => {
            let folders: Vec<String> = candidates
                .iter()
                .map(|folder| folder.display().to_string())
                .collect();
            tracing::warn!(
                ?folders,
                documentation = PORTABLE_MODE_DOCUMENTATION_URL,
                "several earlier portable copies hold tunnels: none was copied"
            );
        }
    }
}

fn copy_and_report(source: &Path, options: &DaemonOptions) {
    match copy_earlier_copy(source, &options.store_path, &options.state_path) {
        Ok(()) => tracing::info!(
            source = %source.display(),
            "copied the tunnels of an earlier portable copy"
        ),
        Err(error) => tracing::warn!(%error, "cannot copy the tunnels of an earlier portable copy"),
    }
}

/// Copies `tunnels.toml` of the portable copy in `source_directory`, then its
/// `data/state.toml` when it has one, to the given paths. A destination that
/// exists is left as it is, and the state is not copied without the tunnels.
fn copy_earlier_copy(
    source_directory: &Path,
    tunnels_destination: &Path,
    state_destination: &Path,
) -> Result<(), MigrationError> {
    let source = PortableLayout::at(source_directory);
    let tunnels_outcome = copy_without_overwriting(&source.tunnels_file(), tunnels_destination)?;
    if tunnels_outcome == CopyOutcome::Copied {
        copy_without_overwriting(&source.state_file(), state_destination)?;
    }
    Ok(())
}

fn copy_without_overwriting(
    source: &Path,
    destination: &Path,
) -> Result<CopyOutcome, MigrationError> {
    copy_file(source, destination).map_err(|error| MigrationError {
        from: source.to_path_buf(),
        to: destination.to_path_buf(),
        source: error,
    })
}

fn copy_file(source: &Path, destination: &Path) -> io::Result<CopyOutcome> {
    let mut input = match open_source(source) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(CopyOutcome::SourceMissing);
        }
        Err(error) => return Err(error),
    };
    if let Some(parent) = destination.parent() {
        create_private_directories(parent)?;
    }
    let mut output = match create_private_file(destination) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Ok(CopyOutcome::DestinationExists);
        }
        Err(error) => return Err(error),
    };
    if let Err(error) = fill_destination(&mut input, &mut output) {
        drop(output);
        remove_partial_copy(destination);
        return Err(error);
    }
    Ok(CopyOutcome::Copied)
}

/// Opens the source without following a symlink, and checks the handle that
/// is going to be read: a regular file that only the user can write.
fn open_source(source: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::custom_flags(&mut options, NO_FOLLOW_FLAG);
    let file = options.open(source)?;
    if !is_trusted_file(&file.metadata()?) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the file is not a regular file owned by the user and closed to others",
        ));
    }
    Ok(file)
}

fn create_private_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, PRIVATE_FILE_MODE);
    options.open(path)
}

fn create_private_directories(directory: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, PRIVATE_DIRECTORY_MODE);
    builder.create(directory)
}

fn fill_destination(input: &mut File, output: &mut File) -> io::Result<()> {
    io::copy(input, output)?;
    output.sync_all()
}

fn remove_partial_copy(destination: &Path) {
    if let Err(error) = fs::remove_file(destination) {
        tracing::warn!(%error, path = %destination.display(), "cannot remove a partial copy");
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::*;

    const SOURCE_TUNNELS: &str = "version = 1\n# earlier\n";
    const SOURCE_STATE: &str = "version = 1\n# earlier state\n";
    const NEIGHBOUR_CONTENT: &str = "version = 1\n# already here\n";

    struct Fixture {
        root: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                root: tempfile::tempdir().unwrap(),
            }
        }

        fn source(&self) -> PathBuf {
            self.root.path().join("0.5.0")
        }

        fn tunnels_destination(&self) -> PathBuf {
            self.root.path().join("config/tunneler/tunnels.toml")
        }

        fn state_destination(&self) -> PathBuf {
            self.root.path().join("state/tunneler/state.toml")
        }

        fn write_source_tunnels(&self) {
            fs::create_dir_all(self.source()).unwrap();
            write_with_mode(&self.source().join("tunnels.toml"), SOURCE_TUNNELS, 0o644);
        }

        fn write_source_state(&self) {
            fs::create_dir_all(self.source().join("data")).unwrap();
            write_with_mode(&self.source().join("data/state.toml"), SOURCE_STATE, 0o644);
        }

        fn copy(&self) -> Result<(), MigrationError> {
            copy_earlier_copy(
                &self.source(),
                &self.tunnels_destination(),
                &self.state_destination(),
            )
        }
    }

    #[cfg(unix)]
    fn write_with_mode(path: &Path, content: &str, mode: u32) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, content).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[cfg(not(unix))]
    fn write_with_mode(path: &Path, content: &str, _mode: u32) {
        fs::write(path, content).unwrap();
    }

    fn write_existing(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, NEIGHBOUR_CONTENT).unwrap();
    }

    #[test]
    fn should_copy_the_tunnels_and_the_state_creating_the_directories() {
        let fixture = Fixture::new();
        fixture.write_source_tunnels();
        fixture.write_source_state();

        fixture.copy().unwrap();

        assert_eq!(
            (
                fs::read_to_string(fixture.tunnels_destination()).unwrap(),
                fs::read_to_string(fixture.state_destination()).unwrap(),
            ),
            (SOURCE_TUNNELS.to_owned(), SOURCE_STATE.to_owned())
        );
    }

    #[test]
    fn should_leave_the_source_byte_identical_when_copying() {
        let fixture = Fixture::new();
        fixture.write_source_tunnels();
        fixture.write_source_state();

        fixture.copy().unwrap();

        assert_eq!(
            (
                fs::read_to_string(fixture.source().join("tunnels.toml")).unwrap(),
                fs::read_to_string(fixture.source().join("data/state.toml")).unwrap(),
            ),
            (SOURCE_TUNNELS.to_owned(), SOURCE_STATE.to_owned())
        );
    }

    #[test]
    fn should_copy_only_the_tunnels_when_the_source_has_no_state() {
        let fixture = Fixture::new();
        fixture.write_source_tunnels();

        fixture.copy().unwrap();

        assert_eq!(
            (
                fixture.tunnels_destination().is_file(),
                fixture.state_destination().exists()
            ),
            (true, false)
        );
    }

    #[test]
    fn should_keep_an_existing_tunnels_file_and_copy_no_state_when_the_tunnels_exist() {
        let fixture = Fixture::new();
        fixture.write_source_tunnels();
        fixture.write_source_state();
        write_existing(&fixture.tunnels_destination());

        fixture.copy().unwrap();

        assert_eq!(
            (
                fs::read_to_string(fixture.tunnels_destination()).unwrap(),
                fixture.state_destination().exists()
            ),
            (NEIGHBOUR_CONTENT.to_owned(), false)
        );
    }

    #[test]
    fn should_keep_an_existing_state_file_when_copying_the_tunnels() {
        let fixture = Fixture::new();
        fixture.write_source_tunnels();
        fixture.write_source_state();
        write_existing(&fixture.state_destination());

        fixture.copy().unwrap();

        assert_eq!(
            (
                fs::read_to_string(fixture.tunnels_destination()).unwrap(),
                fs::read_to_string(fixture.state_destination()).unwrap(),
            ),
            (SOURCE_TUNNELS.to_owned(), NEIGHBOUR_CONTENT.to_owned())
        );
    }

    #[test]
    fn should_report_the_files_when_the_source_cannot_be_read() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.source().join("tunnels.toml")).unwrap();

        let error = fixture.copy().unwrap_err();

        assert_eq!(
            (error.from, fixture.tunnels_destination().exists()),
            (fixture.source().join("tunnels.toml"), false)
        );
    }

    #[cfg(unix)]
    #[test]
    fn should_create_private_files_and_directories_whatever_the_mode_of_the_source() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Fixture::new();
        fixture.write_source_tunnels();
        write_with_mode(
            &fixture.source().join("tunnels.toml"),
            SOURCE_TUNNELS,
            0o644,
        );
        fixture.copy().unwrap();
        let mode_of = |path: PathBuf| fs::metadata(path).unwrap().permissions().mode();

        assert_eq!(
            (
                mode_of(fixture.tunnels_destination()) & 0o777,
                mode_of(
                    fixture
                        .tunnels_destination()
                        .parent()
                        .unwrap()
                        .to_path_buf()
                ) & 0o777,
            ),
            (0o600, 0o700)
        );
    }

    #[cfg(unix)]
    #[test]
    fn should_copy_nothing_when_the_source_is_writable_by_others() {
        let fixture = Fixture::new();
        fixture.write_source_tunnels();
        write_with_mode(
            &fixture.source().join("tunnels.toml"),
            SOURCE_TUNNELS,
            0o666,
        );

        let outcome = fixture.copy();

        assert_eq!(
            (outcome.is_err(), fixture.tunnels_destination().exists()),
            (true, false)
        );
    }

    #[cfg(unix)]
    #[test]
    fn should_copy_nothing_when_the_source_is_a_symlink() {
        let fixture = Fixture::new();
        fixture.write_source_tunnels();
        let real = fixture.source().join("real.toml");
        fs::rename(fixture.source().join("tunnels.toml"), &real).unwrap();
        std::os::unix::fs::symlink(&real, fixture.source().join("tunnels.toml")).unwrap();

        let outcome = fixture.copy();

        assert_eq!(
            (outcome.is_err(), fixture.tunnels_destination().exists()),
            (true, false)
        );
    }
}
