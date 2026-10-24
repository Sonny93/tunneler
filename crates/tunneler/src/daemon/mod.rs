//! The daemon: the composition root that wires the engine, the IPC server, the
//! prompt broker, the UI coordinator, the tray and the signal handling
//! together.
//!
//! [`run`] makes the process the single daemon of its endpoint, then serves
//! until a termination signal or an accepted `daemon.quit` stops it.

mod console;
mod display_language;
mod error;
mod events;
mod handler;
mod instance;
mod launch;
pub(crate) mod login_launch;
mod mapping;
pub(crate) mod notification_identity;
pub(crate) mod os_keyring;
mod prompts;
mod signals;
mod standby_signal;
#[cfg(test)]
#[cfg(unix)]
mod testing;
mod tray;
mod ui;
mod ui_presence;
mod x11_servers;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use engine::{
    ConnectorDependencies, Engine, EngineDependencies, FileStateStore, HostConfigResolver,
    PresetPlatform, SshConfigResolver, Store, SystemAgentLocator, SystemClock, SystemPortScanner,
    SystemProgramLocator, SystemX11Environment, TunnelId, detect_presets,
};
use ipc::Endpoint;
use ssh_config::{ConfigSources, SystemEnvironment};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub(crate) use console::release_console_if_owned;
use display_language::DisplayLanguage;
pub use error::DaemonError;
use events::spawn_event_bridge;
use handler::{DaemonHandler, LocalEffects};
use instance::bind_single_instance;
use launch::SystemLauncher;
use login_launch::{spawn_login_launch_follower, system_login_launch};
use os_keyring::OsKeyring;
use prompts::PromptBroker;
use standby_signal::{
    StandbyPreferences, StandbySignals, SystemNotifier, SystemSoundPlayer,
    spawn_standby_signal_follower,
};
use tray::{TrayService, TrayStatus, TrayStatusCell, spawn_tray_status_publisher};
use ui::{ProcessUiLauncher, UiCoordinator};
use ui_presence::spawn_ui_presence_follower;
use x11_servers::SystemX11Servers;

/// How many wire events may wait for the IPC server before the oldest are lost.
const WIRE_EVENT_CAPACITY: usize = 1024;

/// Lets the answer to `daemon.quit` reach the client before the connections
/// are closed.
const RESPONSE_FLUSH_GRACE: Duration = Duration::from_millis(100);

/// Where the daemon keeps and finds things.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonOptions {
    /// Where the daemon listens.
    pub endpoint: Endpoint,
    /// The tunnels file.
    pub store_path: PathBuf,
    /// The file that remembers the running tunnels and the keyring entries.
    pub state_path: PathBuf,
    /// A user `ssh_config` that replaces the platform's default files, like
    /// `ssh -F`.
    pub ssh_config: Option<PathBuf>,
    /// Whether the daemon shows its notification-area icon.
    pub is_tray_enabled: bool,
    /// Whether the daemon opens the UI once it serves.
    pub should_open_ui_on_start: bool,
}

/// Runs the daemon in the foreground until it is asked to stop.
///
/// # Errors
///
/// Fails when another daemon runs, when the store or the endpoint cannot be
/// used, or when the engine cannot start.
pub async fn run(options: DaemonOptions) -> Result<(), DaemonError> {
    let (store, _) = Store::open(options.store_path.clone())?;
    let server = bind_single_instance(&options.endpoint).await?;
    let (wire_events, wire_receiver) = broadcast::channel(WIRE_EVENT_CAPACITY);
    let ui = Arc::new(UiCoordinator::new(
        server.handle(),
        Arc::new(ProcessUiLauncher::new(
            options.endpoint.socket_path().map(Path::to_path_buf),
        )),
    ));
    let prompts = Arc::new(PromptBroker::new(
        server.handle(),
        wire_events.clone(),
        ui.clone(),
    ));
    let hosts = host_config_resolver(options.ssh_config.as_deref())?;
    let engine = Engine::start(EngineDependencies {
        connector: connector_dependencies(hosts.clone(), &prompts),
        clock: Arc::new(SystemClock),
        store,
        port_scanner: Arc::new(SystemPortScanner),
        x11_environment: Arc::new(SystemX11Environment),
        x11_servers: Arc::new(SystemX11Servers),
        state_store: Arc::new(FileStateStore::new(options.state_path.clone())),
        keyring: Some(Arc::new(OsKeyring)),
        detected_openers: detect_presets(&SystemProgramLocator, PresetPlatform::current()),
    })?;
    let bridge = spawn_event_bridge(engine.subscribe(), wire_events.clone());
    let shutdown = CancellationToken::new();
    let quit_requested = CancellationToken::new();
    let login_launch = spawn_login_launch_follower(
        engine.subscribe(),
        {
            let engine = engine.clone();
            move || engine.snapshot().settings.launch_at_login
        },
        system_login_launch(),
        shutdown.clone(),
    );
    let standby_signals = start_standby_signals(&engine, shutdown.clone());
    let tray_status = Arc::new(TrayStatusCell::new(TrayStatus::Disabled));
    let handler = Arc::new(DaemonHandler::new(
        engine.clone(),
        prompts,
        hosts,
        quit_requested.clone(),
        ui.clone(),
        tray_status.subscribe(),
        LocalEffects {
            program_launcher: Arc::new(SystemLauncher),
            sound_player: Arc::new(SystemSoundPlayer::new()),
        },
    ));
    let ui_presence = spawn_ui_presence_follower(engine.clone(), server.handle(), shutdown.clone());
    let ui_follower = ui.spawn_follower(shutdown.clone());
    let status_publisher =
        spawn_tray_status_publisher(tray_status.subscribe(), wire_events, shutdown.clone());
    let serving = server.serve(handler, wire_receiver, shutdown.clone());
    let tray = start_tray_service(
        &options,
        &engine,
        &tray_status,
        &ui,
        &quit_requested,
        &shutdown,
    )
    .await;
    start_launch_tunnels(&engine);
    tracing::info!(
        store = %options.store_path.display(),
        version = env!("CARGO_PKG_VERSION"),
        "the daemon is serving"
    );
    if options.should_open_ui_on_start {
        open_ui(&ui);
    }
    wait_for_stop_request(&quit_requested).await;
    tracing::info!("the daemon is shutting down");
    engine.shutdown().await;
    shutdown.cancel();
    bridge.abort();
    if let Some(tray) = tray {
        tray.stop().await;
    }
    for task in [
        serving,
        ui_presence,
        ui_follower,
        status_publisher,
        login_launch,
        standby_signals,
    ] {
        if let Err(error) = task.await {
            tracing::warn!(%error, "a daemon task ended abnormally");
        }
    }
    Ok(())
}

