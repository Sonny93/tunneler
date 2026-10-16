use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior, interval_at, sleep};

use crate::error::ConnectError;
use crate::runtime::backoff::random_jitter_fraction;
use crate::runtime::definition_diff::{DefinitionChange, classify_change};
use crate::runtime::forward_run::ForwardRun;
use crate::runtime::local_forward::{
    ConnectionJob, ForwardTarget, TargetReport, bind_listener, forward_connection, probe_target,
};
use crate::runtime::log_buffer::LogLevel;
use crate::runtime::options::EngineOptions;
use crate::runtime::retry::{RetryDecision, decide_after_connect_error, reconnect_delay};
use crate::runtime::session_manager::{AcquireError, SessionLease, SessionManager};
use crate::runtime::shared::TunnelShared;
use crate::runtime::state::{FailureReason, TargetHealth, TunnelState};
use crate::session::SshSession;
use crate::store::{Settings, TunnelDefinition, TunnelKind};

const ACCEPT_ERROR_PAUSE: Duration = Duration::from_millis(50);
const FIRST_ATTEMPT: u32 = 1;

/// What the engine asks a tunnel to do.
#[derive(Debug)]
pub(crate) enum TunnelCommand {
    Start,
    Stop,
    Restart,
    UpdateDefinition(TunnelDefinition),
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Interrupt {
    Stop,
    Restart,
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exit {
    Idle,
    Terminate,
}

enum RunEnd {
    Interrupted(Interrupt),
    Failed,
}

enum Step {
    SessionLost,
    ConnectFailed(Arc<ConnectError>),
    Interrupted(Interrupt),
}

/// The engine's handle on a running tunnel task.
pub(crate) struct TunnelHandle {
    pub(crate) commands: mpsc::UnboundedSender<TunnelCommand>,
    pub(crate) shared: Arc<TunnelShared>,
    pub(crate) task: Option<JoinHandle<()>>,
}

/// What a tunnel task is built from.
pub(crate) struct TunnelContext {
    pub(crate) definition: TunnelDefinition,
    pub(crate) shared: Arc<TunnelShared>,
    pub(crate) sessions: SessionManager,
    pub(crate) settings: watch::Receiver<Settings>,
    pub(crate) probing: watch::Receiver<bool>,
    pub(crate) options: EngineOptions,
}

/// Starts the task of a tunnel. It begins stopped.
pub(crate) fn spawn_tunnel(context: TunnelContext) -> TunnelHandle {
    let (commands, receiver) = mpsc::unbounded_channel();
    let shared = Arc::clone(&context.shared);
    let actor = TunnelActor {
        context,
        commands: receiver,
    };
    TunnelHandle {
        commands,
        shared,
        task: Some(tokio::spawn(actor.run())),
    }
}

struct TunnelActor {
    context: TunnelContext,
    commands: mpsc::UnboundedReceiver<TunnelCommand>,
}

impl TunnelActor {
    async fn run(mut self) {
        while let Some(command) = self.commands.recv().await {
            if self.handle_idle_command(command).await == Exit::Terminate {
                return;
            }
        }
    }

    async fn handle_idle_command(&mut self, command: TunnelCommand) -> Exit {
        match command {
            TunnelCommand::Start | TunnelCommand::Restart => return self.run_until_stopped().await,
            TunnelCommand::Stop => self.context.shared.set_state(TunnelState::Stopped),
            TunnelCommand::UpdateDefinition(definition) => self.context.definition = definition,
            TunnelCommand::Terminate => return Exit::Terminate,
        }
        Exit::Idle
    }

    async fn run_until_stopped(&mut self) -> Exit {
        loop {
            match self.run_once().await {
                RunEnd::Failed => return Exit::Idle,
                RunEnd::Interrupted(Interrupt::Restart) => {}
                RunEnd::Interrupted(Interrupt::Stop) => {
                    self.context.shared.set_state(TunnelState::Stopped);
                    return Exit::Idle;
                }
                RunEnd::Interrupted(Interrupt::Terminate) => {
                    self.context.shared.set_state(TunnelState::Stopped);
                    return Exit::Terminate;
                }
            }
        }
    }

    async fn run_once(&mut self) -> RunEnd {
        let shared = Arc::clone(&self.context.shared);
        shared.counters().reset();
        shared.set_state(TunnelState::Starting);
        let mut run = match self.open_forward().await {
            Ok(run) => run,
            Err(reason) => return self.fail(reason),
        };
        let end = self.drive(&mut run).await;
        run.close_connections().await;
        if run.throttle.should_flush(Instant::now(), &shared.metrics()) {
            shared.publish_metrics(shared.metrics());
        }
        shared.set_health(TargetHealth::Unknown);
        end
    }

    async fn open_forward(&self) -> Result<ForwardRun, FailureReason> {
        let definition = &self.context.definition;
        if definition.kind != TunnelKind::Local {
            return Err(FailureReason::UnsupportedKind(definition.kind));
        }
        let settings = self.current_settings();
        let address = definition.effective_listen_address(&settings);
        let listener = bind_listener(address, definition.listen_port).await?;
        Ok(ForwardRun::new(
            listener,
            self.context.options.metrics_event_interval,
        ))
    }

    async fn drive(&mut self, run: &mut ForwardRun) -> RunEnd {
        let mut failed_attempts = 0_u32;
        loop {
            let (reason, delay) = match self.connect_and_serve(run).await {
                Step::Interrupted(interrupt) => return RunEnd::Interrupted(interrupt),
                Step::SessionLost => {
                    failed_attempts = FIRST_ATTEMPT;
                    let delay = self.next_delay(failed_attempts);
                    ("the SSH session was lost".to_owned(), delay)
                }
                Step::ConnectFailed(error) => {
                    failed_attempts += 1;
                    match self.decide_retry(&error, failed_attempts) {
                        RetryDecision::Fail(reason) => return self.fail(reason),
                        RetryDecision::Retry { delay } => (error.to_string(), delay),
                    }
                }
            };
            if let Some(interrupt) = self
                .wait_to_retry(run, failed_attempts, delay, &reason)
                .await
            {
                return RunEnd::Interrupted(interrupt);
            }
        }
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
        run: &mut ForwardRun,
        attempt: u32,
        delay: Duration,
        reason: &str,
    ) -> Option<Interrupt> {
        let shared = Arc::clone(&self.context.shared);
        shared.set_state(TunnelState::Reconnecting {
            attempt,
            next_retry_at: shared.now() + delay,
        });
        shared.log(
            LogLevel::Warn,
            format!("{reason}; reconnect attempt {attempt} in {delay:?}"),
        );
        self.idle_until(run, sleep(delay)).await.err()
    }

    async fn connect_and_serve(&mut self, run: &mut ForwardRun) -> Step {
        let sessions = self.context.sessions.clone();
        let host = self.context.definition.host.clone();
        let acquisition = self
            .idle_until(run, async move { sessions.acquire(&host).await })
            .await;
        match acquisition {
            Err(interrupt) => Step::Interrupted(interrupt),
            Ok(Ok(lease)) => self.serve(run, &lease).await,
            Ok(Err(AcquireError::Connect(error))) => Step::ConnectFailed(error),
            Ok(Err(AcquireError::Aborted)) => Step::Interrupted(Interrupt::Terminate),
        }
    }

    async fn idle_until<Output>(
        &mut self,
        run: &mut ForwardRun,
        wait: impl Future<Output = Output>,
    ) -> Result<Output, Interrupt> {
        tokio::pin!(wait);
        loop {
            tokio::select! {
                output = &mut wait => return Ok(output),
                accepted = run.listener.accept() => self.refuse_connection(accepted).await,
                command = self.commands.recv() => {
                    if let Some(interrupt) = self.interrupt_from(command) {
                        return Err(interrupt);
                    }
                }
            }
        }
    }

    async fn refuse_connection(&self, accepted: io::Result<(TcpStream, SocketAddr)>) {
        match accepted {
            Ok((_client, peer)) => self.context.shared.log(
                LogLevel::Warn,
                format!("connection from {peer} closed: no SSH session available"),
            ),
            Err(error) => self.report_accept_failure(&error).await,
        }
    }

    async fn report_accept_failure(&self, error: &io::Error) {
        self.context.shared.log(
            LogLevel::Warn,
            format!("cannot accept a connection: {error}"),
        );
        sleep(ACCEPT_ERROR_PAUSE).await;
    }

    async fn serve(&mut self, run: &mut ForwardRun, lease: &SessionLease) -> Step {
        let session = lease.session();
        let shared = Arc::clone(&self.context.shared);
        shared.set_state(TunnelState::Active);
        let mut probe_ticker = ticker(self.context.options.health_probe_interval);
        let mut metrics_ticker = ticker(self.context.options.metrics_event_interval);
        loop {
            tokio::select! {
                () = lease.lost() => {
                    run.close_connections().await;
                    return Step::SessionLost;
                }
                accepted = run.listener.accept() => self.accept_connection(run, &session, accepted).await,
                Some(_) = run.connections.join_next() => {}
                Some(_) = run.probes.join_next() => {}
                Some(report) = run.reports.recv() => self.apply_report(report),
                _ = probe_ticker.tick() => self.start_probe(run, &session),
                _ = metrics_ticker.tick() => self.publish_metrics(run),
                command = self.commands.recv() => {
                    if let Some(interrupt) = self.interrupt_from(command) {
                        return Step::Interrupted(interrupt);
                    }
                }
            }
        }
    }

    async fn accept_connection(
        &self,
        run: &mut ForwardRun,
        session: &Arc<SshSession>,
        accepted: io::Result<(TcpStream, SocketAddr)>,
    ) {
        let (client, peer) = match accepted {
            Ok(connection) => connection,
            Err(error) => return self.report_accept_failure(&error).await,
        };
        run.connections.spawn(forward_connection(ConnectionJob {
            client,
            peer,
            session: Arc::clone(session),
            target: self.forward_target(),
            shared: Arc::clone(&self.context.shared),
            reports: run.reports_sender.clone(),
        }));
    }

    fn apply_report(&self, report: TargetReport) {
        match report {
            TargetReport::Reachable => self.context.shared.set_health(TargetHealth::Reachable),
            TargetReport::Unreachable(error) => self.context.shared.observe_target_failure(error),
        }
    }

    fn start_probe(&self, run: &mut ForwardRun, session: &Arc<SshSession>) {
        if !self.is_probing_enabled() || !run.probes.is_empty() {
            return;
        }
        run.probes.spawn(probe_target(
            Arc::clone(session),
            self.forward_target(),
            run.reports_sender.clone(),
        ));
    }

    fn publish_metrics(&self, run: &mut ForwardRun) {
        let metrics = self.context.shared.metrics();
        if run.throttle.should_emit_at(Instant::now(), &metrics) {
            self.context.shared.publish_metrics(metrics);
        }
    }

    fn is_probing_enabled(&self) -> bool {
        *self.context.probing.borrow() && self.context.settings.borrow().health_probe
    }

    fn forward_target(&self) -> ForwardTarget {
        ForwardTarget {
            host: self.context.definition.effective_target_host().to_owned(),
            port: self.context.definition.effective_target_port(),
        }
    }

    fn current_settings(&self) -> Settings {
        self.context.settings.borrow().clone()
    }

    fn interrupt_from(&mut self, command: Option<TunnelCommand>) -> Option<Interrupt> {
        interrupt_for(&mut self.context.definition, command)
    }

    fn fail(&self, reason: FailureReason) -> RunEnd {
        self.context
            .shared
            .set_state(TunnelState::Failed { reason });
        RunEnd::Failed
    }
}

fn ticker(period: Duration) -> tokio::time::Interval {
    let mut interval = interval_at(Instant::now() + period, period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    interval
}

fn interrupt_for(
    definition: &mut TunnelDefinition,
    command: Option<TunnelCommand>,
) -> Option<Interrupt> {
    match command {
        None | Some(TunnelCommand::Terminate) => Some(Interrupt::Terminate),
        Some(TunnelCommand::Stop) => Some(Interrupt::Stop),
        Some(TunnelCommand::Restart) => Some(Interrupt::Restart),
        Some(TunnelCommand::Start) => None,
        Some(TunnelCommand::UpdateDefinition(updated)) => {
            let change = classify_change(definition, &updated);
            *definition = updated;
            (change == DefinitionChange::RequiresRestart).then_some(Interrupt::Restart)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::store::{TunnelId, TunnelKind};

    use super::*;

    fn definition() -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
            name: "Adonis".to_owned(),
            host: "nova".to_owned(),
            kind: TunnelKind::Local,
            listen_port: 3333,
            listen_address: None,
            target_host: None,
            target_port: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn should_ignore_start_when_the_tunnel_is_already_running() {
        let mut current = definition();

        assert_eq!(
            interrupt_for(&mut current, Some(TunnelCommand::Start)),
            None
        );
    }

    #[test]
    fn should_stop_when_asked_to_stop() {
        let mut current = definition();

        assert_eq!(
            interrupt_for(&mut current, Some(TunnelCommand::Stop)),
            Some(Interrupt::Stop)
        );
    }

    #[test]
    fn should_restart_when_asked_to_restart() {
        let mut current = definition();

        assert_eq!(
            interrupt_for(&mut current, Some(TunnelCommand::Restart)),
            Some(Interrupt::Restart)
        );
    }

    #[test]
    fn should_terminate_when_asked_to_terminate() {
        let mut current = definition();

        assert_eq!(
            interrupt_for(&mut current, Some(TunnelCommand::Terminate)),
            Some(Interrupt::Terminate)
        );
    }

    #[test]
    fn should_terminate_when_the_command_channel_closed() {
        let mut current = definition();

        assert_eq!(
            interrupt_for(&mut current, None),
            Some(Interrupt::Terminate)
        );
    }

    #[test]
    fn should_keep_running_when_only_the_name_changes() {
        let mut current = definition();
        let renamed = TunnelDefinition {
            name: "Renamed".to_owned(),
            ..current.clone()
        };

        let interrupt = interrupt_for(&mut current, Some(TunnelCommand::UpdateDefinition(renamed)));

        assert_eq!(interrupt, None);
    }

    #[test]
    fn should_adopt_the_new_name_when_only_the_name_changes() {
        let mut current = definition();
        let renamed = TunnelDefinition {
            name: "Renamed".to_owned(),
            ..current.clone()
        };

        interrupt_for(&mut current, Some(TunnelCommand::UpdateDefinition(renamed)));

        assert_eq!(current.name, "Renamed");
    }

    #[test]
    fn should_restart_when_the_listen_port_changes() {
        let mut current = definition();
        let moved = TunnelDefinition {
            listen_port: 4444,
            ..current.clone()
        };

        let interrupt = interrupt_for(&mut current, Some(TunnelCommand::UpdateDefinition(moved)));

        assert_eq!(interrupt, Some(Interrupt::Restart));
    }

    #[test]
    fn should_adopt_the_new_port_when_the_listen_port_changes() {
        let mut current = definition();
        let moved = TunnelDefinition {
            listen_port: 4444,
            ..current.clone()
        };

        interrupt_for(&mut current, Some(TunnelCommand::UpdateDefinition(moved)));

        assert_eq!(current.listen_port, 4444);
    }
}
