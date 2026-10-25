use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::{Instant, sleep};
use tracing::{debug, info, warn};

use super::display::LocalDisplay;
use super::error::X11Error;
use super::local::LocalX11Resolver;
use super::local_connection::connect_display_within;
use crate::blocking::run_blocking;

/// How long one reachability probe of the local display may take.
const PROBE_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

/// An X server installed on this machine that could be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11Server {
    /// The name shown to the user, such as `VcXsrv`.
    pub name: String,
    /// The program that runs the server.
    pub executable: PathBuf,
}

/// The X servers of this machine: which one is installed and how to run it.
/// The daemon provides the real one; tests provide a fake.
pub trait LocalX11Servers: Send + Sync {
    /// The installed X server to start, when there is one.
    fn find(&self) -> Option<X11Server>;

    /// Runs `server` detached so that it serves the display `display_number`,
    /// without waiting for it to be ready.
    fn start(&self, server: &X11Server, display_number: u16) -> io::Result<()>;
}

/// What the engine knows about the local X display of a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X11Probe {
    /// The display, as the X11 view shows it.
    pub display: String,
    /// Whether the display accepts connections.
    pub is_reachable: bool,
    /// The name of the installed X server that could be started. Only set
    /// when the display is unreachable and local.
    pub server: Option<String>,
}

/// Probes the local X display of a host and starts an X server for it.
pub(crate) struct X11ServerControl {
    resolver: Arc<LocalX11Resolver>,
    servers: Arc<dyn LocalX11Servers>,
    start_timeout: Duration,
    poll_interval: Duration,
}

impl X11ServerControl {
    pub(crate) fn new(
        resolver: Arc<LocalX11Resolver>,
        servers: Arc<dyn LocalX11Servers>,
        start_timeout: Duration,
        poll_interval: Duration,
    ) -> Self {
        Self {
            resolver,
            servers,
            start_timeout,
            poll_interval,
        }
    }

    pub(crate) async fn probe(&self, host: &str) -> Result<X11Probe, X11Error> {
        let display = self.resolver.resolve_display(host)?;
        let is_reachable = is_reachable(&display).await;
        let server = if !is_reachable && display.is_local() {
            self.find_server().await.map(|server| server.name)
        } else {
            None
        };
        let local_display = display.to_string();
        let server_name = server.as_deref().unwrap_or("none");
        debug!(
            host,
            local_display = local_display.as_str(),
            is_reachable,
            server = server_name,
            "probed the local X display"
        );
        Ok(X11Probe {
            display: display.to_string(),
            is_reachable,
            server,
        })
    }

    /// Starts the installed X server for the display of `host` and waits for
    /// the display to accept connections. Does nothing when it already does.
    pub(crate) async fn start(&self, host: &str) -> Result<(), X11Error> {
        let display = self.resolver.resolve_display(host)?;
        if !display.is_local() {
            return Err(X11Error::ServerDisplayNotLocal {
                display: display.to_string(),
            });
        }
        if is_reachable(&display).await {
            return Ok(());
        }
        let server = self.find_server().await.ok_or(X11Error::NoLocalServer)?;
        let display_number = display.number;
        info!(
            server = server.name.as_str(),
            executable = server.executable.to_string_lossy().as_ref(),
            display_number,
            "starting the X server"
        );
        self.spawn_server(&server, display.number).await?;
        self.wait_until_reachable(&server, &display).await
    }

    async fn find_server(&self) -> Option<X11Server> {
        let servers = Arc::clone(&self.servers);
        run_blocking(move || servers.find()).await
    }

    async fn spawn_server(&self, server: &X11Server, display_number: u16) -> Result<(), X11Error> {
        let servers = Arc::clone(&self.servers);
        let started = server.clone();
        run_blocking(move || servers.start(&started, display_number))
            .await
            .map_err(|source| X11Error::StartServer {
                server: server.name.clone(),
                source,
            })
    }

    async fn wait_until_reachable(
        &self,
        server: &X11Server,
        display: &LocalDisplay,
    ) -> Result<(), X11Error> {
        let deadline = Instant::now() + self.start_timeout;
        loop {
            sleep(self.poll_interval).await;
            if is_reachable(display).await {
                let local_display = display.to_string();
                info!(
                    server = server.name.as_str(),
                    local_display = local_display.as_str(),
                    "the X server answers"
                );
                return Ok(());
            }
            if Instant::now() >= deadline {
                let local_display = display.to_string();
                warn!(
                    server = server.name.as_str(),
                    local_display = local_display.as_str(),
                    "the X server did not answer in time"
                );
                return Err(X11Error::ServerDidNotStart {
                    server: server.name.clone(),
                    display: display.to_string(),
                });
            }
        }
    }
}

async fn is_reachable(display: &LocalDisplay) -> bool {
    connect_display_within(display, PROBE_CONNECT_TIMEOUT)
        .await
        .is_ok()
}
