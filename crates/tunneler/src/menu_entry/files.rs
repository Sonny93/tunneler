//! The files of the menu entry under an XDG data directory: the `.desktop`
//! file and the icons it names.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::MenuEntryError;
use super::entry_text::{is_written_by_tunneler, render_menu_entry};
use crate::desktop_file::{read_if_exists, write_atomically};
use crate::layout::APPLICATION_IDENTIFIER;

const APPLICATIONS_DIRECTORY: &str = "applications";
const ICONS_DIRECTORY: &str = "icons/hicolor";
const ICON_CATEGORY_DIRECTORY: &str = "apps";

struct MenuIcon {
    size_directory: &'static str,
    content: &'static [u8],
}

const MENU_ICONS: [MenuIcon; 3] = [
    MenuIcon {
        size_directory: "32x32",
        content: include_bytes!("../../../../app/src-tauri/icons/32x32.png"),
    },
    MenuIcon {
        size_directory: "128x128",
        content: include_bytes!("../../../../app/src-tauri/icons/128x128.png"),
    },
    MenuIcon {
        size_directory: "256x256",
        content: include_bytes!("../../../../app/src-tauri/icons/128x128@2x.png"),
    },
];

/// Where the menu entry lives under one data directory.
///
/// Only the `.desktop` file can carry an ownership line. The icons cannot, so
/// they are ours by their name alone: the reverse-DNS identifier of the
/// application, which nothing else uses.
pub(super) struct MenuEntryFiles {
    entry_path: PathBuf,
    icon_paths: Vec<PathBuf>,
}

impl MenuEntryFiles {
    /// The files under `data_directory` (`$XDG_DATA_HOME`, usually
    /// `~/.local/share`).
    pub(super) fn in_data_directory(data_directory: &Path) -> Self {
        let entry_path = data_directory
            .join(APPLICATIONS_DIRECTORY)
            .join(format!("{APPLICATION_IDENTIFIER}.desktop"));
        let icon_paths = MENU_ICONS
            .iter()
            .map(|icon| icon_path(data_directory, icon))
            .collect();
        Self {
            entry_path,
            icon_paths,
        }
    }

    /// Writes the icons, then the entry pointing at `ui_program`, and returns
    /// what was written.
    ///
    /// # Errors
    ///
    /// Fails, writing nothing, when a `.desktop` file that Tunneler did not
    /// write is already there.
    pub(super) fn install(&self, ui_program: &Path) -> Result<Vec<PathBuf>, MenuEntryError> {
        self.reject_foreign_entry()?;
        for (path, icon) in self.icon_paths.iter().zip(&MENU_ICONS) {
            write_atomically(path, icon.content).map_err(|source| io_error(path, source))?;
        }
        self.write_entry(ui_program)?;
        let mut written = vec![self.entry_path.clone()];
        written.extend(self.icon_paths.iter().cloned());
        Ok(written)
    }

    /// Points the entry at `ui_program` when Tunneler wrote it and it points
    /// elsewhere. Does nothing when there is no entry, or when it is not ours.
    pub(super) fn refresh(&self, ui_program: &Path) -> Result<(), MenuEntryError> {
        match self.read_entry()? {
            Some(existing)
                if is_written_by_tunneler(&existing)
                    && existing != render_menu_entry(ui_program) =>
            {
                self.write_entry(ui_program)
            }
            _ => Ok(()),
        }
    }

    /// Removes the entry and the icons that exist, and returns what was
    /// removed.
    ///
    /// # Errors
    ///
    /// Fails, removing nothing, when a `.desktop` file that Tunneler did not
    /// write is there.
    pub(super) fn uninstall(&self) -> Result<Vec<PathBuf>, MenuEntryError> {
        self.reject_foreign_entry()?;
        let mut removed = Vec::new();
        for path in std::iter::once(&self.entry_path).chain(&self.icon_paths) {
            if remove_if_exists(path)? {
                removed.push(path.clone());
            }
        }
        Ok(removed)
    }

    /// Whether Tunneler's entry is installed.
    pub(super) fn is_installed(&self) -> bool {
        matches!(self.read_entry(), Ok(Some(existing)) if is_written_by_tunneler(&existing))
    }

    fn reject_foreign_entry(&self) -> Result<(), MenuEntryError> {
        match self.read_entry()? {
            Some(existing) if !is_written_by_tunneler(&existing) => {
                Err(MenuEntryError::ForeignEntry {
                    path: self.entry_path.clone(),
                })
            }
            _ => Ok(()),
        }
    }

    fn read_entry(&self) -> Result<Option<String>, MenuEntryError> {
        read_if_exists(&self.entry_path).map_err(|source| io_error(&self.entry_path, source))
    }

    fn write_entry(&self, ui_program: &Path) -> Result<(), MenuEntryError> {
        write_atomically(&self.entry_path, render_menu_entry(ui_program).as_bytes())
            .map_err(|source| io_error(&self.entry_path, source))
    }
}

fn icon_path(data_directory: &Path, icon: &MenuIcon) -> PathBuf {
    data_directory
        .join(ICONS_DIRECTORY)
        .join(icon.size_directory)
        .join(ICON_CATEGORY_DIRECTORY)
        .join(format!("{APPLICATION_IDENTIFIER}.png"))
}

fn remove_if_exists(path: &Path) -> Result<bool, MenuEntryError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(io_error(path, source)),
    }
}

