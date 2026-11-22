//! Opening one devslot instance in one step: listing the host, removing the
//! stale routes that need no question, tunneling the web ports of the
//! instance and waiting for the tunnels.

use tokio::task::JoinSet;

use crate::devslot::{
    DevslotError, DevslotFailedPort, DevslotInstance, DevslotRemovedRoute, DevslotResyncTunnel,
    DevslotStaleAction, DevslotStaleTunnel, DevslotUp, DevslotUpPlan, DevslotUpPort, PlannedTunnel,
    UpPlanInput, UpTunnelKind, plan_devslot_up,
};
use crate::runtime::devslot_service::{DevslotAnalysis, RemoteDevslotListing};
use crate::runtime::engine::{Engine, EngineError};
use crate::store::TunnelId;

const WAIT_ENDED_MESSAGE: &str = "waiting for the tunnel ended unexpectedly";

/// What the locked part of the opening decided and did.
struct Applied {
    host: String,
    instance: String,
    plan: DevslotUpPlan,
    app_url: Option<String>,
    removed: Vec<DevslotRemovedRoute>,
    needs_review: usize,
    started: Vec<DevslotUpPort>,
    existing: Vec<DevslotUpPort>,
    failed: Vec<DevslotFailedPort>,
}

impl Engine {
    /// Opens one devslot instance of a host: lists the instances there (one
    /// `ds ls --json`, over the shared SSH session), removes the stale routes
    /// that need no question, starts a tunnel for each web port of the
    /// instance that has none, starts the stopped tunnels it already has, and
    /// waits until they are ready.
    ///
    /// Only the automatic part of the resync runs: temporary tunnels whose
    /// port is gone, and saved ones when `devslot_stale` says to delete them,
    /// under the guard against deleting every saved tunnel. A port held by a
    /// tunnel that waits for the user's decision is left alone and reported.
    /// A port that fails is reported in the result, not as an error.
    ///
    /// The listing runs before any lock is taken. The steps that read and
    /// change the tunnels of this machine are serialised, so that two calls
    /// for the same instance create each tunnel once; the lock is released
    /// before the wait.
    pub async fn devslot_up(&self, host: &str, instance: &str) -> Result<DevslotUp, EngineError> {
        self.ensure_running()?;
        let listing = self.inner.list_devslot(host).await?;
        let applied = self.apply_devslot_up(host, instance, listing).await?;
        Ok(self.wait_for_devslot_up(applied).await)
    }

    /// Runs the locked steps in a task of their own: a caller that goes away
    /// (a dropped connection) must not leave a store write half applied.
    async fn apply_devslot_up(
        &self,
        host: &str,
        instance: &str,
        listing: RemoteDevslotListing,
    ) -> Result<Applied, EngineError> {
        let engine = self.clone();
        let (host, instance) = (host.to_owned(), instance.to_owned());
        tokio::spawn(async move {
            engine
                .apply_devslot_up_locked(&host, &instance, &listing)
                .await
        })
        .await
        .map_err(|_| EngineError::DevslotTaskFailed)?
    }

    async fn apply_devslot_up_locked(
        &self,
        host: &str,
        instance: &str,
        listing: &RemoteDevslotListing,
    ) -> Result<Applied, EngineError> {
        let _exclusive = self.inner.devslot_up_lock.lock().await;
        self.ensure_running()?;
        let first = self.inner.analyse_devslot(host, listing).await;
        find_instance(host, instance, &first)?;
        let removed = self.remove_stale_routes(host, &first).await?;
        let analysis = if removed.is_empty() {
            first
        } else {
            self.inner.analyse_devslot(host, listing).await
        };
        let target = find_instance(host, instance, &analysis)?;
        let default_bind_address = self.inner.settings.borrow().default_bind_address.clone();
        let plan = plan_devslot_up(&UpPlanInput {
            host,
            instance: target,
            all_instances: &analysis.discovery.instances,
            is_persisted: analysis.discovery.persist_default,
            stale: &analysis.discovery.stale,
            known: &analysis.known,
            default_bind_address: &default_bind_address,
        });
        let tunnels = self.create_planned_tunnels(&plan).await;
        Ok(Applied {
            host: host.to_owned(),
            instance: instance.to_owned(),
            app_url: target.app_url.clone(),
            removed,
            needs_review: analysis.discovery.stale.len(),
            started: tunnels.started,
            existing: tunnels.existing,
            failed: tunnels.failed,
            plan,
        })
    }

    async fn remove_stale_routes(
        &self,
        host: &str,
        analysis: &DevslotAnalysis,
    ) -> Result<Vec<DevslotRemovedRoute>, EngineError> {
        let stale = &analysis.discovery.stale;
        let requested: Vec<DevslotResyncTunnel> = stale
            .iter()
            .filter(|tunnel| tunnel.action == DevslotStaleAction::Remove)
            .map(|tunnel| DevslotResyncTunnel {
                definition: tunnel.definition.clone(),
                is_temporary: tunnel.is_temporary,
            })
            .collect();
        if requested.is_empty() {
            return Ok(Vec::new());
        }
        let resync = self.inner.resync_devslot(host, requested).await?;
        Ok(stale
            .iter()
            .filter(|tunnel| resync.removed.contains(&tunnel.definition.id))
            .map(describe_removed_route)
            .collect())
    }

