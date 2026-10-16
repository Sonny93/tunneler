use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use thiserror::Error;
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

use crate::blocking::run_blocking;
use crate::clock::Clock;
use crate::connector::{Connector, ConnectorDependencies};
use crate::external::{PortScanner, ScanError, TerminateError, UnmanagedProcess};
use crate::runtime::catalog::{Catalog, TunnelFactory, TunnelSnapshot};
use crate::runtime::events::{EngineEvent, EventBus};
use crate::runtime::log_buffer::TunnelLogEntry;
use crate::runtime::options::EngineOptions;
use crate::runtime::port_watch::{PortWatch, run_scan_loop};
use crate::runtime::reload::run_reload_loop;
use crate::runtime::session_manager::SessionManager;
use crate::runtime::tunnel_actor::TunnelCommand;
use crate::store::{Settings, Store, StoreError, TunnelDefinition, TunnelId, TunnelKind};

/// Why an engine operation failed.
#[derive(Debug, Error)]
pub enum EngineError {
    /// No tunnel has this id.
    #[error("no tunnel has the id {0}")]
    UnknownTunnel(TunnelId),
    /// No tunnel carries this tag.
    #[error("no tunnel carries the tag \"{0}\"")]
    UnknownTag(String),
    /// The store refused or failed to write the change.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The engine was shut down.
    #[error("the engine was shut down")]
    ShutDown,
    /// The tunnel is not provided by an external process.
    #[error("the tunnel {0} is not provided by an external process")]
    NotExternal(TunnelId),
    /// No process with this PID is known to hold a tunnel port.
    #[error("no reported process has the pid {0}")]
    UnknownProcess(u32),
    /// The process could not be asked to terminate.
    #[error(transparent)]
    Terminate(#[from] TerminateError),
    /// The process kept its ports after it was asked to terminate.
    #[error("process {0} still holds its ports after being asked to terminate")]
    ProcessStillRunning(u32),
    /// The listening ports could not be scanned.
    #[error(transparent)]
    PortScan(#[from] ScanError),
}

/// What the engine is built from.
pub struct EngineDependencies {
    /// The collaborators that open SSH sessions.
    pub connector: ConnectorDependencies,
    /// Where timestamps come from.
    pub clock: Arc<dyn Clock>,
    /// The persistent configuration. The engine writes tunnel changes to it
    /// and follows external edits of its file.
    pub store: Store,
    /// Where the listening ports and their processes are read from.
    pub port_scanner: Arc<dyn PortScanner>,
}

/// A tunnel to create; the engine generates its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTunnel {
    /// Unique, human-readable name.
    pub name: String,
    /// The `ssh_config` alias the tunnel goes through.
    pub host: String,
    /// The direction of the forward.
    pub kind: TunnelKind,
    /// The port to listen on.
    pub listen_port: u16,
    /// Overrides the default bind address.
    pub listen_address: Option<String>,
    /// The host the traffic is forwarded to; defaults to `127.0.0.1`.
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to; defaults to the listen port.
    pub target_port: Option<u16>,
    /// Free-form labels.
    pub tags: Vec<String>,
}

impl NewTunnel {
    fn into_definition(self, id: TunnelId) -> TunnelDefinition {
        TunnelDefinition {
            id,
            name: self.name,
            host: self.host,
            kind: self.kind,
            listen_port: self.listen_port,
            listen_address: self.listen_address,
            target_host: self.target_host,
            target_port: self.target_port,
            tags: self.tags,
        }
    }
}

/// Everything a client needs to draw the current picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineSnapshot {
    /// The tunnels in configuration order.
    pub tunnels: Vec<TunnelSnapshot>,
    /// The application settings.
    pub settings: Settings,
    /// The `ssh` processes forwarding ports that match no configured tunnel.
    pub externals: Vec<UnmanagedProcess>,
}

