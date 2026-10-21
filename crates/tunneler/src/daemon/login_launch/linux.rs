//! XDG desktops: an autostart entry, `autostart/tunneler.desktop` in the user
//! configuration directory (`$XDG_CONFIG_HOME`, usually `~/.config`), per the
//! Desktop Entry and Desktop Application Autostart specifications.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use directories::BaseDirs;

use super::{DAEMON_ARGUMENT, LoginLaunch, LoginLaunchError};

const AUTOSTART_DIRECTORY: &str = "autostart";
const ENTRY_FILE_NAME: &str = "tunneler.desktop";
const OWNERSHIP_LINE: &str = "X-Tunneler-Autostart=true";
const EXEC_RESERVED_CHARACTERS: &str = " \t\n\"'\\><~|&;$*?#()`";

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

    fn disable(&self) -> Result<(), LoginLaunchError> {
        match self.read()? {
            Some(existing) if is_written_by_tunneler(&existing) => self.remove(),
            _ => Ok(()),
        }
    }

    fn read(&self) -> Result<Option<String>, LoginLaunchError> {
        match fs::read(&self.path) {
            Ok(content) => Ok(Some(String::from_utf8_lossy(&content).into_owned())),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(self.io_error(source)),
        }
    }

    fn write(&self, content: &str) -> Result<(), LoginLaunchError> {
        let directory = self.path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(directory).map_err(|source| self.io_error(source))?;
        let temporary = directory.join(format!(".{ENTRY_FILE_NAME}.{}.tmp", std::process::id()));
        write_file(&temporary, content).map_err(|source| self.io_error(source))?;
        fs::rename(&temporary, &self.path).map_err(|source| {
            let _ = fs::remove_file(&temporary);
            self.io_error(source)
        })
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

fn write_file(path: &Path, content: &str) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()
}

fn is_written_by_tunneler(content: &str) -> bool {
    content.lines().any(|line| line == OWNERSHIP_LINE)
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

/// Quotes one argument of an `Exec` key: arguments with reserved characters
/// go between double quotes, `"`, `` ` ``, `$` and `\` are escaped inside them,
/// every backslash is doubled again by the string syntax of the file, and `%`
/// is doubled so that it is not read as a field code.
fn quote_exec_argument(argument: &str) -> String {
    let escaped: String = argument.chars().map(escape_exec_character).collect();
    if argument.contains(|character| EXEC_RESERVED_CHARACTERS.contains(character)) {
        return format!("\"{escaped}\"");
    }
    escaped
}

fn escape_exec_character(character: char) -> String {
    match character {
        '"' | '`' | '$' => format!("\\\\{character}"),
        '\\' => "\\\\\\\\".to_owned(),
        '%' => "%%".to_owned(),
        other => other.to_string(),
    }
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

    #[test]
    fn should_not_quote_a_plain_path() {
        assert_eq!(
            quote_exec_argument("/usr/bin/tunneler"),
            "/usr/bin/tunneler"
        );
    }

    #[test]
    fn should_quote_a_path_with_spaces() {
        assert_eq!(
            quote_exec_argument("/opt/my apps/tunneler"),
            "\"/opt/my apps/tunneler\""
        );
    }

    #[test]
    fn should_escape_double_quotes_in_a_quoted_path() {
        assert_eq!(quote_exec_argument("/a\"b"), "\"/a\\\\\"b\"");
    }

    #[test]
    fn should_quote_a_path_with_single_quotes() {
        assert_eq!(quote_exec_argument("/it's"), "\"/it's\"");
    }

    #[test]
    fn should_escape_dollars_and_backticks_in_a_quoted_path() {
        assert_eq!(
            quote_exec_argument("/$HOME/`x`"),
            "\"/\\\\$HOME/\\\\`x\\\\`\""
        );
    }

    #[test]
    fn should_quadruple_backslashes_in_a_quoted_path() {
        assert_eq!(quote_exec_argument("/a\\b"), "\"/a\\\\\\\\b\"");
    }

    #[test]
    fn should_double_percent_signs() {
        assert_eq!(quote_exec_argument("/100%/tunneler"), "/100%%/tunneler");
    }
}
