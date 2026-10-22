//! The commands that look for, install and point to a newer version. They are
//! driven from this side only: the webview is granted no updater or opener
//! permission.

use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::bridge::BridgeState;
use crate::bridge_error::BridgeError;
use crate::pending_update::PendingUpdate;
use crate::update_action::{Installation, OperatingSystem, UpdateAction, decide_update_action};
use crate::update_offer::{UpdateOffer, UpdateProgress, release_page_url};

type CommandResult<T> = Result<T, BridgeError>;

fn current_action() -> UpdateAction {
    decide_update_action(OperatingSystem::current(), Installation::detect())
}

fn offer_from(update: &Update) -> UpdateOffer {
    UpdateOffer {
        version: update.version.clone(),
        notes: update.body.clone().filter(|notes| !notes.trim().is_empty()),
        release_url: release_page_url(&update.version),
        action: current_action(),
    }
}

async fn find_update(app: &AppHandle) -> Result<Option<Update>, tauri_plugin_updater::Error> {
    app.updater()?.check().await
}

/// Looks for a newer version once. A failure (offline, repository still
/// private, bad manifest) is only logged: the check runs unasked, so it never
/// shows a message.
#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> CommandResult<Option<UpdateOffer>> {
    let update = find_update(&app).await.unwrap_or_else(|error| {
        tracing::warn!(%error, "cannot check for updates");
        None
    });
    let offer = update.as_ref().map(offer_from);
    pending.replace(update);
    Ok(offer)
}

fn report_progress(channel: &Channel<UpdateProgress>, progress: UpdateProgress) {
    if let Err(error) = channel.send(progress) {
        tracing::warn!(%error, "cannot report the update progress");
    }
}

async fn download_update(
    update: &Update,
    channel: &Channel<UpdateProgress>,
) -> CommandResult<Vec<u8>> {
    let mut downloaded_bytes: u64 = 0;
    update
        .download(
            |chunk_length, total_bytes| {
                downloaded_bytes += chunk_length as u64;
                report_progress(
                    channel,
                    UpdateProgress::Downloading {
                        downloaded_bytes,
                        total_bytes,
                    },
                );
            },
            || {},
        )
        .await
        .map_err(|error| BridgeError::update_failed(&error))
}

/// Downloads and verifies the pending update, stops the daemon (and with it
/// the tunnels), then runs the installer, which relaunches the program. Only
/// the installed Windows program installs; the process exits on success.
#[tauri::command]
pub async fn install_update(
    state: State<'_, BridgeState>,
    pending: State<'_, PendingUpdate>,
    on_progress: Channel<UpdateProgress>,
) -> CommandResult<()> {
    let update = pending
        .current()
        .filter(|_| current_action() == UpdateAction::InstallAndRestart)
        .ok_or_else(BridgeError::update_unavailable)?;
    let package = download_update(&update, &on_progress).await?;
    report_progress(&on_progress, UpdateProgress::Installing);
    state.client()?.quit(true).await?;
    update
        .install(package)
        .map_err(|error| BridgeError::update_failed(&error))
}

/// Opens the release page of the pending update in the browser.
#[tauri::command]
pub async fn open_release_page(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> CommandResult<()> {
    let update = pending
        .current()
        .ok_or_else(BridgeError::update_unavailable)?;
    app.opener()
        .open_url(release_page_url(&update.version), None::<&str>)
        .map_err(|error| BridgeError::internal(&error.to_string()))
}