pub(crate) struct EngineInner {
    pub(crate) store: Arc<Store>,
    pub(crate) bus: EventBus,
    pub(crate) sessions: SessionManager,
    pub(crate) settings: watch::Sender<Settings>,
    probing: watch::Sender<bool>,
    scanning: watch::Sender<bool>,
    pub(crate) port_watch: Arc<PortWatch>,
    catalog: Mutex<Catalog>,
    pub(crate) mutation: tokio::sync::Mutex<()>,
    reload_task: Mutex<Option<JoinHandle<()>>>,
    scan_task: Mutex<Option<JoinHandle<()>>>,
    pub(crate) is_shut_down: AtomicBool,
}

impl EngineInner {
    pub(crate) fn catalog(&self) -> MutexGuard<'_, Catalog> {
        self.catalog.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) async fn persist_tunnels(
        &self,
        tunnels: Vec<TunnelDefinition>,
    ) -> Result<(), StoreError> {
        let mut document = self.store.document();
        document.tunnels = tunnels;
        let store = Arc::clone(&self.store);
        run_blocking(move || store.save(document)).await
    }
}

/// The tunnel runtime: it owns the tunnels, their SSH sessions and the
/// configuration file, and reports what happens through events.
///
/// Cloning is cheap; every clone drives the same engine.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

impl Engine {
    /// Starts the engine with the tunnels of the store. Every tunnel begins
    /// stopped. Must be called inside a Tokio runtime.
    ///
    /// When the store was read from a hand-written file that lacks tunnel
    /// ids, the generated ids are saved to the file first, so that they stay
    /// the same at the next reload.
    pub fn start(dependencies: EngineDependencies) -> Result<Self, EngineError> {
        Self::start_with_options(dependencies, EngineOptions::default())
    }

    /// Like [`start`](Self::start), with the timing of the runtime tuned.
    pub fn start_with_options(
        dependencies: EngineDependencies,
        options: EngineOptions,
    ) -> Result<Self, EngineError> {
        let EngineDependencies {
            connector,
            clock,
            store,
            port_scanner,
        } = dependencies;
        let store = Arc::new(store);
        if store.has_pending_normalization() {
            store.save(store.document())?;
        }
        let document = store.document();
        let watcher = store.watch()?;
        let (settings_sender, settings) = watch::channel(document.settings);
        let (probing_sender, probing) = watch::channel(false);
        let (scanning_sender, scanning) = watch::channel(false);
        let port_watch = Arc::new(PortWatch::new(port_scanner));
        let options = options.sanitized();
        let bus = EventBus::new();
        let sessions = SessionManager::new(
            Connector::with_settings(connector, settings.clone()),
            settings.clone(),
        );
        let factory = TunnelFactory {
            bus: bus.clone(),
            clock,
            sessions: sessions.clone(),
            settings,
            probing,
            options,
            port_watch: Arc::clone(&port_watch),
        };
        let inner = Arc::new(EngineInner {
            store,
            bus,
            sessions,
            settings: settings_sender,
            probing: probing_sender,
            scanning: scanning_sender,
            port_watch,
            catalog: Mutex::new(Catalog::new(factory, document.tunnels)),
            mutation: tokio::sync::Mutex::new(()),
            reload_task: Mutex::new(None),
            scan_task: Mutex::new(None),
            is_shut_down: AtomicBool::new(false),
        });
        let reload_task = tokio::spawn(run_reload_loop(Arc::downgrade(&inner), watcher));
        *inner
            .reload_task
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(reload_task);
        let scan_task = tokio::spawn(run_scan_loop(
            Arc::downgrade(&inner),
            scanning,
            options.port_scan_interval,
        ));
        *inner
            .scan_task
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(scan_task);
        Ok(Self { inner })
    }

    /// The tunnels with their state, health and metrics, and the settings.
    pub fn snapshot(&self) -> EngineSnapshot {
        EngineSnapshot {
            tunnels: self.inner.catalog().snapshots(),
            settings: self.inner.settings.borrow().clone(),
            externals: self.inner.port_watch.externals(),
        }
    }

