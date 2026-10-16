//! Tunneler UI: Tauri 2 window hosting the React front end.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> tauri::Result<()> {
    tauri::Builder::default().run(tauri::generate_context!())
}
