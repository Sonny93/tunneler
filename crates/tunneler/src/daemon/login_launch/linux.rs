//! XDG desktops: an autostart entry, `autostart/tunneler.desktop` in the user
//! configuration directory (`$XDG_CONFIG_HOME`, usually `~/.config`), per the
//! Desktop Entry and Desktop Application Autostart specifications.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::BaseDirs;

use super::{DAEMON_ARGUMENT, LoginLaunch, LoginLaunchError};
use crate::desktop_file::{
    has_ownership_line, quote_exec_argument, read_if_exists, write_atomically,
};

const AUTOSTART_DIRECTORY: &str = "autostart";
const ENTRY_FILE_NAME: &str = "tunneler.desktop";
const OWNERSHIP_LINE: &str = "X-Tunneler-Autostart=true";

pub(super) struct SystemLoginLaunch;

impl LoginLaunch for SystemLoginLaunch {
    fn apply(&self, is_enabled: bool) -> Result<(), LoginLaunchError> {
        let base_directories = BaseDirs::new().ok_or(LoginLaunchError::NoConfigDirectory)?;
        let entry = AutostartEntry::new(&base_directories.config_dir().join(AUTOSTART_DIRECTORY));
        if !is_enabled {
            return entry.disable();
        }
        let executable = std::env::current_exe().map_err(LoginLaunchError::Executable)?;
        entry.enable(&executable)
    }
}

/// Points the launch-at-login entry at `executable` when it exists and
/// Tunneler wrote it, and returns its path then. Never creates it, and leaves
/// a file that is not ours alone.
pub(crate) fn repoint_owned_entry(executable: &Path) -> Result<Option<PathBuf>, LoginLaunchError> {
    let base_directories = BaseDirs::new().ok_or(LoginLaunchError::NoConfigDirectory)?;
    let entry = AutostartEntry::new(&base_directories.config_dir().join(AUTOSTART_DIRECTORY));
    entry.repoint_if_owned(executable)
}

/// The `tunneler.desktop` file of an autostart directory.
struct AutostartEntry {
    path: PathBuf,
}

impl AutostartEntry {
    fn new(autostart_directory: &Path) -> Self {
        Self {
            path: autostart_directory.join(ENTRY_FILE_NAME),
        }
    }

    fn enable(&self, executable: &Path) -> Result<(), LoginLaunchError> {
        let content = desktop_entry(executable);
        match self.read()? {
            Some(existing) if existing == content => Ok(()),
            Some(existing) if !is_written_by_tunneler(&existing) => {
                Err(LoginLaunchError::ForeignEntry {
                    path: self.path.clone(),
                })
            }
            _ => self.write(&content),
        }
    }

    fn repoint_if_owned(&self, executable: &Path) -> Result<Option<PathBuf>, LoginLaunchError> {
        match self.read()? {
            Some(existing) if is_written_by_tunneler(&existing) => {
                self.enable(executable)?;
                Ok(Some(self.path.clone()))
            }
            _ => Ok(None),
        }
    }

    fn disable(&self) -> Result<(), LoginLaunchError> {
        match self.read()? {
            Some(existing) if is_written_by_tunneler(&existing) => self.remove(),
            _ => Ok(()),
        }
    }

    fn read(&self) -> Result<Option<String>, LoginLaunchError> {
        read_if_exists(&self.path).map_err(|source| self.io_error(source))
    }

    fn write(&self, content: &str) -> Result<(), LoginLaunchError> {
        write_atomically(&self.path, content.as_bytes()).map_err(|source| self.io_error(source))
    }

    fn remove(&self) -> Result<(), LoginLaunchError> {
        match fs::remove_file(&self.path) {
            Err(source) if source.kind() != io::ErrorKind::NotFound => Err(self.io_error(source)),
            _ => Ok(()),
        }
    }

    fn io_error(&self, source: io::Error) -> LoginLaunchError {
        LoginLaunchError::Io {
            path: self.path.clone(),
            source,
        }
    }
}

fn is_written_by_tunneler(content: &str) -> bool {
    has_ownership_line(content, OWNERSHIP_LINE)
}