    /// Receives every event from now on. A subscriber that falls more than
    /// 1024 events behind loses the oldest ones.
    pub fn subscribe(&self) -> broadcast::Receiver<EngineEvent> {
        self.inner.bus.subscribe()
    }

    /// Creates a tunnel, saves it to the store and returns its new id. The
    /// tunnel starts stopped.
    pub async fn create_tunnel(&self, new_tunnel: NewTunnel) -> Result<TunnelId, EngineError> {
        let _serialized = self.inner.mutation.lock().await;
        self.ensure_running()?;
        let definition = new_tunnel.into_definition(TunnelId::generate());
        let id = definition.id;
        let mut tunnels = self.inner.catalog().definitions();
        tunnels.push(definition.clone());
        self.inner.persist_tunnels(tunnels).await?;
        self.inner.catalog().add(definition);
        self.inner.bus.publish(EngineEvent::ConfigReloaded);
        Ok(id)
    }

    /// Replaces the definition of the tunnel that has the same id, saves it,
    /// and applies it: a running tunnel restarts when the host, kind or an
    /// address changed, and keeps running when only its name or tags did.
    pub async fn update_tunnel(&self, definition: TunnelDefinition) -> Result<(), EngineError> {
        let _serialized = self.inner.mutation.lock().await;
        self.ensure_running()?;
        let tunnels: Vec<TunnelDefinition> = self
            .inner
            .catalog()
            .definitions()
            .into_iter()
            .map(|existing| {
                if existing.id == definition.id {
                    definition.clone()
                } else {
                    existing
                }
            })
            .collect();
        if !self.inner.catalog().contains(definition.id) {
            return Err(EngineError::UnknownTunnel(definition.id));
        }
        self.inner.persist_tunnels(tunnels).await?;
        self.inner.catalog().replace(definition);
        self.inner.bus.publish(EngineEvent::ConfigReloaded);
        Ok(())
    }

    /// Validates the settings, saves them to the store and applies them: new
    /// sessions and the running tunnels see them at once.
    pub async fn update_settings(&self, settings: Settings) -> Result<(), EngineError> {
        let _serialized = self.inner.mutation.lock().await;
        self.ensure_running()?;
        let mut document = self.inner.store.document();
        document.settings = settings.clone();
        let store = Arc::clone(&self.inner.store);
        run_blocking(move || store.save(document)).await?;
        self.inner.settings.send_replace(settings);
        self.inner.bus.publish(EngineEvent::ConfigReloaded);
        Ok(())
    }

    /// Stops the tunnel if it runs, removes it and saves the store.
    pub async fn delete_tunnel(&self, id: TunnelId) -> Result<(), EngineError> {
        let _serialized = self.inner.mutation.lock().await;
        self.ensure_running()?;
        if !self.inner.catalog().contains(id) {
            return Err(EngineError::UnknownTunnel(id));
        }
        let mut tunnels = self.inner.catalog().definitions();
        tunnels.retain(|definition| definition.id != id);
        self.inner.persist_tunnels(tunnels).await?;
        let stopping = self.inner.catalog().remove(id);
        if let Some(task) = stopping {
            let _ = task.await;
        }
        self.inner.bus.publish(EngineEvent::ConfigReloaded);
        Ok(())
    }

    /// Starts the tunnel. Does nothing when it already runs.
    pub fn start_tunnel(&self, id: TunnelId) -> Result<(), EngineError> {
        self.send(id, TunnelCommand::Start)
    }

    /// Stops the tunnel. Does nothing when it is already stopped.
    pub fn stop_tunnel(&self, id: TunnelId) -> Result<(), EngineError> {
        self.send(id, TunnelCommand::Stop)
    }

    /// Stops the tunnel and starts it again; starts it when it was stopped.
    pub fn restart_tunnel(&self, id: TunnelId) -> Result<(), EngineError> {
        self.send(id, TunnelCommand::Restart)
    }

    /// Starts every tunnel carrying the tag and returns their ids.
    pub fn start_tag(&self, tag: &str) -> Result<Vec<TunnelId>, EngineError> {
        self.send_to_tag(tag, || TunnelCommand::Start)
    }

