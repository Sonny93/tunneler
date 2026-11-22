//! The commands that save an export to a file and read an import file. The
//! native dialogs open from this side and the front end never names a path:
//! the webview is granted no dialog or file system permission.

use std::fs::File;
use std::path::PathBuf;

use serde::Serialize;
use tauri::WebviewWindow;
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::bridge_error::BridgeError;
use crate::transfer_file::{
    MAX_IMPORT_FILE_BYTES, export_file_name, read_bounded_text, write_private_file,
};

type CommandResult<T> = Result<T, BridgeError>;

const TOML_FILTER_NAME: &str = "TOML";
const TOML_EXTENSIONS: &[&str] = &["toml"];

/// Whether the export was written; `false` when the user cancelled the dialog.
#[derive(Debug, Serialize)]
pub struct ExportOutcome {
    saved: bool,
}

/// The file the user picked: its base name for display and its text.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedImportFile {
    file_name: String,
    contents: String,
}

fn into_path(file: FilePath) -> CommandResult<PathBuf> {
    file.into_path()
        .map_err(|error| BridgeError::internal(&error.to_string()))
}

fn base_name(path: &std::path::Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

fn write_export(path: &std::path::Path, contents: &str) -> CommandResult<()> {
    write_private_file(path, contents).map_err(|error| BridgeError::internal(&error.to_string()))
}

fn read_import(path: &std::path::Path) -> CommandResult<PickedImportFile> {
    let file = File::open(path).map_err(|error| BridgeError::internal(&error.to_string()))?;
    let contents = read_bounded_text(file, MAX_IMPORT_FILE_BYTES)?;
    Ok(PickedImportFile {
        file_name: base_name(path),
        contents,
    })
}

/// Asks where to save `contents`, then writes it there. The dialog asks the
/// user before replacing an existing file.
#[tauri::command]
pub async fn export_to_file(
    window: WebviewWindow,
    contents: String,
    suggested_file_name: String,
) -> CommandResult<ExportOutcome> {
    let file_name = export_file_name(&suggested_file_name);
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        window
            .dialog()
            .file()
            .set_parent(&window)
            .set_file_name(file_name)
            .add_filter(TOML_FILTER_NAME, TOML_EXTENSIONS)
            .blocking_save_file()
    })
    .await
    .map_err(|error| BridgeError::internal(&error.to_string()))?;
    let Some(file) = chosen else {
        return Ok(ExportOutcome { saved: false });
    };
    let path = into_path(file)?;
    tauri::async_runtime::spawn_blocking(move || write_export(&path, &contents))
        .await
        .map_err(|error| BridgeError::internal(&error.to_string()))??;
    Ok(ExportOutcome { saved: true })
}

/// Asks for a file and reads it, refusing one over the size cap. Answers
/// nothing when the user cancelled the dialog.
#[tauri::command]
pub async fn pick_import_file(window: WebviewWindow) -> CommandResult<Option<PickedImportFile>> {
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        window
            .dialog()
            .file()
            .set_parent(&window)
            .add_filter(TOML_FILTER_NAME, TOML_EXTENSIONS)
            .blocking_pick_file()
    })
    .await
    .map_err(|error| BridgeError::internal(&error.to_string()))?;
    let Some(file) = chosen else {
        return Ok(None);
    };
    let path = into_path(file)?;
    let picked = tauri::async_runtime::spawn_blocking(move || read_import(&path))
        .await
        .map_err(|error| BridgeError::internal(&error.to_string()))??;
    Ok(Some(picked))
}
