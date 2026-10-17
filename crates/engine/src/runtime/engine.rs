use std::collections::{BTreeSet, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use thiserror::Error;
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

use crate::blocking::run_blocking;
use crate::clock::Clock;
use crate::connector::{Connector, ConnectorDependencies};
use crate::external::{PortScanner, ScanError, TerminateError, UnmanagedProcess};
use crate::keyring::SecretKeyring;
use crate::passphrase_vault::PassphraseVault;
use crate::runtime::catalog::{Catalog, TunnelFactory, TunnelSnapshot};
use crate::runtime::events::{EngineEvent, EventBus};
use crate::runtime::log_buffer::TunnelLogEntry;
use crate::runtime::options::EngineOptions;
use crate::runtime::port_watch::{PortWatch, run_scan_loop};
use crate::runtime::reload::run_reload_loop;
use crate::runtime::session_manager::SessionManager;
use crate::runtime::tunnel_actor::TunnelCommand;
use crate::runtime::x11_anchors::X11Anchors;
use crate::runtime::x11_launcher::X11Launches;
use crate::runtime::x11_state::X11AnchorSnapshot;
use crate::state_tracker::StateTracker;
use crate::store::{
    Settings, StateStore, Store, StoreError, TunnelDefinition, TunnelId, TunnelKind, X11Launcher,
};
use crate::x11::{LocalX11Resolver, X11Environment, X11Error};

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
    /// The host has no X11 launcher with this name.
    #[error("host {host} has no X11 launcher named \"{name}\"")]
    UnknownLauncher {
        /// The host alias.
        host: String,
        /// The launcher name.
        name: String,
    },
    /// X11 forwarding could not be set up.
    #[error(transparent)]
    X11(#[from] X11Error),
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
    /// Where the local X display and its credentials are read from. Kept
    /// apart from the SSH configuration environment: it reads `$DISPLAY`,
    /// `$XAUTHORITY` and the `Xauthority` location, nothing else.
    pub x11_environment: Arc<dyn X11Environment>,
    /// Where the tunnels the user wants running and the keyring entries are
    /// remembered between two runs.
    pub state_store: Arc<dyn StateStore>,
    /// The OS keyring that holds remembered passphrases while
    /// `Settings::use_os_keyring` is on; `None` when there is none.
    pub keyring: Option<Arc<dyn SecretKeyring>>,
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
    /// The X11 anchors that are not stopped, sorted by host.
    pub x11_anchors: Vec<X11AnchorSnapshot>,
    /// The saved X11 launchers.
    pub x11_launchers: Vec<X11Launcher>,
}

pub(crate) struct EngineInner {
    pub(crate) store: Arc<Store>,
    pub(crate) bus: EventBus,
    pub(crate) sessions: SessionManager,
    pub(crate) settings: watch::Sender<Settings>,
    probing: watch::Sender<bool>,
    scanning: watch::Sender<bool>,
    pub(crate) port_watch: Arc<PortWatch>,
    x11_anchors: X11Anchors,
    x11_launches: X11Launches,
    state: Arc<StateTracker>,
    passphrases: Arc<PassphraseVault>,
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

    /// Remembers that the user wants these tunnels running.
    pub(crate) fn record_wanted(&self, ids: &[TunnelId]) {
        self.update_wanted(|running| running.extend(ids.iter().copied()));
    }

    /// Remembers that the user no longer wants these tunnels running.
    pub(crate) fn record_unwanted(&self, ids: &[TunnelId]) {
        self.update_wanted(|running| {
            for id in ids {
                running.remove(id);
            }
        });
    }

    fn update_wanted(&self, change: impl FnOnce(&mut BTreeSet<TunnelId>)) {
        let known: HashSet<TunnelId> = self.catalog().ids().into_iter().collect();
        self.state.update(|state| {
            change(&mut state.running);
            state.running.retain(|id| known.contains(id));
        });
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
            x11_environment,
            state_store,
            keyring,
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
        let state = StateTracker::load(state_store);
        let passphrases = Arc::new(PassphraseVault::new(
            keyring,
            settings.clone(),
            Arc::clone(&state),
        ));
        let sessions = SessionManager::new(
            Connector::with_settings(connector, settings.clone())
                .with_timeouts(options.connect_timeouts)
                .with_passphrase_vault(Arc::clone(&passphrases)),
            settings.clone(),
        );
        let x11_resolver = Arc::new(LocalX11Resolver::new(
            Arc::clone(&store),
            settings.clone(),
            x11_environment,
        ));
        let x11_anchors = X11Anchors::new(
            bus.clone(),
            Arc::clone(&clock),
            sessions.clone(),
            Arc::clone(&x11_resolver),
            settings.clone(),
        );
        let x11_launches = X11Launches::new(sessions.clone(), x11_resolver);
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
            x11_anchors,
            x11_launches,
            state,
            passphrases,
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
            x11_anchors: self.inner.x11_anchors.snapshots(),
            x11_launchers: self.inner.store.document().x11_launchers,
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
        self.inner.record_unwanted(&[id]);
        if let Some(task) = stopping {
            let _ = task.await;
        }
        self.inner.bus.publish(EngineEvent::ConfigReloaded);
        Ok(())
    }

    /// Starts the tunnel. Does nothing when it already runs.
    pub fn start_tunnel(&self, id: TunnelId) -> Result<(), EngineError> {
        self.send(id, TunnelCommand::Start)?;
        self.inner.record_wanted(&[id]);
        Ok(())
    }

    /// Stops the tunnel. Does nothing when it is already stopped.
    pub fn stop_tunnel(&self, id: TunnelId) -> Result<(), EngineError> {
        self.send(id, TunnelCommand::Stop)?;
        self.inner.record_unwanted(&[id]);
        Ok(())
    }

    /// Stops the tunnel and starts it again; starts it when it was stopped.
    pub fn restart_tunnel(&self, id: TunnelId) -> Result<(), EngineError> {
        self.send(id, TunnelCommand::Restart)
    }

    /// Starts every tunnel carrying the tag and returns their ids.
    pub fn start_tag(&self, tag: &str) -> Result<Vec<TunnelId>, EngineError> {
        let ids = self.send_to_tag(tag, || TunnelCommand::Start)?;
        self.inner.record_wanted(&ids);
        Ok(ids)
    }

    /// Stops every tunnel carrying the tag and returns their ids.
    pub fn stop_tag(&self, tag: &str) -> Result<Vec<TunnelId>, EngineError> {
        let ids = self.send_to_tag(tag, || TunnelCommand::Stop)?;
        self.inner.record_unwanted(&ids);
        Ok(ids)
    }

    /// Starts the tunnels selected for the launch of the daemon: those the
    /// user wanted running at the last exit when `Settings::restore_on_launch`
    /// is on, and those carrying a tag of `Settings::start_on_launch_tags`.
    /// Each starts once, in configuration order; the ids are returned. The
    /// starts are not recorded as wishes of the user. X11 anchors are not
    /// restored.
    pub fn start_launch_tunnels(&self) -> Result<Vec<TunnelId>, EngineError> {
        self.ensure_running()?;
        let settings = self.inner.settings.borrow().clone();
        let wanted = self.inner.state.snapshot().running;
        let restored = settings.restore_on_launch.then_some(&wanted);
        let catalog = self.inner.catalog();
        let ids = catalog.ids_to_start_on_launch(restored, &settings.start_on_launch_tags);
        for id in &ids {
            catalog.send(*id, TunnelCommand::Start);
        }
        Ok(ids)
    }

    /// Forgets every remembered passphrase: the ones in memory and the ones
    /// stored in the OS keyring. Returns how many identity files had one.
    pub async fn forget_secrets(&self) -> usize {
        self.inner.passphrases.forget_all().await
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

    /// Starts the X11 anchor of the host: a session channel that asks the
    /// server for X11 forwarding and reports the remote `DISPLAY`. Does
    /// nothing when it already runs.
    pub fn start_x11(&self, host: &str) -> Result<(), EngineError> {
        self.ensure_running()?;
        self.inner.x11_anchors.start(host);
        Ok(())
    }

    /// Stops the X11 anchor of the host. Does nothing when it is stopped.
    pub fn stop_x11(&self, host: &str) -> Result<(), EngineError> {
        self.ensure_running()?;
        self.inner.x11_anchors.stop(host);
        Ok(())
    }

    /// Runs the saved X11 launcher `name` of the host in a session with X11
    /// forwarding, and returns once the server accepted the command. It does
    /// not need the anchor of the host.
    pub async fn launch_x11(&self, host: &str, name: &str) -> Result<(), EngineError> {
        self.ensure_running()?;
        let launcher = self
            .inner
            .store
            .document()
            .x11_launchers
            .into_iter()
            .find(|launcher| launcher.host == host && launcher.name == name)
            .ok_or_else(|| EngineError::UnknownLauncher {
                host: host.to_owned(),
                name: name.to_owned(),
            })?;
        Ok(self.inner.x11_launches.launch(launcher).await?)
    }

    /// How many X11 anchors hold resources: starting, active or reconnecting.
    pub fn active_x11_count(&self) -> usize {
        self.inner.x11_anchors.active_count()
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
        for task in stopping
            .into_iter()
            .chain(self.inner.x11_anchors.terminate_all())
        {
            let _ = task.await;
        }
        self.inner.sessions.shutdown().await;
        self.inner.state.flush().await;
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
