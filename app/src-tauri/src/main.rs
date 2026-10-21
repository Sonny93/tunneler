//! Tunneler UI: Tauri 2 window hosting the React front end.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod bridge_error;
mod browser_url;
mod commands;
mod connection_slot;
mod credential;
mod daemon_binary;
mod reveal_target;
mod startup_claim;
mod window_attention;

use std::time::Duration;

use bridge::BridgeState;
use tauri::Manager;

const STARTUP_REVEAL_DEADLINE: Duration = Duration::from_secs(15);

fn main() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(BridgeState::default())
        .setup(|app| {
            reveal_window_when_startup_stalls(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect_daemon,
            commands::snapshot,
            commands::list_hosts,
            commands::resolve_host,
            commands::import_ssh_config,
            commands::import_script,
            commands::tunnel_logs,
            commands::open_in_browser,
            commands::create_tunnel,
            commands::update_tunnel,
            commands::start_temporary_tunnel,
            commands::persist_tunnel,
            commands::check_target,
            commands::delete_tunnel,
            commands::start_tunnel,
            commands::stop_tunnel,
            commands::restart_tunnel,
            commands::start_tag,
            commands::stop_tag,
            commands::rename_tag,
            commands::delete_tag,
            commands::reveal_config,
            commands::start_x11,
            commands::stop_x11,
            commands::launch_x11,
            commands::adopt_external,
            commands::kill_external,
            commands::ignore_external,
            commands::update_settings,
            commands::respond_credential,
            commands::forget_secrets,
            commands::respond_host_key,
            commands::quit_daemon,
        ])
        .run(tauri::generate_context!())
}

fn reveal_window_when_startup_stalls(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_REVEAL_DEADLINE).await;
        app.state::<BridgeState>().reveal_if_unsettled(&app);
    });
}
