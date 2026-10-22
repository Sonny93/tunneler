//! Creates the main window. It is declared in `tauri.conf.json` with
//! `"create": false` and built here with the same options, so that its webview
//! can keep its data next to the executable in portable mode.
//!
//! Without a data directory of its own, Tauri puts the webview data in
//! `<local data dir>/<identifier>` on Windows (`WebView2`: `EBWebView`) and on
//! Linux (`WebKitGTK`), which is what `tunneler purge` removes.

use std::path::PathBuf;

use ipc::{PortableLayout, detect_portable_layout};
use tauri::{App, WebviewWindowBuilder};

const MAIN_WINDOW_LABEL: &str = "main";

/// Where the webview keeps its data: the portable `data/webview` folder in
/// portable mode, `None` (Tauri's default) otherwise.
pub fn webview_data_directory(portable: Option<&PortableLayout>) -> Option<PathBuf> {
    portable.map(PortableLayout::webview_data_directory)
}

/// Builds the hidden main window declared in `tauri.conf.json`.
pub fn create_main_window(app: &App) -> tauri::Result<()> {
    let Some(window_config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW_LABEL)
    else {
        return Err(tauri::Error::WindowNotFound);
    };
    let mut builder = WebviewWindowBuilder::from_config(app.handle(), window_config)?;
    if let Some(directory) = webview_data_directory(detect_portable_layout().as_ref()) {
        builder = builder.data_directory(directory);
    }
    builder.build()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ipc::portable_layout;

    use super::*;

    #[test]
    fn should_keep_the_webview_data_in_the_portable_data_directory_when_portable() {
        let portable = portable_layout(Path::new("/opt/tunneler"), true);

        assert_eq!(
            webview_data_directory(portable.as_ref()),
            Some(PathBuf::from("/opt/tunneler/data/webview"))
        );
    }

    #[test]
    fn should_leave_the_default_webview_data_directory_when_not_portable() {
        assert_eq!(webview_data_directory(None), None);
    }
}