    async fn create_planned_tunnels(&self, plan: &DevslotUpPlan) -> CreatedTunnels {
        let mut created = CreatedTunnels::default();
        for planned in &plan.to_start {
            match self.create_planned_tunnel(planned).await {
                Ok(tunnel_id) => created.started.push(DevslotUpPort {
                    remote_port: planned.remote_port,
                    listen_port: planned.tunnel.listen_port,
                    tunnel_id,
                }),
                Err(error) => created
                    .failed
                    .push(describe_failure(planned.remote_port, &error)),
            }
        }
        for existing in &plan.existing {
            match self.start_tunnel(existing.tunnel_id) {
                Ok(()) => created.existing.push(*existing),
                Err(error) => created
                    .failed
                    .push(describe_failure(existing.remote_port, &error)),
            }
        }
        created
    }

    async fn create_planned_tunnel(
        &self,
        planned: &PlannedTunnel,
    ) -> Result<TunnelId, EngineError> {
        match &planned.kind {
            UpTunnelKind::Temporary(origin) => {
                self.start_temporary_tunnel(planned.tunnel.clone(), Some(origin.clone()))
                    .await
            }
            UpTunnelKind::Saved => {
                let id = self.create_tunnel(planned.tunnel.clone()).await?;
                self.start_tunnel(id)?;
                Ok(id)
            }
        }
    }

    async fn wait_for_devslot_up(&self, applied: Applied) -> DevslotUp {
        let Applied {
            host,
            instance,
            plan,
            app_url,
            removed,
            needs_review,
            started,
            existing,
            mut failed,
        } = applied;
        let tunneled: Vec<DevslotUpPort> = started.iter().chain(&existing).copied().collect();
        let outcomes = self.wait_for_tunnels(&tunneled).await;
        let (ready, not_ready) = split_by_outcome(&tunneled, outcomes);
        failed.extend(not_ready);
        failed.sort_by_key(|port| port.remote_port);
        let is_ready = |port: &DevslotUpPort| ready.contains(port);
        let app_tunnel = plan.app_port.and_then(|app_port| {
            ready
                .iter()
                .find(|port| port.remote_port == app_port)
                .map(|port| port.tunnel_id)
        });
        DevslotUp {
            host,
            instance,
            started: started.into_iter().filter(is_ready).collect(),
            existing: existing.into_iter().filter(is_ready).collect(),
            skipped: plan.skipped,
            failed,
            removed,
            needs_review,
            app_tunnel,
            app_url,
            is_app_port_shared: plan.is_app_port_shared,
        }
    }

    async fn wait_for_tunnels(&self, ports: &[DevslotUpPort]) -> Vec<Result<(), String>> {
        let mut waiting = JoinSet::new();
        for (index, port) in ports.iter().enumerate() {
            let engine = self.clone();
            let tunnel_id = port.tunnel_id;
            waiting.spawn(async move {
                let outcome = engine.start_tunnel_and_wait(tunnel_id).await;
                (index, outcome.map_err(|error| error.to_string()))
            });
        }
        let mut outcomes: Vec<Option<Result<(), String>>> = vec![None; ports.len()];
        while let Some(joined) = waiting.join_next().await {
            if let Ok((index, outcome)) = joined {
                outcomes[index] = Some(outcome);
            }
        }
        outcomes
            .into_iter()
            .map(|outcome| outcome.unwrap_or_else(|| Err(WAIT_ENDED_MESSAGE.to_owned())))
            .collect()
    }
}

#[derive(Default)]
struct CreatedTunnels {
    started: Vec<DevslotUpPort>,
    existing: Vec<DevslotUpPort>,
    failed: Vec<DevslotFailedPort>,
}

fn find_instance<'analysis>(
    host: &str,
    instance: &str,
    analysis: &'analysis DevslotAnalysis,
) -> Result<&'analysis DevslotInstance, DevslotError> {
    analysis
        .discovery
        .instances
        .iter()
        .find(|listed| listed.name == instance)
        .ok_or_else(|| DevslotError::InstanceNotFound {
            host: host.to_owned(),
            instance: instance.to_owned(),
        })
}

fn describe_removed_route(tunnel: &DevslotStaleTunnel) -> DevslotRemovedRoute {
    DevslotRemovedRoute {
        listen_port: tunnel.definition.listen_port,
        remote_port: tunnel.definition.effective_target_port(),
        is_temporary: tunnel.is_temporary,
    }
}

fn describe_failure(remote_port: u16, error: &EngineError) -> DevslotFailedPort {
    DevslotFailedPort {
        remote_port,
        message: error.to_string(),
    }
}

fn split_by_outcome(
    ports: &[DevslotUpPort],
    outcomes: Vec<Result<(), String>>,
) -> (Vec<DevslotUpPort>, Vec<DevslotFailedPort>) {
    let mut ready = Vec::new();
    let mut not_ready = Vec::new();
    for (port, outcome) in ports.iter().zip(outcomes) {
        match outcome {
            Ok(()) => ready.push(*port),
            Err(message) => not_ready.push(DevslotFailedPort {
                remote_port: port.remote_port,
                message,
            }),
        }
    }
    (ready, not_ready)
}
