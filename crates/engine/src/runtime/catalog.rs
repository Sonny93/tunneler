use std::sync::Arc;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::clock::Clock;
use crate::runtime::definition_diff::{ReloadPlan, plan_reload};
use crate::runtime::events::EventBus;
use crate::runtime::log_buffer::TunnelLogEntry;
use crate::runtime::metrics::TunnelMetrics;
use crate::runtime::options::EngineOptions;
use crate::runtime::session_manager::SessionManager;
use crate::runtime::shared::TunnelShared;
use crate::runtime::state::{TargetHealth, TunnelState};
use crate::runtime::tunnel_actor::{TunnelCommand, TunnelContext, TunnelHandle, spawn_tunnel};
use crate::store::{Settings, TunnelDefinition, TunnelId};

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
}

/// Builds the task of a tunnel.
pub(crate) struct TunnelFactory {
    pub(crate) bus: EventBus,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) sessions: SessionManager,
    pub(crate) settings: watch::Receiver<Settings>,
    pub(crate) probing: watch::Receiver<bool>,
    pub(crate) options: EngineOptions,
}

impl TunnelFactory {
    fn spawn(&self, definition: TunnelDefinition) -> TunnelEntry {
        let shared = TunnelShared::new(definition.id, self.bus.clone(), Arc::clone(&self.clock));
        let handle = spawn_tunnel(TunnelContext {
            definition: definition.clone(),
            shared,
            sessions: self.sessions.clone(),
            settings: self.settings.clone(),
            probing: self.probing.clone(),
            options: self.options,
        });
        TunnelEntry { definition, handle }
    }
}

pub(crate) struct TunnelEntry {
    definition: TunnelDefinition,
    handle: TunnelHandle,
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
        }
    }

    fn terminate(mut self) -> Option<JoinHandle<()>> {
        self.send(TunnelCommand::Terminate);
        self.handle.task.take()
    }
}

/// The tunnels the engine runs, in configuration order.
pub(crate) struct Catalog {
    factory: TunnelFactory,
    entries: Vec<TunnelEntry>,
}

impl Catalog {
    pub(crate) fn new(factory: TunnelFactory, definitions: Vec<TunnelDefinition>) -> Self {
        let entries = definitions
            .into_iter()
            .map(|definition| factory.spawn(definition))
            .collect();
        Self { factory, entries }
    }

    pub(crate) fn snapshots(&self) -> Vec<TunnelSnapshot> {
        self.entries.iter().map(TunnelEntry::snapshot).collect()
    }

    pub(crate) fn definitions(&self) -> Vec<TunnelDefinition> {
        self.entries
            .iter()
            .map(|entry| entry.definition.clone())
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

    pub(crate) fn add(&mut self, definition: TunnelDefinition) {
        self.entries.push(self.factory.spawn(definition));
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
        } = plan_reload(&self.definitions(), next);
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
        });
    }

    fn find(&self, id: TunnelId) -> Option<&TunnelEntry> {
        self.entries.iter().find(|entry| entry.definition.id == id)
    }
}
