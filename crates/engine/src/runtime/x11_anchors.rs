use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::clock::Clock;
use crate::runtime::events::EventBus;
use crate::runtime::session_manager::SessionManager;
use crate::runtime::x11_anchor::{
    AnchorCommand, AnchorContext, AnchorHandle, X11AnchorShared, spawn_anchor,
};
use crate::runtime::x11_state::{X11AnchorSnapshot, X11AnchorState};
use crate::store::Settings;
use crate::x11::LocalX11Resolver;

/// The X11 anchors of the hosts, one task per host that was ever started.
pub(crate) struct X11Anchors {
    bus: EventBus,
    clock: Arc<dyn Clock>,
    sessions: SessionManager,
    resolver: Arc<LocalX11Resolver>,
    settings: watch::Receiver<Settings>,
    handles: Mutex<BTreeMap<String, AnchorHandle>>,
}

impl X11Anchors {
    pub(crate) fn new(
        bus: EventBus,
        clock: Arc<dyn Clock>,
        sessions: SessionManager,
        resolver: Arc<LocalX11Resolver>,
        settings: watch::Receiver<Settings>,
    ) -> Self {
        Self {
            bus,
            clock,
            sessions,
            resolver,
            settings,
            handles: Mutex::new(BTreeMap::new()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, AnchorHandle>> {
        self.handles.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts the anchor of `host`; does nothing when it already runs.
    pub(crate) fn start(&self, host: &str) {
        let mut handles = self.lock();
        let handle = handles
            .entry(host.to_owned())
            .or_insert_with(|| spawn_anchor(self.context_for(host)));
        let _ = handle.commands.send(AnchorCommand::Start);
    }

    /// Stops the anchor of `host`; does nothing when it is stopped.
    pub(crate) fn stop(&self, host: &str) {
        if let Some(handle) = self.lock().get(host) {
            let _ = handle.commands.send(AnchorCommand::Stop);
        }
    }

    /// The anchors that are not stopped, sorted by host.
    pub(crate) fn snapshots(&self) -> Vec<X11AnchorSnapshot> {
        self.lock()
            .iter()
            .map(|(host, handle)| X11AnchorSnapshot {
                host: host.clone(),
                state: handle.shared.state(),
            })
            .filter(|snapshot| snapshot.state != X11AnchorState::Stopped)
            .collect()
    }

    /// How many anchors hold resources: starting, active or reconnecting.
    pub(crate) fn active_count(&self) -> usize {
        self.lock()
            .values()
            .filter(|handle| handle.shared.state().is_running())
            .count()
    }

    /// Asks every anchor to stop for good and returns their tasks.
    pub(crate) fn terminate_all(&self) -> Vec<JoinHandle<()>> {
        let handles = std::mem::take(&mut *self.lock());
        handles
            .into_values()
            .filter_map(|mut handle| {
                let _ = handle.commands.send(AnchorCommand::Terminate);
                handle.task.take()
            })
            .collect()
    }

    fn context_for(&self, host: &str) -> AnchorContext {
        AnchorContext {
            host: host.to_owned(),
            shared: X11AnchorShared::new(host.to_owned(), self.bus.clone()),
            sessions: self.sessions.clone(),
            resolver: Arc::clone(&self.resolver),
            settings: self.settings.clone(),
            clock: Arc::clone(&self.clock),
        }
    }
}
