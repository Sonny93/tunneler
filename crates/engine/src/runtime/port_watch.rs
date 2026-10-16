use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::{Instant, sleep};
use tracing::warn;

use crate::blocking::run_blocking;
use crate::external::{
    PortOccupation, PortScanner, ProcessIdentity, ScanError, SystemScan, UnmanagedProcess,
    occupation_of, unmanaged_processes,
};
use crate::runtime::engine::{EngineError, EngineInner};
use crate::runtime::events::EngineEvent;
use crate::runtime::state::TunnelState;
use crate::runtime::tunnel_actor::TunnelCommand;
use crate::store::{Settings, TunnelDefinition, TunnelId};

const RELEASE_POLL_INTERVAL: Duration = Duration::from_millis(100);
const RELEASE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Observed {
    latest: Option<SystemScan>,
    ignored: HashSet<ProcessIdentity>,
    externals: Vec<UnmanagedProcess>,
}

/// What the engine knows about the processes holding ports: the latest scan,
/// the processes the user chose to ignore and the unmanaged `ssh` processes.
pub(crate) struct PortWatch {
    scanner: Arc<dyn PortScanner>,
    pub(crate) own_pid: u32,
    observed: Mutex<Observed>,
    pub(crate) serial: tokio::sync::Mutex<()>,
}

impl PortWatch {
    pub(crate) fn new(scanner: Arc<dyn PortScanner>) -> Self {
        Self {
            scanner,
            own_pid: std::process::id(),
            observed: Mutex::new(Observed::default()),
            serial: tokio::sync::Mutex::new(()),
        }
    }

    pub(crate) async fn occupation_at_start(
        &self,
        definition: &TunnelDefinition,
        settings: &Settings,
    ) -> Result<Option<PortOccupation>, ScanError> {
        let scan = self.scan().await?;
        Ok(occupation_of(
            definition,
            settings,
            &scan,
            &HashSet::new(),
            self.own_pid,
        ))
    }

    pub(crate) fn externals(&self) -> Vec<UnmanagedProcess> {
        self.observed().externals.clone()
    }

    async fn scan(&self) -> Result<SystemScan, ScanError> {
        let scanner = Arc::clone(&self.scanner);
        let scan = run_blocking(move || scanner.scan()).await?;
        self.observed().latest = Some(scan.clone());
        Ok(scan)
    }

    async fn terminate(&self, identity: ProcessIdentity) -> Result<(), EngineError> {
        let scanner = Arc::clone(&self.scanner);
        run_blocking(move || scanner.terminate(identity)).await?;
        Ok(())
    }

    async fn wait_for_release(&self, pid: u32) -> Result<(), EngineError> {
        let deadline = Instant::now() + RELEASE_TIMEOUT;
        loop {
            let scan = self.scan().await?;
            if scan.listeners.iter().all(|socket| socket.pid != pid) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(EngineError::ProcessStillRunning(pid));
            }
            sleep(RELEASE_POLL_INTERVAL).await;
        }
    }

    fn identity_of(&self, pid: u32) -> Option<ProcessIdentity> {
        let observed = self.observed();
        let latest = observed.latest.as_ref()?;
        latest.process(pid).map(|process| process.identity)
    }

    fn ignore(&self, identity: ProcessIdentity) {
        self.observed().ignored.insert(identity);
    }

    fn ignored(&self) -> HashSet<ProcessIdentity> {
        self.observed().ignored.clone()
    }

    fn replace_externals(&self, next: Vec<UnmanagedProcess>) -> bool {
        let mut observed = self.observed();
        if observed.externals == next {
            return false;
        }
        observed.externals = next;
        true
    }

    fn observed(&self) -> MutexGuard<'_, Observed> {
        self.observed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Scans the listening ports while scanning is on, until the engine is gone.
pub(crate) async fn run_scan_loop(
    engine: Weak<EngineInner>,
    mut scanning: watch::Receiver<bool>,
    interval: Duration,
) {
    loop {
        if !*scanning.borrow_and_update() {
            if scanning.changed().await.is_err() {
                return;
            }
            continue;
        }
        let Some(inner) = engine.upgrade() else {
            return;
        };
        inner.run_pass().await;
        drop(inner);
        tokio::select! {
            () = sleep(interval) => {}
            changed = scanning.changed() => {
                if changed.is_err() {
                    return;
                }
            }
        }
    }
}

impl EngineInner {
    pub(crate) async fn run_pass(&self) {
        let _serialized = self.port_watch.serial.lock().await;
        self.run_pass_serialized().await;
    }

    async fn run_pass_serialized(&self) {
        match self.port_watch.scan().await {
            Ok(scan) => self.apply_scan(&scan),
            Err(error) => warn!(%error, "cannot scan the listening ports"),
        }
    }