fn desktop_entry(executable: &Path) -> String {
    let exec = format!(
        "{} {DAEMON_ARGUMENT}",
        quote_exec_argument(&executable.to_string_lossy())
    );
    format!(
        "[Desktop Entry]\nType=Application\nName=Tunneler\nExec={exec}\nX-GNOME-Autostart-enabled=true\nNoDisplay=false\nTerminal=false\n{OWNERSHIP_LINE}\n"
    )
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::*;

    const EXECUTABLE: &str = "/usr/bin/tunneler";

    fn entry_in(directory: &TempDir) -> AutostartEntry {
        AutostartEntry::new(&directory.path().join("autostart"))
    }

    fn content_of(entry: &AutostartEntry) -> String {
        fs::read_to_string(&entry.path).unwrap()
    }

    #[test]
    fn should_write_the_desktop_entry_and_create_the_autostart_directory_when_enabling() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);

        entry.enable(Path::new(EXECUTABLE)).unwrap();

        assert_eq!(
            content_of(&entry),
            "[Desktop Entry]\nType=Application\nName=Tunneler\nExec=/usr/bin/tunneler daemon\nX-GNOME-Autostart-enabled=true\nNoDisplay=false\nTerminal=false\nX-Tunneler-Autostart=true\n"
        );
        assert!(entry.path.is_file());
    }

    #[test]
    fn should_leave_no_temporary_file_when_enabling() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);

        entry.enable(Path::new(EXECUTABLE)).unwrap();

        let names: Vec<_> = fs::read_dir(directory.path().join("autostart"))
            .unwrap()
            .map(|file| file.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![ENTRY_FILE_NAME]);
    }

    #[test]
    fn should_keep_the_same_content_when_enabling_twice() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);
        entry.enable(Path::new(EXECUTABLE)).unwrap();
        let first = content_of(&entry);

        entry.enable(Path::new(EXECUTABLE)).unwrap();

        assert_eq!(content_of(&entry), first);
    }

    #[test]
    fn should_refresh_the_executable_when_the_path_is_stale() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);
        entry.enable(Path::new("/old/tunneler")).unwrap();

        entry.enable(Path::new(EXECUTABLE)).unwrap();

        assert!(content_of(&entry).contains("Exec=/usr/bin/tunneler daemon\n"));
    }

    #[test]
    fn should_repoint_the_entry_when_it_is_ours() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);
        entry.enable(Path::new("/old/tunneler")).unwrap();

        let repointed = entry.repoint_if_owned(Path::new(EXECUTABLE)).unwrap();

        assert_eq!(repointed, Some(entry.path.clone()));
        assert!(content_of(&entry).contains("Exec=/usr/bin/tunneler daemon\n"));
    }

    #[test]
    fn should_not_create_the_entry_when_repointing_and_there_is_none() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);

        let repointed = entry.repoint_if_owned(Path::new(EXECUTABLE)).unwrap();

        assert_eq!(repointed, None);
        assert!(!entry.path.exists());
    }

    #[test]
    fn should_leave_a_file_that_is_not_ours_alone_when_repointing() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);
        fs::create_dir_all(entry.path.parent().unwrap()).unwrap();
        fs::write(&entry.path, "[Desktop Entry]\nName=Someone else\n").unwrap();

        entry.repoint_if_owned(Path::new(EXECUTABLE)).unwrap();

        assert_eq!(content_of(&entry), "[Desktop Entry]\nName=Someone else\n");
    }

    #[test]
    fn should_remove_the_entry_when_disabling() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);
        entry.enable(Path::new(EXECUTABLE)).unwrap();

        entry.disable().unwrap();

        assert!(!entry.path.exists());
    }

    #[test]
    fn should_succeed_when_disabling_and_there_is_no_entry() {
        let directory = tempfile::tempdir().unwrap();

        assert!(entry_in(&directory).disable().is_ok());
    }

    #[test]
    fn should_not_remove_a_file_that_is_not_ours_when_disabling() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);
        fs::create_dir_all(entry.path.parent().unwrap()).unwrap();
        fs::write(&entry.path, "[Desktop Entry]\nName=Someone else\n").unwrap();

        entry.disable().unwrap();

        assert!(entry.path.exists());
    }

    #[test]
    fn should_not_overwrite_a_file_that_is_not_ours_when_enabling() {
        let directory = tempfile::tempdir().unwrap();
        let entry = entry_in(&directory);
        fs::create_dir_all(entry.path.parent().unwrap()).unwrap();
        fs::write(&entry.path, "[Desktop Entry]\nName=Someone else\n").unwrap();

        let outcome = entry.enable(Path::new(EXECUTABLE));

        assert!(matches!(
            outcome,
            Err(LoginLaunchError::ForeignEntry { .. })
        ));
        assert_eq!(content_of(&entry), "[Desktop Entry]\nName=Someone else\n");
    }
}
