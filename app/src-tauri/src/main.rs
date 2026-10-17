//! Tunneler UI: Tauri 2 window hosting the React front end.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod bridge_error;
mod commands;
mod connection_slot;
mod credential;
mod daemon_binary;
mod window_attention;

use bridge::BridgeState;

fn main() -> tauri::Result<()> {
    tauri::Builder::default()
        .manage(BridgeState::default())
        .invoke_handler(tauri::generate_handler![
            commands::connect_daemon,
            commands::snapshot,
            commands::list_hosts,
            commands::create_tunnel,
            commands::update_tunnel,
            commands::delete_tunnel,
            commands::start_tunnel,
            commands::stop_tunnel,
            commands::restart_tunnel,
            commands::start_tag,
            commands::stop_tag,
            commands::start_x11,
            commands::stop_x11,
            commands::launch_x11,
            commands::adopt_external,
            commands::kill_external,
            commands::ignore_external,
            commands::update_settings,
            commands::respond_credential,
            commands::respond_host_key,
            commands::quit_daemon,
        ])
        .run(tauri::generate_context!())
}
