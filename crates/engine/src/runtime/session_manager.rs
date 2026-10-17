use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use thiserror::Error;
use tokio::runtime::Handle;
use tokio::sync::watch;
use tokio::task::AbortHandle;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::connector::Connector;
use crate::error::ConnectError;
use crate::session::SshSession;
use crate::store::Settings;

/// Why a session could not be leased.
#[derive(Debug, Clone, Error)]
pub(crate) enum AcquireError {
    /// Opening the session failed. The failure is shared with every caller
    /// that waited for the same attempt.
    #[error("{0}")]
    Connect(Arc<ConnectError>),
    /// The manager shut down, or the attempt died before it finished.
    #[error("the session manager is shutting down")]
    Aborted,
}

type ConnectOutcome = Result<Arc<LiveSession>, Arc<ConnectError>>;
type ConnectProgress = watch::Receiver<Option<ConnectOutcome>>;
type ConnectAttempt = Arc<watch::Sender<Option<ConnectOutcome>>>;

struct LiveSession {
    alias: String,
    session: Arc<SshSession>,
    idle_grace: Duration,
}

struct LiveSlot {
    live: Arc<LiveSession>,
    leases: usize,
    grace_timer: Option<AbortHandle>,
}

enum Slot {
    Connecting(ConnectAttempt),
    Live(LiveSlot),
}

#[derive(Default)]
struct Slots {
    by_alias: HashMap<String, Slot>,
    is_shut_down: bool,
}

struct ManagerInner {
    connector: Connector,
    settings: watch::Receiver<Settings>,
    runtime: Handle,
    slots: Mutex<Slots>,
}

enum Step {
    Leased(SessionLease),
    Wait(ConnectProgress),
}

/// Shares one SSH session per host alias between the tunnels that use it.
///
/// The session is opened when the first tunnel asks for it, concurrent
/// requests share a single connection attempt, and it is closed once the
/// last lease is gone and the idle grace period has passed. The grace period
/// is the one in force when the session was opened.
///
/// A connection attempt nobody waits for any more is abandoned: when every
/// caller that waited for it gave up, the attempt is dropped, and the next
/// request starts a new one.
///
/// Jump hosts belong to the session that runs through them: two targets
/// behind the same bastion each open their own connection to it.
#[derive(Clone)]
pub(crate) struct SessionManager {
    inner: Arc<ManagerInner>,
}

impl SessionManager {
    pub(crate) fn new(connector: Connector, settings: watch::Receiver<Settings>) -> Self {
        Self {
            inner: Arc::new(ManagerInner {
                connector,
                settings,
                runtime: Handle::current(),
                slots: Mutex::new(Slots::default()),
            }),
        }
    }

    /// Leases the session of `alias`, opening it when none is available.
    pub(crate) async fn acquire(&self, alias: &str) -> Result<SessionLease, AcquireError> {
        loop {
            match self.inner.lease_or_begin_connect(alias)? {
                Step::Leased(lease) => return Ok(lease),
                Step::Wait(progress) => {
                    let live = wait_for_outcome(progress).await?;
                    if let Some(lease) = self.inner.lease_live(&live) {
                        return Ok(lease);
                    }
                }
            }
        }
    }

    /// Closes every session and refuses further requests.
    pub(crate) async fn shutdown(&self) {
        let sessions = self.inner.drain_for_shutdown();
        for live in sessions {
            disconnect(&live).await;
        }
    }
}

async fn wait_for_outcome(mut progress: ConnectProgress) -> Result<Arc<LiveSession>, AcquireError> {
    let outcome = progress
        .wait_for(Option::is_some)
        .await
        .map_err(|_| AcquireError::Aborted)?
        .clone();
    match outcome {
        Some(Ok(live)) => Ok(live),
        Some(Err(error)) => Err(AcquireError::Connect(error)),
        None => Err(AcquireError::Aborted),
    }
}

fn remove_attempt_from(slots: &mut Slots, alias: &str, attempt: &ConnectAttempt) {
    let is_current = matches!(
        slots.by_alias.get(alias),
        Some(Slot::Connecting(current)) if Arc::ptr_eq(current, attempt)
    );
    if is_current {
        slots.by_alias.remove(alias);
    }
}

