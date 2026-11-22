//! The engine side of the devslot discovery: running `ds ls --json` on a
//! host and removing the stale tunnels it reveals.

use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::blocking::run_blocking;
use crate::devslot::{
    BindProbe, DEFAULT_PROGRAM, DevslotDiscovery, DevslotError, DevslotInstance, DevslotResync,
    DevslotResyncTunnel, DevslotWarning, KnownTunnel, MAX_PATH_BYTES, assign_local_ports,
    assign_port_statuses, build_listing_command, find_stale_tunnels, parse_listing, plan_resync,
    sanitize_stderr, sanitize_text,
};
use crate::remote_command::{CommandOutput, RemoteCommandError, run_remote_command};
use crate::runtime::catalog::TunnelSnapshot;
use crate::runtime::engine::{EngineError, EngineInner};
use crate::runtime::events::EngineEvent;
use crate::runtime::session_manager::AcquireError;

const NOT_FOUND_EXIT_STATUS: u32 = 127;
const MILLIS_PER_SECOND: u128 = 1000;
const STDOUT_TOO_LARGE_REASON: &str = "the output is larger than 1 MiB";
const REFUSED_MESSAGE: &str = "the server refused to run the command";

/// What a host listed, read and cleaned, before it is compared with the
/// tunnels and the ports of this machine.
pub(crate) struct RemoteDevslotListing {
    instances: Vec<DevslotInstance>,
    warnings: Vec<DevslotWarning>,
    persist_override: Option<bool>,
}

/// A listing compared with the tunnels and the ports of this machine, and the
/// tunnels it was compared with.
pub(crate) struct DevslotAnalysis {
    pub(crate) discovery: DevslotDiscovery,
    pub(crate) known: Vec<KnownTunnel>,
}

impl EngineInner {
    pub(crate) async fn discover_devslot(
        &self,
        host: &str,
    ) -> Result<DevslotDiscovery, DevslotError> {
        let listing = self.list_devslot(host).await?;
        Ok(self.analyse_devslot(host, &listing).await.discovery)
    }

    /// Runs `ds ls --json` on the host and reads its output. Nothing here
    /// depends on the tunnels of this machine, so it never needs a lock.
    pub(crate) async fn list_devslot(
        &self,
        host: &str,
    ) -> Result<RemoteDevslotListing, DevslotError> {
        let overrides = self
            .store
            .document()
            .hosts
            .get(host)
            .cloned()
            .unwrap_or_default();
        let stdout = self
            .list_devslot_instances(host, overrides.devslot_path.as_deref())
            .await?;
        let parsed = parse_listing(&stdout).map_err(|error| DevslotError::InvalidOutput {
            host: host.to_owned(),
            reason: error.to_string(),
        })?;
        Ok(RemoteDevslotListing {
            instances: parsed.instances,
            warnings: parsed.warnings,
            persist_override: overrides.devslot_persist,
        })
    }

    /// Compares a listing with the tunnels and the free ports of this machine
    /// as they are now.
    pub(crate) async fn analyse_devslot(
        &self,
        host: &str,
        listing: &RemoteDevslotListing,
    ) -> DevslotAnalysis {
        let settings = self.settings.borrow().clone();
        let known = known_tunnels(self.catalog().snapshots());
        let discovery = DevslotDiscovery {
            host: host.to_owned(),
            persist_default: listing.persist_override.unwrap_or(settings.devslot_persist),
            stale: find_stale_tunnels(host, &listing.instances, &known, settings.devslot_stale),
            instances: self
                .assign_local_ports(host, listing.instances.clone(), known.clone())
                .await,
            warnings: listing.warnings.clone(),
        };
        DevslotAnalysis { discovery, known }
    }

    async fn assign_local_ports(
        &self,
        host: &str,
        instances: Vec<DevslotInstance>,
        known: Vec<KnownTunnel>,
    ) -> Vec<DevslotInstance> {
        let with_statuses = assign_port_statuses(host, instances, &known);
        run_blocking(move || assign_local_ports(with_statuses, &known, &BindProbe)).await
    }

    async fn list_devslot_instances(
        &self,
        host: &str,
        program: Option<&str>,
    ) -> Result<Vec<u8>, DevslotError> {
        let lease = self
            .sessions
            .acquire(host)
            .await
            .map_err(|error| acquisition_failure(host, error))?;
        let command = build_listing_command(program);
        let outcome =
            run_remote_command(&lease.session(), &command, self.remote_command_timeout).await;
        drop(lease);
        let output =
            outcome.map_err(|error| command_failure(host, error, self.remote_command_timeout))?;
        interpret_exit(host, program, output)
    }

    pub(crate) async fn resync_devslot(
        &self,
        host: &str,
        requested: Vec<DevslotResyncTunnel>,
    ) -> Result<DevslotResync, EngineError> {
        let _serialized = self.mutation.lock().await;
        if self.is_shut_down.load(Ordering::SeqCst) {
            return Err(EngineError::ShutDown);
        }
        let known = known_tunnels(self.catalog().snapshots());
        let plan = plan_resync(host, &requested, &known);
        self.delete_saved_tunnels(&plan.saved).await?;
        self.remove_temporary_tunnels(&plan.temporaries);
        if !plan.saved.is_empty() || !plan.temporaries.is_empty() {
            self.bus.publish(EngineEvent::ConfigReloaded);
        }
        Ok(DevslotResync {
            removed: plan.removed,
            skipped: plan.skipped,
        })
    }