    fn apply_scan(&self, scan: &SystemScan) {
        let settings = self.settings.borrow().clone();
        let ignored = self.port_watch.ignored();
        let own_pid = self.port_watch.own_pid;
        let definitions = {
            let catalog = self.catalog();
            catalog.observe_ports(&settings, scan, &ignored, own_pid);
            catalog.definitions()
        };
        let externals = unmanaged_processes(&definitions, &settings, scan, &ignored, own_pid);
        if self.port_watch.replace_externals(externals.clone()) {
            self.bus.publish(EngineEvent::ExternalsChanged {
                processes: externals,
            });
        }
    }

    pub(crate) async fn adopt_external(&self, id: TunnelId) -> Result<Vec<TunnelId>, EngineError> {
        let _serialized = self.port_watch.serial.lock().await;
        let pid = self.external_owner_pid(id)?;
        let adopted = self.tunnels_provided_by(pid);
        self.end_process(pid).await?;
        let catalog = self.catalog();
        for adopted_id in &adopted {
            catalog.send(*adopted_id, TunnelCommand::Start);
        }
        Ok(adopted)
    }

    pub(crate) async fn kill_external(&self, pid: u32) -> Result<(), EngineError> {
        let _serialized = self.port_watch.serial.lock().await;
        self.ensure_reported(pid)?;
        self.end_process(pid).await
    }

    pub(crate) async fn ignore_external(&self, pid: u32) -> Result<(), EngineError> {
        let _serialized = self.port_watch.serial.lock().await;
        self.ensure_reported(pid)?;
        let identity = self
            .port_watch
            .identity_of(pid)
            .ok_or(EngineError::UnknownProcess(pid))?;
        self.port_watch.ignore(identity);
        self.run_pass_serialized().await;
        Ok(())
    }

    async fn end_process(&self, pid: u32) -> Result<(), EngineError> {
        let identity = self
            .port_watch
            .identity_of(pid)
            .ok_or(EngineError::UnknownProcess(pid))?;
        self.port_watch.terminate(identity).await?;
        self.port_watch.wait_for_release(pid).await?;
        self.run_pass_serialized().await;
        Ok(())
    }

    fn external_owner_pid(&self, id: TunnelId) -> Result<u32, EngineError> {
        let snapshots = self.catalog().snapshots();
        let snapshot = snapshots
            .iter()
            .find(|snapshot| snapshot.definition.id == id)
            .ok_or(EngineError::UnknownTunnel(id))?;
        match &snapshot.state {
            TunnelState::External { owner } => Ok(owner.pid),
            _ => Err(EngineError::NotExternal(id)),
        }
    }

    fn tunnels_provided_by(&self, pid: u32) -> Vec<TunnelId> {
        self.catalog()
            .snapshots()
            .into_iter()
            .filter(|snapshot| {
                matches!(&snapshot.state, TunnelState::External { owner } if owner.pid == pid)
            })
            .map(|snapshot| snapshot.definition.id)
            .collect()
    }

    fn ensure_reported(&self, pid: u32) -> Result<(), EngineError> {
        let is_held = self.catalog().snapshots().iter().any(|snapshot| {
            matches!(
                &snapshot.state,
                TunnelState::External { owner }
                    | TunnelState::Conflict { owner: Some(owner) } if owner.pid == pid
            )
        });
        let is_unmanaged = self
            .port_watch
            .externals()
            .iter()
            .any(|process| process.pid == pid);
        if is_held || is_unmanaged {
            return Ok(());
        }
        Err(EngineError::UnknownProcess(pid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct IdleScanner;

    impl PortScanner for IdleScanner {
        fn scan(&self) -> Result<SystemScan, ScanError> {
            Ok(SystemScan::default())
        }

        fn terminate(
            &self,
            _process: ProcessIdentity,
        ) -> Result<(), crate::external::TerminateError> {
            Ok(())
        }
    }

    fn unmanaged(pid: u32) -> UnmanagedProcess {
        UnmanagedProcess {
            pid,
            process: "ssh".to_owned(),
            destination: None,
            forwards: Vec::new(),
        }
    }

    fn watch() -> PortWatch {
        PortWatch::new(Arc::new(IdleScanner))
    }

    #[test]
    fn should_report_a_change_when_the_unmanaged_processes_differ() {
        assert!(watch().replace_externals(vec![unmanaged(1)]));
    }

    #[test]
    fn should_report_no_change_when_the_unmanaged_processes_are_the_same() {
        let watch = watch();
        watch.replace_externals(vec![unmanaged(1)]);

        assert!(!watch.replace_externals(vec![unmanaged(1)]));
    }

    #[test]
    fn should_know_no_identity_when_nothing_was_scanned() {
        assert_eq!(watch().identity_of(1), None);
    }

    #[tokio::test]
    async fn should_find_no_holder_when_the_scan_is_empty() {
        let occupation = watch()
            .occupation_at_start(
                &crate::store::test_support::tunnel(crate::store::TunnelKind::Local),
                &Settings::default(),
            )
            .await;

        assert_eq!(occupation.unwrap(), None);
    }
}
