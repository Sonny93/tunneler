use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ulid::Ulid;

use super::error::{StoreError, io_error};

#[cfg(unix)]
const NEW_FILE_MODE: u32 = 0o600;
#[cfg(unix)]
const NEW_DIRECTORY_MODE: u32 = 0o700;

/// Reads a file; a missing file reads as `None`.
pub(crate) fn read_if_present(path: &Path) -> Result<Option<Vec<u8>>, StoreError> {
    match fs::read(path) {
        Ok(content) => Ok(Some(content)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(io_error(path, source)),
    }
}

/// Replaces the content of `path` atomically: write a temporary file next to
/// it, fsync, rename. Symlinks are written through, an existing file keeps its
/// permissions, a new file is private to the user (`0600` on Unix) and missing
/// parent directories are created private (`0700` on Unix).
pub(crate) fn replace_contents(path: &Path, content: &[u8]) -> Result<(), StoreError> {
    let target = resolve_symlink(path);
    let directory = parent_directory(&target);
    create_private_directories(directory)?;
    let temp =
        TempFile::create(directory, &target).map_err(|source| io_error(directory, source))?;
    temp.write_replacement_of(&target, content)
        .map_err(|source| io_error(&target, source))?;
    temp.persist_as(&target)
        .map_err(|source| io_error(&target, source))
}

/// Creates `directory` and its missing ancestors, private to the user on Unix.
pub(crate) fn create_private_directories(directory: &Path) -> Result<(), StoreError> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, NEW_DIRECTORY_MODE);
    builder
        .create(directory)
        .map_err(|source| io_error(directory, source))
}

pub(crate) fn parent_directory(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn resolve_symlink(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// A temporary file removed on drop unless it was renamed into place.
struct TempFile {
    path: PathBuf,
    file: File,
    is_persisted: bool,
}

impl TempFile {
    fn create(directory: &Path, target: &Path) -> io::Result<Self> {
        let target_name = target.file_name().unwrap_or(OsStr::new("store"));
        let path = directory.join(format!(
            ".{}.{}.tmp",
            target_name.to_string_lossy(),
            Ulid::generate()
        ));
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
        let target = directory.path().join("tunnels.toml");

        replace_contents(&target, b"content").unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"content");
    }

    #[test]
    fn should_replace_content_when_target_exists() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("tunnels.toml");
        fs::write(&target, "old").unwrap();

        replace_contents(&target, b"new").unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"new");
    }

    #[test]
    fn should_leave_no_temporary_file_when_replacement_succeeds() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("tunnels.toml");

        replace_contents(&target, b"content").unwrap();

        assert_eq!(directory_entries(directory.path()), vec!["tunnels.toml"]);
    }

    #[test]
    fn should_create_missing_parent_directories_when_target_is_nested() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("a").join("b").join("tunnels.toml");

        replace_contents(&target, b"content").unwrap();

        assert!(target.is_file());
    }

    #[test]
    fn should_read_as_none_when_file_is_missing() {
        let directory = tempfile::tempdir().unwrap();

        assert_eq!(
            read_if_present(&directory.path().join("absent")).unwrap(),
            None
        );
    }

    #[cfg(unix)]
    mod unix {
        use super::*;
        use pretty_assertions::assert_eq;
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        const MODE_MASK: u32 = 0o777;

        fn mode_of(path: &Path) -> u32 {
            fs::metadata(path).unwrap().mode() & MODE_MASK
        }

        #[test]
        fn should_create_private_file_when_target_is_missing() {
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join("tunnels.toml");

            replace_contents(&target, b"content").unwrap();

            assert_eq!(mode_of(&target), 0o600);
        }

        #[test]
        fn should_create_private_parent_directory_when_it_is_missing() {
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join("tunneler").join("tunnels.toml");

            replace_contents(&target, b"content").unwrap();

            assert_eq!(mode_of(&directory.path().join("tunneler")), 0o700);
        }

        #[test]
        fn should_preserve_permissions_when_target_exists() {
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join("tunnels.toml");
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
            std::os::unix::fs::symlink(&real, &link).unwrap();

            replace_contents(&link, b"new").unwrap();

            assert_eq!(fs::read(&real).unwrap(), b"new");
        }
    }
}
