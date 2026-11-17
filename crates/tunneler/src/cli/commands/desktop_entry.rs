use super::super::CliError;
use super::super::args::{DesktopEntryAction, DesktopEntryArguments};
use crate::menu_entry::{MenuEntryReport, install_menu_entry, uninstall_menu_entry};

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

fn describe_install(report: &MenuEntryReport) -> Vec<String> {
    let mut lines: Vec<String> = report
        .files
        .iter()
        .map(|path| format!("Wrote {}", path.display()))
        .collect();
    if let Some(path) = &report.repointed_autostart {
        lines.push(format!("Updated {}", path.display()));
    }
    lines
}

fn describe_uninstall(report: &MenuEntryReport) -> Vec<String> {
    if report.files.is_empty() {
        return vec!["No menu entry to remove.".to_owned()];
    }
    report
        .files
        .iter()
        .map(|path| format!("Removed {}", path.display()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;

    use super::*;

    fn report(files: &[&str], repointed_autostart: Option<&str>) -> MenuEntryReport {
        MenuEntryReport {
            files: files.iter().map(PathBuf::from).collect(),
            repointed_autostart: repointed_autostart.map(PathBuf::from),
        }
    }

    #[test]
    fn should_list_every_written_file_when_describing_an_install() {
        assert_eq!(
            describe_install(&report(&["/a.desktop", "/a.png"], None)),
            vec!["Wrote /a.desktop", "Wrote /a.png"]
        );
    }

    #[test]
    fn should_mention_the_autostart_entry_when_it_was_repointed() {
        let lines = describe_install(&report(&["/a.desktop"], Some("/autostart.desktop")));

        assert_eq!(lines.last().unwrap(), "Updated /autostart.desktop");
    }

    #[test]
    fn should_say_there_is_nothing_to_remove_when_no_file_was_removed() {
        assert_eq!(
            describe_uninstall(&report(&[], None)),
            vec!["No menu entry to remove."]
        );
    }

    #[test]
    fn should_list_every_removed_file_when_describing_an_uninstall() {
        assert_eq!(
            describe_uninstall(&report(&["/a.desktop"], None)),
            vec!["Removed /a.desktop"]
        );
    }
}
