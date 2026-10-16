//! The commands the front end invokes; each one forwards to the daemon.

use ipc::protocol::{
    HostKeyDecisionDto, SettingsDto, SnapshotDto, TunnelDefinitionDto, TunnelDto, TunnelSpecDto,
};
use tauri::{AppHandle, State};

use crate::bridge::BridgeState;
use crate::bridge_error::BridgeError;
use crate::credential::CredentialAnswerInput;

const EXIT_SUCCESS: i32 = 0;

type CommandResult<T> = Result<T, BridgeError>;

#[tauri::command]
pub async fn connect_daemon(app: AppHandle, state: State<'_, BridgeState>) -> CommandResult<()> {
    state.connect(&app).await
}

#[tauri::command]
pub async fn snapshot(state: State<'_, BridgeState>) -> CommandResult<SnapshotDto> {
    Ok(state.client()?.snapshot().await?)
}

#[tauri::command]
pub async fn list_hosts(state: State<'_, BridgeState>) -> CommandResult<Vec<String>> {
    Ok(state.client()?.list_hosts().await?)
}

#[tauri::command]
pub async fn create_tunnel(
    state: State<'_, BridgeState>,
    spec: TunnelSpecDto,
) -> CommandResult<TunnelDto> {
    Ok(state.client()?.create_tunnel(spec).await?)
}

#[tauri::command]
pub async fn update_tunnel(
    state: State<'_, BridgeState>,
    definition: TunnelDefinitionDto,
) -> CommandResult<TunnelDto> {
    Ok(state.client()?.update_tunnel(definition).await?)
}

#[tauri::command]
pub async fn delete_tunnel(state: State<'_, BridgeState>, id: String) -> CommandResult<()> {
    Ok(state.client()?.delete_tunnel(&id).await?)
}

#[tauri::command]
pub async fn start_tunnel(state: State<'_, BridgeState>, id: String) -> CommandResult<()> {
    Ok(state.client()?.start_tunnel(&id).await?)
}

#[tauri::command]
pub async fn stop_tunnel(state: State<'_, BridgeState>, id: String) -> CommandResult<()> {
    Ok(state.client()?.stop_tunnel(&id).await?)
}

#[tauri::command]
pub async fn restart_tunnel(state: State<'_, BridgeState>, id: String) -> CommandResult<()> {
    Ok(state.client()?.restart_tunnel(&id).await?)
}

#[tauri::command]
pub async fn start_tag(state: State<'_, BridgeState>, tag: String) -> CommandResult<()> {
    Ok(state.client()?.start_tag(&tag).await?)
}

#[tauri::command]
pub async fn stop_tag(state: State<'_, BridgeState>, tag: String) -> CommandResult<()> {
    Ok(state.client()?.stop_tag(&tag).await?)
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, BridgeState>,
    settings: SettingsDto,
) -> CommandResult<()> {
    Ok(state.client()?.update_settings(settings).await?)
}

#[tauri::command]
pub async fn respond_credential(
    state: State<'_, BridgeState>,
    prompt_id: String,
    answer: Option<CredentialAnswerInput>,
) -> CommandResult<()> {
    let answer = answer.map(Into::into);
    Ok(state
        .client()?
        .respond_credential(&prompt_id, answer)
        .await?)
}

#[tauri::command]
pub async fn respond_host_key(
    state: State<'_, BridgeState>,
    prompt_id: String,
    decision: HostKeyDecisionDto,
) -> CommandResult<()> {
    Ok(state
        .client()?
        .respond_host_key(&prompt_id, decision)
        .await?)
}

#[tauri::command]
pub async fn quit_daemon(
    app: AppHandle,
    state: State<'_, BridgeState>,
    force: bool,
) -> CommandResult<()> {
    state.client()?.quit(force).await?;
    app.exit(EXIT_SUCCESS);
    Ok(())
}