    /// Stops every tunnel carrying the tag and returns their ids.
    pub fn stop_tag(&self, tag: &str) -> Result<Vec<TunnelId>, EngineError> {
        self.send_to_tag(tag, || TunnelCommand::Stop)
    }

    /// The most recent log entries of the tunnel, oldest first.
    pub fn logs(&self, id: TunnelId) -> Result<Vec<TunnelLogEntry>, EngineError> {
        self.inner
            .catalog()
            .logs(id)
            .ok_or(EngineError::UnknownTunnel(id))
    }

    /// Turns the periodic health probes on or off. They run for active
    /// tunnels while this is on and `Settings::health_probe` is set; the
    /// daemon turns them on while a UI is connected.
    pub fn set_health_probing(&self, is_enabled: bool) {
        self.inner.probing.send_replace(is_enabled);
    }

    /// Turns the periodic scan of the listening ports on or off. While it is
    /// on, tunnels whose port is held by another process report `External` or
    /// `Conflict` and unmanaged `ssh` forwards are listed; the daemon turns it
    /// on while a UI is connected, like health probing.
    pub fn set_port_scanning(&self, is_enabled: bool) {
        self.inner.scanning.send_if_modified(|current| {
            let is_changed = *current != is_enabled;
            *current = is_enabled;
            is_changed
        });
    }

    /// Terminates the external process providing the tunnel, waits until it
    /// released its ports, then starts every tunnel it provided and returns
    /// their ids.
    pub async fn adopt_external(&self, id: TunnelId) -> Result<Vec<TunnelId>, EngineError> {
        self.ensure_running()?;
        self.inner.adopt_external(id).await
    }

    /// Terminates a reported process: the holder of a tunnel port or an
    /// unmanaged `ssh` process.
    pub async fn kill_external(&self, pid: u32) -> Result<(), EngineError> {
        self.ensure_running()?;
        self.inner.kill_external(pid).await
    }

    /// Stops reporting a process: the holder of a tunnel port or an unmanaged
    /// `ssh` process.
    pub async fn ignore_external(&self, pid: u32) -> Result<(), EngineError> {
        self.ensure_running()?;
        self.inner.ignore_external(pid).await
    }

    /// How many tunnels hold resources: starting, active or reconnecting.
    pub fn active_tunnel_count(&self) -> usize {
        self.inner.catalog().active_count()
    }

    /// Stops every tunnel, closes every session and waits for the tasks to
    /// end. The engine accepts no further changes.
    pub async fn shutdown(&self) {
        self.inner.is_shut_down.store(true, Ordering::SeqCst);
        let reload_task = self
            .inner
            .reload_task
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(task) = reload_task {
            task.abort();
        }
        let scan_task = self
            .inner
            .scan_task
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(task) = scan_task {
            task.abort();
        }
        let _serialized = self.inner.mutation.lock().await;
        let stopping = self.inner.catalog().terminate_all();
        for task in stopping {
            let _ = task.await;
        }
        self.inner.sessions.shutdown().await;
    }

    fn ensure_running(&self) -> Result<(), EngineError> {
        if self.inner.is_shut_down.load(Ordering::SeqCst) {
            return Err(EngineError::ShutDown);
        }
        Ok(())
    }

    fn send(&self, id: TunnelId, command: TunnelCommand) -> Result<(), EngineError> {
        self.ensure_running()?;
        if self.inner.catalog().send(id, command) {
            return Ok(());
        }
        Err(EngineError::UnknownTunnel(id))
    }

    fn send_to_tag(
        &self,
        tag: &str,
        command: fn() -> TunnelCommand,
    ) -> Result<Vec<TunnelId>, EngineError> {
        self.ensure_running()?;
        let catalog = self.inner.catalog();
        let ids = catalog.ids_with_tag(tag);
        if ids.is_empty() {
            return Err(EngineError::UnknownTag(tag.to_owned()));
        }
        for id in &ids {
            catalog.send(*id, command());
        }
        Ok(ids)
    }
}