fn io_error(path: &Path, source: io::Error) -> MenuEntryError {
    MenuEntryError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::*;

    const UI_PROGRAM: &str = "/opt/tunneler/tunneler-ui";
    const FOREIGN_ENTRY: &str = "[Desktop Entry]\nName=Someone else\n";

    fn files_in(directory: &TempDir) -> MenuEntryFiles {
        MenuEntryFiles::in_data_directory(directory.path())
    }

    fn write_foreign_entry(files: &MenuEntryFiles) {
        fs::create_dir_all(files.entry_path.parent().unwrap()).unwrap();
        fs::write(&files.entry_path, FOREIGN_ENTRY).unwrap();
    }

    #[test]
    fn should_write_the_entry_and_the_three_icons_when_installing() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);

        let written = files.install(Path::new(UI_PROGRAM)).unwrap();

        assert_eq!(written.len(), 4);
        assert!(written.iter().all(|path| path.is_file()));
        assert_eq!(
            fs::read_to_string(&files.entry_path).unwrap(),
            render_menu_entry(Path::new(UI_PROGRAM))
        );
    }

    #[test]
    fn should_name_the_icons_after_the_application_identifier_in_each_size_directory() {
        let directory = tempfile::tempdir().unwrap();

        let files = files_in(&directory);

        let relative: Vec<_> = files
            .icon_paths
            .iter()
            .map(|path| path.strip_prefix(directory.path()).unwrap().to_owned())
            .collect();
        assert_eq!(
            relative,
            vec![
                PathBuf::from("icons/hicolor/32x32/apps/com.minimalstuff.tunneler.png"),
                PathBuf::from("icons/hicolor/128x128/apps/com.minimalstuff.tunneler.png"),
                PathBuf::from("icons/hicolor/256x256/apps/com.minimalstuff.tunneler.png"),
            ]
        );
    }

    #[test]
    fn should_rewrite_the_exec_when_installing_again_from_another_folder() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);
        files.install(Path::new("/old/tunneler-ui")).unwrap();

        files.install(Path::new(UI_PROGRAM)).unwrap();

        let content = fs::read_to_string(&files.entry_path).unwrap();
        assert!(
            content.contains(&format!("Exec={UI_PROGRAM}\n")),
            "{content}"
        );
    }

    #[test]
    fn should_write_nothing_when_a_foreign_entry_is_in_the_way() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);
        write_foreign_entry(&files);

        let outcome = files.install(Path::new(UI_PROGRAM));

        assert!(matches!(outcome, Err(MenuEntryError::ForeignEntry { .. })));
        assert_eq!(
            fs::read_to_string(&files.entry_path).unwrap(),
            FOREIGN_ENTRY
        );
        assert!(files.icon_paths.iter().all(|path| !path.exists()));
    }

    #[test]
    fn should_remove_the_entry_and_the_icons_when_uninstalling() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);
        files.install(Path::new(UI_PROGRAM)).unwrap();

        let removed = files.uninstall().unwrap();

        assert_eq!(removed.len(), 4);
        assert!(!files.entry_path.exists());
        assert!(files.icon_paths.iter().all(|path| !path.exists()));
    }

    #[test]
    fn should_succeed_and_remove_nothing_when_uninstalling_without_an_entry() {
        let directory = tempfile::tempdir().unwrap();

        assert_eq!(
            files_in(&directory).uninstall().unwrap(),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn should_keep_the_foreign_entry_and_the_icons_when_uninstalling() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);
        files.install(Path::new(UI_PROGRAM)).unwrap();
        fs::write(&files.entry_path, FOREIGN_ENTRY).unwrap();

        let outcome = files.uninstall();

        assert!(matches!(outcome, Err(MenuEntryError::ForeignEntry { .. })));
        assert_eq!(
            fs::read_to_string(&files.entry_path).unwrap(),
            FOREIGN_ENTRY
        );
        assert!(files.icon_paths.iter().all(|path| path.exists()));
    }

    #[test]
    fn should_keep_the_other_files_of_the_directories_when_uninstalling() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);
        files.install(Path::new(UI_PROGRAM)).unwrap();
        let other_entry = directory.path().join("applications/other.desktop");
        let other_icon = directory.path().join("icons/hicolor/32x32/apps/other.png");
        fs::write(&other_entry, "x").unwrap();
        fs::write(&other_icon, "x").unwrap();

        files.uninstall().unwrap();

        assert!(other_entry.exists() && other_icon.exists());
    }

    #[test]
    fn should_point_an_owned_entry_at_the_new_program_when_refreshing() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);
        files.install(Path::new("/old/tunneler-ui")).unwrap();

        files.refresh(Path::new(UI_PROGRAM)).unwrap();

        let content = fs::read_to_string(&files.entry_path).unwrap();
        assert!(
            content.contains(&format!("TryExec={UI_PROGRAM}\n")),
            "{content}"
        );
    }

    #[test]
    fn should_not_create_the_entry_when_refreshing_and_there_is_none() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);

        files.refresh(Path::new(UI_PROGRAM)).unwrap();

        assert!(!files.entry_path.exists());
    }

    #[test]
    fn should_leave_a_foreign_entry_alone_when_refreshing() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);
        write_foreign_entry(&files);

        files.refresh(Path::new(UI_PROGRAM)).unwrap();

        assert_eq!(
            fs::read_to_string(&files.entry_path).unwrap(),
            FOREIGN_ENTRY
        );
    }

    #[test]
    fn should_report_the_entry_as_installed_when_it_is_ours() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);
        files.install(Path::new(UI_PROGRAM)).unwrap();

        assert!(files.is_installed());
    }

    #[test]
    fn should_not_report_the_entry_as_installed_when_there_is_none() {
        let directory = tempfile::tempdir().unwrap();

        assert!(!files_in(&directory).is_installed());
    }

    #[test]
    fn should_not_report_the_entry_as_installed_when_it_is_not_ours() {
        let directory = tempfile::tempdir().unwrap();
        let files = files_in(&directory);
        write_foreign_entry(&files);

        assert!(!files.is_installed());
    }
}
