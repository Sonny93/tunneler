use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use russh::Channel;
use russh::client::Msg;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tracing::{error, info, warn};

use crate::clock::Clock;
use crate::error::ConnectError;
use crate::runtime::backoff::random_jitter_fraction;
use crate::runtime::events::{EngineEvent, EventBus};
use crate::runtime::retry::{RetryDecision, decide_after_connect_error, reconnect_delay};
use crate::runtime::session_manager::{AcquireError, SessionLease, SessionManager};
use crate::runtime::x11_state::X11AnchorState;
use crate::session::SshSession;
use crate::store::Settings;
use crate::x11::{
    ANCHOR_COMMAND, ChannelEvent, LocalX11Resolver, X11Error, next_channel_event,
    read_display_line, start_command,
};

const FIRST_ATTEMPT: u32 = 1;
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);

/// What the engine asks an anchor to do.
#[derive(Debug)]
pub(crate) enum AnchorCommand {
    Start,
    Stop,
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Interrupt {
    Stop,
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exit {
    Idle,
    Terminate,
}

enum Attempt {
    Interrupted(Interrupt),
    SessionLost,
    ConnectFailed(Arc<ConnectError>),
    Failed(String),
}

/// What an anchor publishes about itself: its state, which reaches the
/// subscribers as events and the log.
pub(crate) struct X11AnchorShared {
    host: String,
    bus: EventBus,
    state: Mutex<X11AnchorState>,
}

impl X11AnchorShared {
    pub(crate) fn new(host: String, bus: EventBus) -> Arc<Self> {
        Arc::new(Self {
            host,
            bus,
            state: Mutex::new(X11AnchorState::Stopped),
        })
    }

    pub(crate) fn state(&self) -> X11AnchorState {
        self.lock().clone()
    }

    pub(crate) fn set_state(&self, next: X11AnchorState) {
        let mut state = self.lock();
        if *state == next {
            return;
        }
        state.clone_from(&next);
        drop(state);
        self.log(&next);
        self.bus.publish(EngineEvent::X11Changed {
            host: self.host.clone(),
            state: next,
        });
    }

    fn lock(&self) -> MutexGuard<'_, X11AnchorState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn log(&self, state: &X11AnchorState) {
        let host = &self.host;
        match state {
            X11AnchorState::Failed { .. } => error!(%host, "x11 anchor: {state}"),
            X11AnchorState::Reconnecting { .. } => warn!(%host, "x11 anchor: {state}"),
            X11AnchorState::Stopped | X11AnchorState::Starting | X11AnchorState::Active { .. } => {
                info!(%host, "x11 anchor: {state}");
            }
        }
    }
}

/// The engine's handle on a running anchor task.
pub(crate) struct AnchorHandle {
    pub(crate) commands: mpsc::UnboundedSender<AnchorCommand>,
    pub(crate) shared: Arc<X11AnchorShared>,
    pub(crate) task: Option<JoinHandle<()>>,
}

/// What an anchor task is built from.
pub(crate) struct AnchorContext {
    pub(crate) host: String,
    pub(crate) shared: Arc<X11AnchorShared>,
    pub(crate) sessions: SessionManager,
    pub(crate) resolver: Arc<LocalX11Resolver>,
    pub(crate) settings: watch::Receiver<Settings>,
    pub(crate) clock: Arc<dyn Clock>,
}

/// Starts the task of an anchor. It begins stopped.
pub(crate) fn spawn_anchor(context: AnchorContext) -> AnchorHandle {
    let (commands, receiver) = mpsc::unbounded_channel();
    let shared = Arc::clone(&context.shared);
    let actor = AnchorActor {
        context,
        commands: receiver,
    };
    AnchorHandle {
        commands,
        shared,
        task: Some(tokio::spawn(actor.run())),
    }
}

struct AnchorActor {
    context: AnchorContext,
    commands: mpsc::UnboundedReceiver<AnchorCommand>,
}

impl AnchorActor {
    async fn run(mut self) {
        while let Some(command) = self.commands.recv().await {
            match command {
                AnchorCommand::Start => {
                    if self.run_until_stopped().await == Exit::Terminate {
                        return;
                    }
                }
                AnchorCommand::Stop => self.context.shared.set_state(X11AnchorState::Stopped),
                AnchorCommand::Terminate => return,
            }
        }
    }

    async fn run_until_stopped(&mut self) -> Exit {
        self.context.shared.set_state(X11AnchorState::Starting);
        let mut failed_attempts = 0_u32;
        loop {
            let (reason, delay) = match self.attempt().await {
                Attempt::Interrupted(interrupt) => return self.stop_after(interrupt),
                Attempt::Failed(message) => return self.fail(message),
                Attempt::SessionLost => {
                    failed_attempts = FIRST_ATTEMPT;
                    let delay = self.next_delay(failed_attempts);
                    ("the SSH session was lost".to_owned(), delay)
                }
                Attempt::ConnectFailed(error) => {
                    failed_attempts += 1;
                    match self.decide_retry(&error, failed_attempts) {
                        RetryDecision::Fail(reason) => return self.fail(reason.to_string()),
                        RetryDecision::Retry { delay } => (error.to_string(), delay),
                    }
                }
            };
            if let Some(interrupt) = self.wait_to_retry(failed_attempts, delay, &reason).await {
                return self.stop_after(interrupt);
            }
        }
    }

    fn stop_after(&self, interrupt: Interrupt) -> Exit {
        self.context.shared.set_state(X11AnchorState::Stopped);
        match interrupt {
            Interrupt::Stop => Exit::Idle,
            Interrupt::Terminate => Exit::Terminate,
        }
    }

