use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::clock::Clock;
use crate::external::{ProcessIdentity, SystemScan, occupation_of};
use crate::runtime::definition_diff::{ReloadPlan, plan_reload};
use crate::runtime::events::EventBus;
use crate::runtime::log_buffer::TunnelLogEntry;
use crate::runtime::metrics::TunnelMetrics;
use crate::runtime::options::EngineOptions;
use crate::runtime::port_watch::PortWatch;
use crate::runtime::session_manager::SessionManager;
use crate::runtime::shared::TunnelShared;
use crate::runtime::state::{TargetHealth, TunnelState};
use crate::runtime::tunnel_actor::{TunnelCommand, TunnelContext, TunnelHandle, spawn_tunnel};
use crate::store::{Settings, TunnelDefinition, TunnelId, TunnelKind};

/// A tunnel definition, its state, health and metrics at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelSnapshot {
    /// The configured tunnel.
    pub definition: TunnelDefinition,
    /// Its life-cycle state.
    pub state: TunnelState,
    /// The health of its target.
    pub health: TargetHealth,
    /// What it carried.
    pub metrics: TunnelMetrics,
    /// Whether the tunnel lives only in the daemon: it is never saved to the
    /// store and is removed when it is stopped.
    pub is_temporary: bool,
}

/// Builds the task of a tunnel.
pub(crate) struct TunnelFactory {
    pub(crate) bus: EventBus,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) sessions: SessionManager,
    pub(crate) settings: watch::Receiver<Settings>,
    pub(crate) probing: watch::Receiver<bool>,
    pub(crate) options: EngineOptions,
    pub(crate) port_watch: Arc<PortWatch>,
}

impl TunnelFactory {
    fn spawn(&self, definition: TunnelDefinition, is_temporary: bool) -> TunnelEntry {
        let shared = TunnelShared::new(definition.id, self.bus.clone(), Arc::clone(&self.clock));
        let handle = spawn_tunnel(TunnelContext {
            definition: definition.clone(),
            shared,
            sessions: self.sessions.clone(),
            settings: self.settings.clone(),
            probing: self.probing.clone(),
            options: self.options,
            port_watch: Arc::clone(&self.port_watch),
        });
        TunnelEntry {
            definition,
            handle,
            is_temporary,
        }
    }
}

pub(crate) struct TunnelEntry {
    definition: TunnelDefinition,
    handle: TunnelHandle,
    is_temporary: bool,
}

impl TunnelEntry {
    fn send(&self, command: TunnelCommand) {
        let _ = self.handle.commands.send(command);
    }

    fn snapshot(&self) -> TunnelSnapshot {
        let status = self.handle.shared.status();
        TunnelSnapshot {
            definition: self.definition.clone(),
            state: status.state,
            health: status.health,
            metrics: self.handle.shared.metrics(),
            is_temporary: self.is_temporary,
        }
    }

    fn terminate(mut self) -> Option<JoinHandle<()>> {
        self.send(TunnelCommand::Terminate);
        self.handle.task.take()
    }
}

/// The tunnels the engine runs, in configuration order, then the temporary
/// ones in the order they were started.
///
/// Temporary tunnels live only here. The only way to read definitions for the
/// store is [`persistent_definitions`](Self::persistent_definitions), which
/// leaves them out, so a save cannot write one by forgetting to filter.
pub(crate) struct Catalog {
    factory: TunnelFactory,
    entries: Vec<TunnelEntry>,
}

impl Catalog {
    pub(crate) fn new(factory: TunnelFactory, definitions: Vec<TunnelDefinition>) -> Self {
        let entries = definitions
            .into_iter()
            .map(|definition| factory.spawn(definition, false))
            .collect();
        Self { factory, entries }
    }

    pub(crate) fn snapshots(&self) -> Vec<TunnelSnapshot> {
        self.entries.iter().map(TunnelEntry::snapshot).collect()
    }

    /// The definitions that belong in the store: every tunnel but the
    /// temporary ones.
    pub(crate) fn persistent_definitions(&self) -> Vec<TunnelDefinition> {
        self.entries
            .iter()
            .filter(|entry| !entry.is_temporary)
            .map(|entry| entry.definition.clone())
            .collect()
    }

    /// The definitions of every tunnel the daemon runs, temporary ones
    /// included. Never write these to the store.
    pub(crate) fn running_definitions(&self) -> Vec<TunnelDefinition> {
        self.entries
            .iter()
            .map(|entry| entry.definition.clone())
            .collect()
    }

    /// The ids of the tunnels the store knows.
    pub(crate) fn persistent_ids(&self) -> Vec<TunnelId> {
        self.entries
            .iter()
            .filter(|entry| !entry.is_temporary)
            .map(|entry| entry.definition.id)
            .collect()
    }

    pub(crate) fn is_temporary(&self, id: TunnelId) -> bool {
        self.find(id).is_some_and(|entry| entry.is_temporary)
    }

    pub(crate) fn is_running(&self, id: TunnelId) -> bool {
        self.find(id)
            .is_some_and(|entry| entry.handle.shared.status().state.is_running())
    }

