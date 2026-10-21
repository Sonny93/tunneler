use std::sync::Arc;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;
use tokio::time::timeout;

use crate::error::SessionError;
use crate::runtime::engine::EngineError;
use crate::runtime::failure::is_target_unreachable;
use crate::runtime::local_forward::{ForwardTarget, TargetReport};
use crate::runtime::remote_forward::connect_to_target;
use crate::runtime::shared::TunnelShared;
use crate::runtime::state::TargetHealth;
use crate::session::SshSession;
use crate::store::{TunnelId, TunnelKind};

const OPEN_CHANNEL_TIMEOUT: Duration = Duration::from_secs(10);

/// Where the answer of an explicit target check goes.
pub(crate) type TargetCheckReply = oneshot::Sender<Result<TargetHealth, EngineError>>;

/// Everything the task of one explicit target check needs.
pub(crate) struct TargetCheckJob {
    pub(crate) id: TunnelId,
    pub(crate) kind: TunnelKind,
    pub(crate) session: Arc<SshSession>,
    pub(crate) target: ForwardTarget,
    pub(crate) shared: Arc<TunnelShared>,
    pub(crate) reply: TargetCheckReply,
}

/// Checks the target once, records the outcome in the health of the tunnel
/// like a passive report, and answers with the resulting health.
pub(crate) async fn check_target(job: TargetCheckJob) {
    let TargetCheckJob {
        id,
        kind,
        session,
        target,
        shared,
        reply,
    } = job;
    let outcome = match kind {
        TunnelKind::Local => probe_through_session(&session, &target).await,
        TunnelKind::Remote => Ok(probe_local_target(&target).await),
        TunnelKind::Dynamic => Err(EngineError::TunnelHasNoTarget(id)),
    };
    let _ = reply.send(outcome.map(|report| record_report(&shared, report)));
}

async fn probe_through_session(
    session: &SshSession,
    target: &ForwardTarget,
) -> Result<TargetReport, EngineError> {
    let opening = timeout(
        OPEN_CHANNEL_TIMEOUT,
        session.open_direct_tcpip(&target.host, target.port, None),
    )
    .await;
    match opening {
        Err(_) => Ok(TargetReport::Unreachable(format!(
            "no answer within {OPEN_CHANNEL_TIMEOUT:?}"
        ))),
        Ok(Ok(mut channel)) => {
            let _ = channel.shutdown().await;
            Ok(TargetReport::Reachable)
        }
        Ok(Err(error)) => unreachable_or_failed(error),
    }
}

fn unreachable_or_failed(error: SessionError) -> Result<TargetReport, EngineError> {
    if is_target_unreachable(&error) {
        return Ok(TargetReport::Unreachable(error.to_string()));
    }
    Err(EngineError::TargetCheckFailed(error))
}

async fn probe_local_target(target: &ForwardTarget) -> TargetReport {
    match connect_to_target(target).await {
        Ok(_connection) => TargetReport::Reachable,
        Err(error) => TargetReport::Unreachable(error.to_string()),
    }
}

fn record_report(shared: &TunnelShared, report: TargetReport) -> TargetHealth {
    match report {
        TargetReport::Reachable => shared.set_health(TargetHealth::Reachable),
        TargetReport::Unreachable(error) => shared.observe_target_failure(error),
    }
    shared.status().health
}