    fn fail(&self, message: String) -> Exit {
        self.context
            .shared
            .set_state(X11AnchorState::Failed { message });
        Exit::Idle
    }

    fn current_settings(&self) -> Settings {
        self.context.settings.borrow().clone()
    }

    fn decide_retry(&self, error: &ConnectError, attempt: u32) -> RetryDecision {
        decide_after_connect_error(
            error,
            attempt,
            &self.current_settings(),
            random_jitter_fraction(),
        )
    }

    fn next_delay(&self, attempt: u32) -> Duration {
        reconnect_delay(attempt, &self.current_settings(), random_jitter_fraction())
    }

    async fn wait_to_retry(
        &mut self,
        attempt: u32,
        delay: Duration,
        reason: &str,
    ) -> Option<Interrupt> {
        self.context.shared.set_state(X11AnchorState::Reconnecting {
            attempt,
            next_retry_at: self.context.clock.now() + delay,
        });
        warn!(host = %self.context.host, "{reason}; x11 reconnect attempt {attempt} in {delay:?}");
        interruptible(&mut self.commands, sleep(delay)).await.err()
    }

    async fn attempt(&mut self) -> Attempt {
        let acquisition = interruptible(
            &mut self.commands,
            self.context.sessions.acquire(&self.context.host),
        )
        .await;
        match acquisition {
            Err(interrupt) => Attempt::Interrupted(interrupt),
            Ok(Ok(lease)) => self.serve(&lease).await,
            Ok(Err(AcquireError::Connect(error))) => Attempt::ConnectFailed(error),
            Ok(Err(AcquireError::Aborted)) => Attempt::Interrupted(Interrupt::Terminate),
        }
    }

    async fn serve(&mut self, lease: &SessionLease) -> Attempt {
        let session = lease.session();
        let mut channel = match self.open_channel(lease, &session).await {
            Ok(channel) => channel,
            Err(attempt) => return attempt,
        };
        let attempt = self.drive(lease, &mut channel).await;
        close_quietly(&channel).await;
        attempt
    }

    async fn open_channel(
        &mut self,
        lease: &SessionLease,
        session: &SshSession,
    ) -> Result<Channel<Msg>, Attempt> {
        let local = guarded(
            &mut self.commands,
            lease,
            self.context.resolver.resolve(&self.context.host),
        )
        .await?
        .map_err(|error| failure_attempt(&error))?;
        let forwarding = session
            .enable_x11(local)
            .map_err(|error| failure_attempt(&error))?;
        guarded(
            &mut self.commands,
            lease,
            session.open_x11_session(&forwarding),
        )
        .await?
        .map_err(|error| failure_attempt(&error))
    }

    async fn drive(&mut self, lease: &SessionLease, channel: &mut Channel<Msg>) -> Attempt {
        let display = match self.announce_display(lease, channel).await {
            Ok(display) => display,
            Err(attempt) => return attempt,
        };
        self.context
            .shared
            .set_state(X11AnchorState::Active { display });
        self.hold(lease, channel).await
    }

    async fn announce_display(
        &mut self,
        lease: &SessionLease,
        channel: &mut Channel<Msg>,
    ) -> Result<String, Attempt> {
        let announcing = async {
            start_command(channel, ANCHOR_COMMAND).await?;
            read_display_line(channel).await
        };
        guarded(&mut self.commands, lease, announcing)
            .await?
            .map_err(|error| failure_attempt(&error))
    }

    async fn hold(&mut self, lease: &SessionLease, channel: &mut Channel<Msg>) -> Attempt {
        loop {
            match guarded(&mut self.commands, lease, next_channel_event(channel)).await {
                Err(attempt) => return attempt,
                Ok(ChannelEvent::Closed) => {
                    warn!(host = %self.context.host, "the x11 anchor channel was closed");
                    return Attempt::SessionLost;
                }
                Ok(_) => {}
            }
        }
    }
}

async fn next_interrupt(commands: &mut mpsc::UnboundedReceiver<AnchorCommand>) -> Interrupt {
    loop {
        match commands.recv().await {
            Some(AnchorCommand::Stop) => return Interrupt::Stop,
            Some(AnchorCommand::Terminate) | None => return Interrupt::Terminate,
            Some(AnchorCommand::Start) => {}
        }
    }
}

async fn interruptible<Output>(
    commands: &mut mpsc::UnboundedReceiver<AnchorCommand>,
    work: impl Future<Output = Output>,
) -> Result<Output, Interrupt> {
    tokio::select! {
        interrupt = next_interrupt(commands) => Err(interrupt),
        output = work => Ok(output),
    }
}

async fn guarded<Output>(
    commands: &mut mpsc::UnboundedReceiver<AnchorCommand>,
    lease: &SessionLease,
    work: impl Future<Output = Output>,
) -> Result<Output, Attempt> {
    tokio::select! {
        interrupt = next_interrupt(commands) => Err(Attempt::Interrupted(interrupt)),
        () = lease.lost() => Err(Attempt::SessionLost),
        output = work => Ok(output),
    }
}

fn failure_attempt(error: &X11Error) -> Attempt {
    if error.is_session_loss() {
        warn!(%error, "the x11 anchor lost its channel");
        return Attempt::SessionLost;
    }
    Attempt::Failed(error.to_string())
}

async fn close_quietly(channel: &Channel<Msg>) {
    let _ = timeout(CLOSE_TIMEOUT, channel.close()).await;
}