    /// The tunnels to start at launch, in configuration order: those in
    /// `restored` and those carrying one of `tags`.
    pub(crate) fn ids_to_start_on_launch(
        &self,
        restored: Option<&BTreeSet<TunnelId>>,
        tags: &[String],
    ) -> Vec<TunnelId> {
        self.entries
            .iter()
            .filter(|entry| !entry.is_temporary)
            .filter(|entry| {
                let definition = &entry.definition;
                restored.is_some_and(|ids| ids.contains(&definition.id))
                    || definition.tags.iter().any(|tag| tags.contains(tag))
            })
            .map(|entry| entry.definition.id)
            .collect()
    }

    pub(crate) fn contains(&self, id: TunnelId) -> bool {
        self.find(id).is_some()
    }

    pub(crate) fn logs(&self, id: TunnelId) -> Option<Vec<TunnelLogEntry>> {
        self.find(id).map(|entry| entry.handle.shared.logs())
    }

    pub(crate) fn active_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.handle.shared.status().state.is_running())
            .count()
    }

    pub(crate) fn ids_with_tag(&self, tag: &str) -> Vec<TunnelId> {
        self.entries
            .iter()
            .filter(|entry| {
                entry
                    .definition
                    .tags
                    .iter()
                    .any(|candidate| candidate == tag)
            })
            .map(|entry| entry.definition.id)
            .collect()
    }

    pub(crate) fn send(&self, id: TunnelId, command: TunnelCommand) -> bool {
        self.find(id).map(|entry| entry.send(command)).is_some()
    }

    pub(crate) fn observe_ports(
        &self,
        settings: &Settings,
        scan: &SystemScan,
        ignored: &HashSet<ProcessIdentity>,
        own_pid: u32,
    ) {
        for entry in &self.entries {
            if entry.definition.kind == TunnelKind::Remote {
                continue;
            }
            let occupation = occupation_of(&entry.definition, settings, scan, ignored, own_pid);
            entry.send(TunnelCommand::PortObserved(occupation));
        }
    }

    pub(crate) fn add(&mut self, definition: TunnelDefinition) {
        let entry = self.factory.spawn(definition, false);
        let position = self
            .entries
            .iter()
            .position(|existing| existing.is_temporary)
            .unwrap_or(self.entries.len());
        self.entries.insert(position, entry);
    }

    pub(crate) fn add_temporary(&mut self, definition: TunnelDefinition) {
        self.entries.push(self.factory.spawn(definition, true));
    }

    /// Turns a temporary tunnel into a persistent one; `false` when there is
    /// no temporary tunnel with this id.
    pub(crate) fn mark_persistent(&mut self, id: TunnelId) -> bool {
        self.set_temporary(id, true, false)
    }

    /// Turns a persistent tunnel back into a temporary one; `false` when
    /// there is no persistent tunnel with this id.
    pub(crate) fn mark_temporary(&mut self, id: TunnelId) -> bool {
        self.set_temporary(id, false, true)
    }

    fn set_temporary(&mut self, id: TunnelId, from: bool, to: bool) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.definition.id == id && entry.is_temporary == from)
        else {
            return false;
        };
        entry.is_temporary = to;
        self.entries.sort_by_key(|entry| entry.is_temporary);
        true
    }

    /// Removes a temporary tunnel and tells it to terminate; `false` when
    /// there is no temporary tunnel with this id. The tunnel finishes
    /// stopping in the background.
    pub(crate) fn remove_temporary(&mut self, id: TunnelId) -> bool {
        let Some(position) = self
            .entries
            .iter()
            .position(|entry| entry.definition.id == id && entry.is_temporary)
        else {
            return false;
        };
        self.entries.remove(position).terminate();
        true
    }

    pub(crate) fn replace(&mut self, definition: TunnelDefinition) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.definition.id == definition.id)
        else {
            return false;
        };
        entry.send(TunnelCommand::UpdateDefinition(definition.clone()));
        entry.definition = definition;
        true
    }

    pub(crate) fn remove(&mut self, id: TunnelId) -> Option<JoinHandle<()>> {
        let position = self
            .entries
            .iter()
            .position(|entry| entry.definition.id == id)?;
        self.entries.remove(position).terminate()
    }

    pub(crate) fn apply(&mut self, next: &[TunnelDefinition]) -> Vec<JoinHandle<()>> {
        let ReloadPlan {
            removed,
            added,
            changed,
        } = plan_reload(&self.persistent_definitions(), next);
        let stopping = removed
            .into_iter()
            .filter_map(|id| self.remove(id))
            .collect();
        for definition in changed {
            self.replace(definition);
        }
        for definition in added {
            self.add(definition);
        }
        self.order_like(next);
        stopping
    }

    pub(crate) fn terminate_all(&mut self) -> Vec<JoinHandle<()>> {
        self.entries
            .drain(..)
            .filter_map(TunnelEntry::terminate)
            .collect()
    }

    fn order_like(&mut self, definitions: &[TunnelDefinition]) {
        self.entries.sort_by_cached_key(|entry| {
            definitions
                .iter()
                .position(|definition| definition.id == entry.definition.id)
                .unwrap_or(usize::MAX)
        });
    }

    fn find(&self, id: TunnelId) -> Option<&TunnelEntry> {
        self.entries.iter().find(|entry| entry.definition.id == id)
    }
}
