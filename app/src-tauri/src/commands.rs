//! The commands the front end invokes; each one forwards to the daemon.

use ipc::protocol::{
    AvailableOpenerDto, DeleteTagResult, ForgetSecretsResult, HostKeyDecisionDto, ImportPreviewDto,
    LogEntryDto, OpenTunnelResult, OpenerSpecDto, RememberDto, ResolvedHostDto, SettingsDto,
    SnapshotDto, SoundEventDto, StandbySoundDto, TargetHealthDto, TemporaryTunnelSpecDto,
    TunnelDefinitionDto, TunnelDto, TunnelSpecDto,
};
use std::path::Path;

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::bridge::BridgeState;
use crate::bridge_error::BridgeError;
use crate::credential::CredentialAnswerInput;
use crate::reveal_target::{RevealTarget, reveal_target};

const EXIT_SUCCESS: i32 = 0;

type CommandResult<T> = Result<T, BridgeError>;

#[tauri::command]
pub async fn connect_daemon(app: AppHandle, state: State<'_, BridgeState>) -> CommandResult<()> {
    let connection = state.connect(&app).await;
    state.settle_startup(&app, &connection).await;
    connection
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
pub async fn resolve_host(
    state: State<'_, BridgeState>,
    host: String,
) -> CommandResult<ResolvedHostDto> {
    Ok(state.client()?.resolve_host(&host).await?)
}

#[tauri::command]
pub async fn import_ssh_config(state: State<'_, BridgeState>) -> CommandResult<ImportPreviewDto> {
    Ok(state.client()?.import_ssh_config().await?)
}

#[tauri::command]
pub async fn import_script(
    state: State<'_, BridgeState>,
    text: String,
) -> CommandResult<ImportPreviewDto> {
    Ok(state.client()?.import_script(&text).await?)
}

#[tauri::command]
pub async fn tunnel_logs(
    state: State<'_, BridgeState>,
    id: String,
) -> CommandResult<Vec<LogEntryDto>> {
    Ok(state.client()?.tunnel_logs(&id).await?)
}

/// Asks the daemon to open the tunnel with its default action, starting it
/// first when it is stopped; answers with what was opened.
#[tauri::command]
pub async fn open_tunnel(
    state: State<'_, BridgeState>,
    id: String,
    with: Option<String>,
) -> CommandResult<OpenTunnelResult> {
    let target = state.client()?.open_tunnel(&id, with.as_deref()).await?;
    Ok(OpenTunnelResult { target })
}

/// Replaces the openers of the user; answers with the openers that can be
/// used afterwards.
#[tauri::command]
pub async fn update_openers(
    state: State<'_, BridgeState>,
    openers: Vec<OpenerSpecDto>,
) -> CommandResult<Vec<AvailableOpenerDto>> {
    Ok(state.client()?.update_openers(openers).await?)
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
pub async fn start_temporary_tunnel(
    state: State<'_, BridgeState>,
    spec: TemporaryTunnelSpecDto,
) -> CommandResult<TunnelDto> {
    Ok(state.client()?.start_temporary_tunnel(spec).await?)
}

#[tauri::command]
pub async fn persist_tunnel(state: State<'_, BridgeState>, id: String) -> CommandResult<()> {
    Ok(state.client()?.persist_tunnel(&id).await?)
}

#[tauri::command]
pub async fn check_target(
    state: State<'_, BridgeState>,
    id: String,
) -> CommandResult<TargetHealthDto> {
    Ok(state.client()?.check_target(&id).await?)
}

#[tauri::command]
pub async fn preview_sound(
    state: State<'_, BridgeState>,
    sound: StandbySoundDto,
    event: SoundEventDto,
) -> CommandResult<()> {
    Ok(state.client()?.preview_sound(sound, event).await?)
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

/// Renames a tag everywhere; returns how many tunnels carried it.
#[tauri::command]
pub async fn rename_tag(
    state: State<'_, BridgeState>,
    from: String,
    to: String,
) -> CommandResult<u64> {
    Ok(state.client()?.rename_tag(&from, &to).await?)
}

/// Removes a tag from every tunnel; returns how many tunnels carried it.
#[tauri::command]
pub async fn delete_tag(
    state: State<'_, BridgeState>,
    tag: String,
) -> CommandResult<DeleteTagResult> {
    let removed_count = state.client()?.delete_tag(&tag).await?;
    Ok(DeleteTagResult { removed_count })
}

/// Shows the configuration file in the file manager; the path comes from the
/// daemon, which may run with another `--config`.
#[tauri::command]
pub async fn reveal_config(app: AppHandle, state: State<'_, BridgeState>) -> CommandResult<()> {
    let paths = state.client()?.daemon_paths().await?;
    let opener = app.opener();
    match reveal_target(Path::new(&paths.config_file), Path::exists) {
        RevealTarget::File(file) => opener.reveal_item_in_dir(file),
        RevealTarget::ParentFolder(folder) => {
            opener.open_path(folder.to_string_lossy(), None::<&str>)
        }
    }
    .map_err(|error| BridgeError::internal(&error.to_string()))
}

#[tauri::command]
pub async fn start_x11(state: State<'_, BridgeState>, host: String) -> CommandResult<()> {
    Ok(state.client()?.start_x11(&host).await?)
}

#[tauri::command]
pub async fn stop_x11(state: State<'_, BridgeState>, host: String) -> CommandResult<()> {
    Ok(state.client()?.stop_x11(&host).await?)
}

#[tauri::command]
pub async fn launch_x11(
    state: State<'_, BridgeState>,
    host: String,
    name: String,
) -> CommandResult<()> {
    Ok(state.client()?.launch_x11(&host, &name).await?)
}

#[tauri::command]
pub async fn adopt_external(
    state: State<'_, BridgeState>,
    id: String,
) -> CommandResult<Vec<String>> {
    Ok(state.client()?.adopt_external(&id).await?)
}

#[tauri::command]
pub async fn kill_external(state: State<'_, BridgeState>, pid: u32) -> CommandResult<()> {
    Ok(state.client()?.kill_external(pid).await?)
}

#[tauri::command]
pub async fn ignore_external(state: State<'_, BridgeState>, pid: u32) -> CommandResult<()> {
    Ok(state.client()?.ignore_external(pid).await?)
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
    remember: Option<RememberDto>,
) -> CommandResult<()> {
    let answer = answer.map(Into::into);
    Ok(state
        .client()?
        .respond_credential(&prompt_id, answer, remember)
        .await?)
}

#[tauri::command]
pub async fn forget_secrets(state: State<'_, BridgeState>) -> CommandResult<ForgetSecretsResult> {
    Ok(state.client()?.forget_secrets().await?)
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
