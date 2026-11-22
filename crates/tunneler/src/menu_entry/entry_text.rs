//! The text of the application menu entry.

use std::path::Path;

use crate::desktop_file::{escape_string_value, has_ownership_line, quote_exec_argument};
use crate::layout::APPLICATION_IDENTIFIER;

/// The line that marks the file as written by Tunneler.
pub(super) const OWNERSHIP_LINE: &str = "X-Tunneler-MenuEntry=true";
/// The window class of the UI: Tauri takes it from the program name.
pub(super) const WINDOW_CLASS: &str = "tunneler-ui";

/// The scheme handler Tunneler registers, so that `tunneler://` links open it.
const LINK_MIME_TYPE: &str = "x-scheme-handler/tunneler;";
/// The Desktop Entry field code for one URL. It is appended after the quoting
/// of the program path, which would double the `%`.
const URL_FIELD_CODE: &str = "%u";

/// The `.desktop` file that starts `ui_program` from the application menu.
pub(super) fn render_menu_entry(ui_program: &Path) -> String {
    let program = ui_program.to_string_lossy();
    let exec = quote_exec_argument(&program);
    let try_exec = escape_string_value(&program);
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Tunneler\n\
         Comment=SSH tunnel manager\n\
         Comment[fr]=Gestionnaire de tunnels SSH\n\
         Exec={exec} {URL_FIELD_CODE}\n\
         TryExec={try_exec}\n\
         Icon={APPLICATION_IDENTIFIER}\n\
         Terminal=false\n\
         Categories=Network;RemoteAccess;\n\
         MimeType={LINK_MIME_TYPE}\n\
         StartupWMClass={WINDOW_CLASS}\n\
         {OWNERSHIP_LINE}\n"
    )
}

/// Whether `content` is a menu entry Tunneler wrote.
pub(super) fn is_written_by_tunneler(content: &str) -> bool {
    has_ownership_line(content, OWNERSHIP_LINE)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn should_render_the_exact_entry_when_the_path_is_plain() {
        assert_eq!(
            render_menu_entry(Path::new("/opt/tunneler/tunneler-ui")),
            "[Desktop Entry]\nType=Application\nName=Tunneler\nComment=SSH tunnel manager\nComment[fr]=Gestionnaire de tunnels SSH\nExec=/opt/tunneler/tunneler-ui %u\nTryExec=/opt/tunneler/tunneler-ui\nIcon=com.minimalstuff.tunneler\nTerminal=false\nCategories=Network;RemoteAccess;\nMimeType=x-scheme-handler/tunneler;\nStartupWMClass=tunneler-ui\nX-Tunneler-MenuEntry=true\n"
        );
    }

    #[test]
    fn should_quote_the_exec_and_leave_the_try_exec_plain_when_the_path_has_a_space() {
        let entry = render_menu_entry(Path::new("/opt/my apps/tunneler-ui"));

        assert!(
            entry.contains("\nExec=\"/opt/my apps/tunneler-ui\" %u\n"),
            "{entry}"
        );
        assert!(
            entry.contains("\nTryExec=/opt/my apps/tunneler-ui\n"),
            "{entry}"
        );
    }

    #[test]
    fn should_register_the_tunneler_scheme_when_rendering() {
        let entry = render_menu_entry(Path::new("/opt/tunneler-ui"));

        assert!(
            entry.contains("\nMimeType=x-scheme-handler/tunneler;\n"),
            "{entry}"
        );
    }

    #[test]
    fn should_end_the_exec_with_the_url_field_code_when_the_path_is_plain() {
        let entry = render_menu_entry(Path::new("/opt/tunneler-ui"));

        assert!(entry.contains("\nExec=/opt/tunneler-ui %u\n"), "{entry}");
    }

    #[test]
    fn should_keep_the_url_field_code_single_when_the_path_has_a_percent_sign() {
        let entry = render_menu_entry(Path::new("/opt/100%/tunneler-ui"));

        let exec_line = entry
            .lines()
            .find(|line| line.starts_with("Exec="))
            .unwrap();
        assert!(exec_line.ends_with(" %u"), "{exec_line}");
        assert!(!exec_line.ends_with("%%u"), "{exec_line}");
    }

    #[test]
    fn should_recognise_its_own_entry() {
        let entry = render_menu_entry(Path::new("/opt/tunneler-ui"));

        assert!(is_written_by_tunneler(&entry));
    }

    #[test]
    fn should_not_recognise_an_entry_without_the_ownership_line() {
        assert!(!is_written_by_tunneler(
            "[Desktop Entry]\nName=Tunneler\nExec=tunneler-ui\n"
        ));
    }
}