async fn disconnect(live: &LiveSession) {
    debug!(alias = %live.alias, "closing the session");
    if let Err(error) = live.session.disconnect().await {
        warn!(alias = %live.alias, %error, "cannot close the session cleanly");
    }
}

impl ManagerInner {
    fn lock(&self) -> MutexGuard<'_, Slots> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lease_or_begin_connect(self: &Arc<Self>, alias: &str) -> Result<Step, AcquireError> {
        let mut slots = self.lock();
        if slots.is_shut_down {
            return Err(AcquireError::Aborted);
        }
        match slots.by_alias.get_mut(alias) {
            Some(Slot::Live(slot)) if !slot.live.session.is_closed() => {
                slot.leases += 1;
                if let Some(timer) = slot.grace_timer.take() {
                    timer.abort();
                }
                Ok(Step::Leased(self.lease_for(&slot.live)))
            }
            Some(Slot::Connecting(attempt)) => Ok(Step::Wait(attempt.subscribe())),
            Some(Slot::Live(_)) | None => Ok(Step::Wait(self.begin_connect(&mut slots, alias))),
        }
    }

    fn begin_connect(self: &Arc<Self>, slots: &mut Slots, alias: &str) -> ConnectProgress {
        let (sender, progress) = watch::channel(None);
        let attempt = Arc::new(sender);
        if let Some(Slot::Live(stale)) = slots
            .by_alias
            .insert(alias.to_owned(), Slot::Connecting(Arc::clone(&attempt)))
        {
            stale.grace_timer.iter().for_each(AbortHandle::abort);
        }
        let inner = Arc::clone(self);
        let alias = alias.to_owned();
        self.runtime
            .spawn(async move { inner.connect_and_install(alias, attempt).await });
        progress
    }

    async fn connect_and_install(self: Arc<Self>, alias: String, attempt: ConnectAttempt) {
        let Some(connected) = self.connect_while_awaited(&alias, &attempt).await else {
            return;
        };
        match connected {
            Ok(session) => self.install(alias, session, &attempt).await,
            Err(error) => {
                self.remove_attempt(&alias, &attempt);
                attempt.send_replace(Some(Err(Arc::new(error))));
            }
        }
    }

    /// Connects, unless every caller that waited for the attempt gave up
    /// first: then the attempt is dropped and `None` is returned.
    async fn connect_while_awaited(
        &self,
        alias: &str,
        attempt: &ConnectAttempt,
    ) -> Option<Result<SshSession, ConnectError>> {
        let connecting = self.connector.connect(alias);
        tokio::pin!(connecting);
        loop {
            tokio::select! {
                connected = &mut connecting => return Some(connected),
                () = attempt.closed() => {
                    if self.abandon_if_unwatched(alias, attempt) {
                        debug!(%alias, "the connection attempt was abandoned: nobody waits for it");
                        return None;
                    }
                }
            }
        }
    }

    fn abandon_if_unwatched(&self, alias: &str, attempt: &ConnectAttempt) -> bool {
        let mut slots = self.lock();
        if attempt.receiver_count() > 0 {
            return false;
        }
        remove_attempt_from(&mut slots, alias, attempt);
        true
    }

    fn remove_attempt(&self, alias: &str, attempt: &ConnectAttempt) {
        remove_attempt_from(&mut self.lock(), alias, attempt);
    }

    async fn install(
        self: &Arc<Self>,
        alias: String,
        session: SshSession,
        attempt: &ConnectAttempt,
    ) {
        let live = Arc::new(LiveSession {
            alias: alias.clone(),
            session: Arc::new(session),
            idle_grace: self.settings.borrow().session_idle_grace,
        });
        if !self.register(&alias, &live) {
            disconnect(&live).await;
            return;
        }
        self.watch_for_loss(&live);
        attempt.send_replace(Some(Ok(live)));
    }

    fn register(self: &Arc<Self>, alias: &str, live: &Arc<LiveSession>) -> bool {
        let mut slots = self.lock();
        if slots.is_shut_down {
            return false;
        }
        slots.by_alias.insert(
            alias.to_owned(),
            Slot::Live(LiveSlot {
                live: Arc::clone(live),
                leases: 0,
                grace_timer: Some(self.start_grace_timer(live)),
            }),
        );
        true
    }

    fn lease_live(self: &Arc<Self>, live: &Arc<LiveSession>) -> Option<SessionLease> {
        let mut slots = self.lock();
        let Some(Slot::Live(slot)) = slots.by_alias.get_mut(&live.alias) else {
            return None;
        };
        if !Arc::ptr_eq(&slot.live, live) || live.session.is_closed() {
            return None;
        }
        slot.leases += 1;
        if let Some(timer) = slot.grace_timer.take() {
            timer.abort();
        }
        Some(self.lease_for(live))
    }

    fn lease_for(self: &Arc<Self>, live: &Arc<LiveSession>) -> SessionLease {
        SessionLease {
            live: Arc::clone(live),
            manager: Arc::clone(self),
        }
    }

    fn release(self: &Arc<Self>, live: &Arc<LiveSession>) {
        let mut slots = self.lock();
        let Some(Slot::Live(slot)) = slots.by_alias.get_mut(&live.alias) else {
            return;
        };
        if !Arc::ptr_eq(&slot.live, live) {
            return;
        }
        slot.leases = slot.leases.saturating_sub(1);
        if slot.leases == 0 {
            slot.grace_timer = Some(self.start_grace_timer(live));
        }
    }

    fn start_grace_timer(self: &Arc<Self>, live: &Arc<LiveSession>) -> AbortHandle {
        let inner = Arc::clone(self);
        let live = Arc::clone(live);
        self.runtime
            .spawn(async move {
                sleep(live.idle_grace).await;
                if inner.remove_if_unused(&live) {
                    disconnect(&live).await;
                }
            })
            .abort_handle()
    }

    fn remove_if_unused(&self, live: &Arc<LiveSession>) -> bool {
        let mut slots = self.lock();
        let is_unused = matches!(
            slots.by_alias.get(&live.alias),
            Some(Slot::Live(slot)) if Arc::ptr_eq(&slot.live, live) && slot.leases == 0
        );
        if is_unused {
            slots.by_alias.remove(&live.alias);
        }
        is_unused
    }

    fn watch_for_loss(self: &Arc<Self>, live: &Arc<LiveSession>) {
        let inner = Arc::clone(self);
        let live = Arc::clone(live);
        self.runtime.spawn(async move {
            live.session.closed().await;
            inner.forget(&live);
        });
    }

    fn forget(&self, live: &Arc<LiveSession>) {
        let mut slots = self.lock();
        let is_current = matches!(
            slots.by_alias.get(&live.alias),
            Some(Slot::Live(slot)) if Arc::ptr_eq(&slot.live, live)
        );
        if !is_current {
            return;
        }
        if let Some(Slot::Live(slot)) = slots.by_alias.remove(&live.alias) {
            slot.grace_timer.iter().for_each(AbortHandle::abort);
            debug!(alias = %live.alias, "the session was lost");
        }
    }

    fn drain_for_shutdown(&self) -> Vec<Arc<LiveSession>> {
        let mut slots = self.lock();
        slots.is_shut_down = true;
        slots
            .by_alias
            .drain()
            .filter_map(|(_, slot)| match slot {
                Slot::Live(slot) => {
                    slot.grace_timer.iter().for_each(AbortHandle::abort);
                    Some(slot.live)
                }
                Slot::Connecting(_) => None,
            })
            .collect()
    }
}

/// A claim on a session. The session stays open while a lease exists and
/// for the idle grace period after the last one is dropped.
pub(crate) struct SessionLease {
    live: Arc<LiveSession>,
    manager: Arc<ManagerInner>,
}

impl SessionLease {
    /// The leased session.
    pub(crate) fn session(&self) -> Arc<SshSession> {
        Arc::clone(&self.live.session)
    }

    /// Resolves once the session is lost, however it ended.
    pub(crate) async fn lost(&self) {
        self.live.session.closed().await;
    }
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        self.manager.release(&self.live);
    }
}