    async fn delete_saved_tunnels(
        &self,
        ids: &[crate::store::TunnelId],
    ) -> Result<(), EngineError> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut tunnels = self.catalog().persistent_definitions();
        tunnels.retain(|definition| !ids.contains(&definition.id));
        self.persist_tunnels(tunnels).await?;
        let stopping: Vec<_> = {
            let mut catalog = self.catalog();
            ids.iter().filter_map(|id| catalog.remove(*id)).collect()
        };
        self.record_unwanted(ids);
        for task in stopping {
            let _ = task.await;
        }
        Ok(())
    }

    fn remove_temporary_tunnels(&self, ids: &[crate::store::TunnelId]) {
        let mut catalog = self.catalog();
        for id in ids {
            catalog.remove_temporary(*id);
        }
    }
}

pub(crate) fn known_tunnels(snapshots: Vec<TunnelSnapshot>) -> Vec<KnownTunnel> {
    snapshots
        .into_iter()
        .map(|snapshot| KnownTunnel {
            definition: snapshot.definition,
            is_temporary: snapshot.is_temporary,
            origin: snapshot.devslot,
        })
        .collect()
}

fn acquisition_failure(host: &str, error: AcquireError) -> DevslotError {
    match error {
        AcquireError::Connect(source) => DevslotError::Connect {
            host: host.to_owned(),
            source,
        },
        AcquireError::Aborted => DevslotError::ShuttingDown,
    }
}

fn command_failure(host: &str, error: RemoteCommandError, time_limit: Duration) -> DevslotError {
    match error {
        RemoteCommandError::Timeout(_) => DevslotError::Timeout {
            host: host.to_owned(),
            seconds: u64::try_from(time_limit.as_millis().div_ceil(MILLIS_PER_SECOND))
                .unwrap_or(u64::MAX),
        },
        RemoteCommandError::StdoutTooLarge => DevslotError::InvalidOutput {
            host: host.to_owned(),
            reason: STDOUT_TOO_LARGE_REASON.to_owned(),
        },
        RemoteCommandError::Refused => failure_without_status(host, REFUSED_MESSAGE),
        RemoteCommandError::Channel(source) => failure_without_status(host, &source.to_string()),
    }
}

fn failure_without_status(host: &str, message: &str) -> DevslotError {
    DevslotError::Failed {
        host: host.to_owned(),
        status: None,
        stderr: sanitize_stderr(message.as_bytes()),
    }
}

fn interpret_exit(
    host: &str,
    program: Option<&str>,
    output: CommandOutput,
) -> Result<Vec<u8>, DevslotError> {
    match output.exit_status {
        Some(0) => Ok(output.stdout),
        Some(NOT_FOUND_EXIT_STATUS) => Err(DevslotError::NotFound {
            host: host.to_owned(),
            path: sanitize_text(program.unwrap_or(DEFAULT_PROGRAM), MAX_PATH_BYTES),
        }),
        status => Err(DevslotError::Failed {
            host: host.to_owned(),
            status,
            stderr: sanitize_stderr(&output.stderr_tail),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(exit_status: Option<u32>) -> CommandOutput {
        CommandOutput {
            stdout: b"{}".to_vec(),
            stderr_tail: b"boom\n".to_vec(),
            exit_status,
        }
    }

    #[test]
    fn should_give_the_output_when_the_command_exits_with_zero() {
        assert_eq!(
            interpret_exit("vega", None, output(Some(0))).unwrap(),
            b"{}".to_vec()
        );
    }

    #[test]
    fn should_report_not_found_with_the_default_program_when_the_status_is_127() {
        assert!(matches!(
            interpret_exit("vega", None, output(Some(127))),
            Err(DevslotError::NotFound { path, .. }) if path == "ds"
        ));
    }

    #[test]
    fn should_report_not_found_with_the_configured_program_when_the_status_is_127() {
        assert!(matches!(
            interpret_exit("vega", Some("~/bin/ds"), output(Some(127))),
            Err(DevslotError::NotFound { path, .. }) if path == "~/bin/ds"
        ));
    }

    #[test]
    fn should_report_the_status_and_the_standard_error_when_the_command_fails() {
        assert!(matches!(
            interpret_exit("vega", None, output(Some(2))),
            Err(DevslotError::Failed { status: Some(2), stderr, .. }) if stderr == "boom"
        ));
    }

    #[test]
    fn should_report_a_failure_without_status_when_the_command_gave_none() {
        assert!(matches!(
            interpret_exit("vega", None, output(None)),
            Err(DevslotError::Failed { status: None, .. })
        ));
    }

    #[test]
    fn should_round_the_time_limit_up_to_whole_seconds_when_reporting_a_timeout() {
        let error = command_failure(
            "vega",
            RemoteCommandError::Timeout(Duration::from_millis(1500)),
            Duration::from_millis(1500),
        );

        assert!(matches!(error, DevslotError::Timeout { seconds: 2, .. }));
    }
}
