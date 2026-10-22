//! The command that opens the documentation site in the browser. Driven from this side only:
//! the webview is granted no opener permission.

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::bridge_error::BridgeError;
use crate::documentation_page::{DocumentationLocale, DocumentationPage, documentation_url};

/// Opens `page` of the documentation, in `locale`, in the browser.
#[tauri::command]
pub async fn open_documentation(
    app: AppHandle,
    page: DocumentationPage,
    locale: DocumentationLocale,
) -> Result<(), BridgeError> {
    app.opener()
        .open_url(documentation_url(page, locale), None::<&str>)
        .map_err(|error| BridgeError::internal(&error.to_string()))
}
