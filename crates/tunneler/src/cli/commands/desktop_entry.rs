use super::super::CliError;
use super::super::args::{DesktopEntryAction, DesktopEntryArguments};
use std::path::PathBuf;

use crate::file_change::{ChangedFile, FileChange};
use crate::menu_entry::{InstallReport, install_menu_entry, uninstall_menu_entry};

/// `tunneler desktop-entry`: installs or removes the application menu entry.
pub fn desktop_entry(arguments: &DesktopEntryArguments) -> Result<(), CliError> {
    let lines = match arguments.action {
        DesktopEntryAction::Install => describe_install(&install_menu_entry()?),
        DesktopEntryAction::Uninstall => describe_uninstall(&uninstall_menu_entry()?),
    };
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

fn describe_install(report: &InstallReport) -> Vec<String> {
    let mut lines: Vec<String> = report
        .files
        .iter()
        .map(|file| describe_file(file, "Wrote"))
        .collect();
    if let Some(file) = &report.autostart {
        lines.push(describe_file(file, "Updated"));
    }
    lines
}

fn describe_file(file: &ChangedFile, written_verb: &str) -> String {
    let verb = match file.change {
        FileChange::Written => written_verb,
        FileChange::Unchanged => "Unchanged",
    };
    format!("{verb} {}", file.path.display())
}

fn describe_uninstall(removed: &[PathBuf]) -> Vec<String> {
    if removed.is_empty() {
        return vec!["No menu entry to remove.".to_owned()];
    }
    removed
        .iter()
        .map(|path| format!("Removed {}", path.display()))
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn file(path: &str, change: FileChange) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            change,
        }
    }

    #[test]
    fn should_say_wrote_for_a_written_file_and_unchanged_for_the_others_when_describing_an_install()
    {
        let report = InstallReport {
            files: vec![
                file("/a.desktop", FileChange::Written),
                file("/a.png", FileChange::Unchanged),
            ],
            autostart: None,
        };

        assert_eq!(
            describe_install(&report),
            vec!["Wrote /a.desktop", "Unchanged /a.png"]
        );
    }

    #[test]
    fn should_say_updated_when_the_autostart_entry_was_rewritten() {
        let report = InstallReport {
            files: vec![],
            autostart: Some(file("/autostart.desktop", FileChange::Written)),
        };

        assert_eq!(
            describe_install(&report),
            vec!["Updated /autostart.desktop"]
        );
    }

    #[test]
    fn should_say_unchanged_when_the_autostart_entry_was_already_right() {
        let report = InstallReport {
            files: vec![],
            autostart: Some(file("/autostart.desktop", FileChange::Unchanged)),
        };

        assert_eq!(
            describe_install(&report),
            vec!["Unchanged /autostart.desktop"]
        );
    }

    #[test]
    fn should_say_nothing_about_the_autostart_entry_when_there_is_none() {
        assert_eq!(
            describe_install(&InstallReport::default()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn should_say_there_is_nothing_to_remove_when_no_file_was_removed() {
        assert_eq!(describe_uninstall(&[]), vec!["No menu entry to remove."]);
    }

    #[test]
    fn should_list_every_removed_file_when_describing_an_uninstall() {
        assert_eq!(
            describe_uninstall(&[PathBuf::from("/a.desktop")]),
            vec!["Removed /a.desktop"]
        );
    }
}
