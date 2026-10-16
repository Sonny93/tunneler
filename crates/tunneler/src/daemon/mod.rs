//! The daemon: the composition root that wires the engine, the IPC server, the
//! prompt broker and the signal handling together.
//!
//! [`run`] makes the process the single daemon of its endpoint, then serves
//! until a termination signal or an accepted `daemon.quit` stops it.

mod error;
mod events;
mod handler;
mod health;
mod instance;
mod mapping;
mod prompts;
mod signals;
#[cfg(test)]
#[cfg(unix)]
mod testing;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use engine::{
    ConnectorDependencies, Engine, EngineDependencies, HostConfigResolver, SshConfigResolver,
    Store, SystemAgentLocator, SystemClock,
};
use ipc::Endpoint;
use ssh_config::{ConfigSources, SystemEnvironment};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub use error::DaemonError;
use events::spawn_event_bridge;
use handler::DaemonHandler;
use health::spawn_health_probing_follower;
use instance::bind_single_instance;
use prompts::PromptBroker;

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
    /// A user `ssh_config` that replaces the platform's default files, like
    /// `ssh -F`.
    pub ssh_config: Option<PathBuf>,
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
    let prompts = Arc::new(PromptBroker::new(server.handle(), wire_events.clone()));
    let hosts = host_config_resolver(options.ssh_config.as_deref())?;
    let engine = Engine::start(EngineDependencies {
        connector: connector_dependencies(hosts.clone(), &prompts),
        clock: Arc::new(SystemClock),
        store,
    })?;
    let bridge = spawn_event_bridge(engine.subscribe(), wire_events);
    let shutdown = CancellationToken::new();
    let quit_requested = CancellationToken::new();
    let handler = Arc::new(DaemonHandler::new(
        engine.clone(),
        prompts,
        hosts,
        quit_requested.clone(),
    ));
    let health = spawn_health_probing_follower(engine.clone(), server.handle(), shutdown.clone());
    let serving = server.serve(handler, wire_receiver, shutdown.clone());
    tracing::info!(
        store = %options.store_path.display(),
        version = env!("CARGO_PKG_VERSION"),
        "the daemon is serving"
    );
    wait_for_stop_request(&quit_requested).await;
    tracing::info!("the daemon is shutting down");
    engine.shutdown().await;
    shutdown.cancel();
    bridge.abort();
    for task in [serving, health] {
        if let Err(error) = task.await {
            tracing::warn!(%error, "a daemon task ended abnormally");
        }
    }
    Ok(())
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
