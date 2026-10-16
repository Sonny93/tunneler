use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::error::Error;

const TEMP_SUFFIX_BYTES: usize = std::mem::size_of::<u64>();
#[cfg(unix)]
const NEW_FILE_MODE: u32 = 0o600;
#[cfg(unix)]
const NEW_DIRECTORY_MODE: u32 = 0o700;

/// Reads a file; a missing file reads as empty.
pub(crate) fn read_or_empty(path: &Path) -> Result<Vec<u8>, Error> {
    match fs::read(path) {
        Ok(content) => Ok(content),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(io_error(path, source)),
    }
}

/// Replaces the content of `path` atomically: write a temporary file next to
/// it, fsync, rename. Symlinks are written through, an existing file keeps its
/// permissions, a new file is private to the user (`0600` on Unix) and missing
/// parent directories are created private (`0700` on Unix).
pub(crate) fn replace_contents(path: &Path, content: &[u8]) -> Result<(), Error> {
    let target = resolve_symlink(path);
    let directory = parent_directory(&target);
    create_directories(directory).map_err(|source| io_error(directory, source))?;
    let temp = TempFile::create(directory).map_err(|source| io_error(directory, source))?;
    temp.write_replacement_of(&target, content)
        .map_err(|source| io_error(&target, source))?;
    temp.persist_as(&target)
        .map_err(|source| io_error(&target, source))
}

fn io_error(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn resolve_symlink(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn parent_directory(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn create_directories(directory: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, NEW_DIRECTORY_MODE);
    builder.create(directory)
}

/// A temporary file removed on drop unless it was renamed into place.
struct TempFile {
    path: PathBuf,
    file: File,
    is_persisted: bool,
}

impl TempFile {
    fn create(directory: &Path) -> io::Result<Self> {
        let path = directory.join(format!(".known_hosts.{}.tmp", random_suffix()?));
        let file = create_private_file(&path)?;
        Ok(Self {
            path,
            file,
            is_persisted: false,
        })
    }

    fn write_replacement_of(&self, target: &Path, content: &[u8]) -> io::Result<()> {
        if let Some(permissions) = existing_permissions(target)? {
            self.file.set_permissions(permissions)?;
        }
        let mut file = &self.file;
        file.write_all(content)?;
        file.sync_all()
    }

    fn persist_as(mut self, target: &Path) -> io::Result<()> {
        fs::rename(&self.path, target)?;
        self.is_persisted = true;
        Ok(())
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.is_persisted {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn existing_permissions(path: &Path) -> io::Result<Option<fs::Permissions>> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata.permissions())),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(source),
    }
}

fn create_private_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, NEW_FILE_MODE);
    options.open(path)
}

fn random_suffix() -> io::Result<String> {
    let mut bytes = [0_u8; TEMP_SUFFIX_BYTES];
    getrandom::fill(&mut bytes).map_err(io::Error::other)?;
    Ok(format!("{:016x}", u64::from_le_bytes(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn directory_entries(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn should_create_file_when_target_is_missing() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("known_hosts");

        replace_contents(&target, b"content").unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"content");
    }

    #[test]
    fn should_replace_content_when_target_exists() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("known_hosts");
        fs::write(&target, "old").unwrap();

        replace_contents(&target, b"new").unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"new");
    }

    #[test]
    fn should_leave_no_temporary_file_when_replacement_succeeds() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("known_hosts");

        replace_contents(&target, b"content").unwrap();

        assert_eq!(directory_entries(directory.path()), vec!["known_hosts"]);
    }

    #[test]
    fn should_read_as_empty_when_file_is_missing() {
        let directory = tempfile::tempdir().unwrap();

        assert_eq!(
            read_or_empty(&directory.path().join("absent")).unwrap(),
            b""
        );
    }

    #[test]
    fn should_create_missing_parent_directories_when_target_is_nested() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("a").join("b").join("known_hosts");

        replace_contents(&target, b"content").unwrap();

        assert!(target.is_file());
    }

    #[cfg(unix)]
    mod unix {
        use super::*;
        use pretty_assertions::assert_eq;
        use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

        const MODE_MASK: u32 = 0o777;

        fn mode_of(path: &Path) -> u32 {
            fs::metadata(path).unwrap().mode() & MODE_MASK
        }

        #[test]
        fn should_create_private_file_when_target_is_missing() {
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join("known_hosts");

            replace_contents(&target, b"content").unwrap();

            assert_eq!(mode_of(&target), 0o600);
        }

        #[test]
        fn should_create_private_parent_directory_when_it_is_missing() {
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join(".ssh").join("known_hosts");

            replace_contents(&target, b"content").unwrap();

            assert_eq!(mode_of(&directory.path().join(".ssh")), 0o700);
        }

        #[test]
        fn should_preserve_permissions_when_target_exists() {
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join("known_hosts");
            fs::write(&target, "old").unwrap();
            fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();

            replace_contents(&target, b"new").unwrap();

            assert_eq!(mode_of(&target), 0o644);
        }

        #[test]
        fn should_write_through_symlink_when_target_is_a_symlink() {
            let directory = tempfile::tempdir().unwrap();
            let real = directory.path().join("real");
            let link = directory.path().join("link");
            fs::write(&real, "old").unwrap();
            symlink(&real, &link).unwrap();

            replace_contents(&link, b"new").unwrap();

            assert_eq!(fs::read(&real).unwrap(), b"new");
        }

        #[test]
        fn should_keep_symlink_when_target_is_a_symlink() {
            let directory = tempfile::tempdir().unwrap();
            let real = directory.path().join("real");
            let link = directory.path().join("link");
            fs::write(&real, "old").unwrap();
            symlink(&real, &link).unwrap();

            replace_contents(&link, b"new").unwrap();

            assert!(
                fs::symlink_metadata(&link)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }

        #[test]
        fn should_remove_temporary_file_when_rename_fails() {
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join("known_hosts");
            fs::create_dir(&target).unwrap();
            fs::write(target.join("occupant"), "").unwrap();

            let outcome = replace_contents(&target, b"content");

            assert!(outcome.is_err());
            assert_eq!(directory_entries(directory.path()), vec!["known_hosts"]);
        }

        #[test]
        fn should_report_error_with_path_when_directory_is_not_writable() {
            let directory = tempfile::tempdir().unwrap();
            let locked = directory.path().join("locked");
            fs::create_dir(&locked).unwrap();
            fs::set_permissions(&locked, fs::Permissions::from_mode(0o500)).unwrap();
            let target = locked.join("known_hosts");

            let outcome = replace_contents(&target, b"content");

            fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
            assert!(matches!(outcome, Err(Error::Io { path, .. }) if path == locked));
        }
    }
}