async fn start_tray_service(
    options: &DaemonOptions,
    engine: &Engine,
    status: &Arc<TrayStatusCell>,
    ui: &Arc<UiCoordinator>,
    quit_requested: &CancellationToken,
    shutdown: &CancellationToken,
) -> Option<TrayService> {
    if !options.is_tray_enabled {
        tracing::info!("the tray icon is disabled");
        return None;
    }
    Some(
        TrayService::start(
            engine,
            Arc::clone(status),
            Arc::clone(ui),
            quit_requested.clone(),
            shutdown.clone(),
        )
        .await,
    )
}

fn start_standby_signals(engine: &Engine, shutdown: CancellationToken) -> JoinHandle<()> {
    spawn_standby_signal_follower(
        engine.subscribe(),
        {
            let engine = engine.clone();
            move || {
                let settings = engine.snapshot().settings;
                StandbyPreferences {
                    notify: settings.standby_notify,
                    sound: settings.standby_sound,
                    volume: settings.standby_volume,
                    language: DisplayLanguage::current(settings.language),
                }
            }
        },
        {
            let engine = engine.clone();
            move |id| tunnel_label(&engine, id)
        },
        StandbySignals::new(
            Arc::new(SystemNotifier::new()),
            Arc::new(SystemSoundPlayer::new()),
        ),
        shutdown,
    )
}

fn tunnel_label(engine: &Engine, id: TunnelId) -> Option<String> {
    engine
        .snapshot()
        .tunnels
        .into_iter()
        .find(|tunnel| tunnel.definition.id == id)
        .map(|tunnel| tunnel.definition.label())
}

fn start_launch_tunnels(engine: &Engine) {
    match engine.start_launch_tunnels() {
        Ok(started) if started.is_empty() => tracing::info!("no tunnel to start at launch"),
        Ok(started) => {
            let ids: Vec<String> = started.iter().map(ToString::to_string).collect();
            tracing::info!(tunnels = ?ids, "starting the tunnels selected for the launch");
        }
        Err(error) => tracing::warn!(%error, "cannot start the tunnels selected for the launch"),
    }
}

fn open_ui(ui: &UiCoordinator) {
    if let Err(error) = ui.show() {
        tracing::warn!(%error, "the UI did not open at start");
    }
}

fn host_config_resolver(
    ssh_config: Option<&Path>,
) -> Result<Arc<dyn HostConfigResolver>, DaemonError> {
    let environment = SystemEnvironment::new()?;
    let sources = config_sources(ssh_config, &environment);
    Ok(Arc::new(SshConfigResolver::new(sources, environment)))
}

fn connector_dependencies(
    resolver: Arc<dyn HostConfigResolver>,
    prompts: &Arc<PromptBroker>,
) -> ConnectorDependencies {
    ConnectorDependencies {
        resolver,
        prompter: prompts.clone(),
        verifier: prompts.clone(),
        agent_locator: Arc::new(SystemAgentLocator),
    }
}

fn config_sources(ssh_config: Option<&Path>, environment: &SystemEnvironment) -> ConfigSources {
    match ssh_config {
        Some(path) => ConfigSources::user_only(path),
        None => ConfigSources::platform_default(environment),
    }
}

async fn wait_for_stop_request(quit_requested: &CancellationToken) {
    tokio::select! {
        () = wait_for_termination_signal() => {}
        () = quit_requested.cancelled() => tokio::time::sleep(RESPONSE_FLUSH_GRACE).await,
    }
}

async fn wait_for_termination_signal() {
    if let Err(error) = signals::wait_for_termination().await {
        tracing::error!(%error, "cannot listen for termination signals, only `daemon.quit` can stop the daemon");
        std::future::pending::<()>().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_only_the_given_file_when_an_ssh_config_is_given() {
        let environment = SystemEnvironment::new().unwrap();

        let sources = config_sources(Some(Path::new("/tmp/ssh_config")), &environment);

        assert_eq!(sources, ConfigSources::user_only("/tmp/ssh_config"));
    }

    #[test]
    fn should_read_the_platform_files_when_no_ssh_config_is_given() {
        let environment = SystemEnvironment::new().unwrap();

        let sources = config_sources(None, &environment);

        assert_eq!(sources, ConfigSources::platform_default(&environment));
    }
}
