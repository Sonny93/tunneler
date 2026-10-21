use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior, interval_at, sleep, timeout};

use crate::error::{ConnectError, SessionError};
use crate::external::PortOccupation;
use crate::remote_forwards::{ForwardedConnection, RemoteForwardRegistration};
use crate::runtime::backoff::random_jitter_fraction;
use crate::runtime::definition_diff::{DefinitionChange, classify_change};
use crate::runtime::dynamic_forward::{SocksJob, socks_connection};
use crate::runtime::engine::EngineError;
use crate::runtime::forward_run::{ClientProtocol, ForwardRun, Inbound};
use crate::runtime::local_forward::{
    ConnectionJob, ForwardTarget, TargetReport, bind_listener, format_socket_address,
    forward_connection,
};
use crate::runtime::log_buffer::LogLevel;
use crate::runtime::options::EngineOptions;
use crate::runtime::port_watch::PortWatch;
use crate::runtime::remote_forward::{ForwardedJob, serve_forwarded_connection};
use crate::runtime::retry::{RetryDecision, decide_after_connect_error, reconnect_delay};
use crate::runtime::session_manager::{AcquireError, SessionLease, SessionManager};
use crate::runtime::shared::TunnelShared;
use crate::runtime::state::{FailureReason, TargetHealth, TunnelState};
use crate::runtime::target_check::{TargetCheckJob, TargetCheckReply, check_target};
use crate::session::SshSession;
use crate::store::{Settings, TunnelDefinition, TunnelKind};

const ACCEPT_ERROR_PAUSE: Duration = Duration::from_millis(50);
const FIRST_ATTEMPT: u32 = 1;
const CANCEL_FORWARD_TIMEOUT: Duration = Duration::from_secs(5);

/// What the engine asks a tunnel to do.
#[derive(Debug)]
pub(crate) enum TunnelCommand {
    Start,
    Stop,
    Restart,
    UpdateDefinition(TunnelDefinition),
    PortObserved(Option<PortOccupation>),
    CheckTarget(TargetCheckReply),
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
    Refused(FailureReason),
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
    pub(crate) options: EngineOptions,
    pub(crate) port_watch: Arc<PortWatch>,
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
            TunnelCommand::Stop => self.stop_when_idle(),
            TunnelCommand::UpdateDefinition(definition) => self.context.definition = definition,
            TunnelCommand::PortObserved(occupation) => self.apply_observation(occupation),
            TunnelCommand::CheckTarget(reply) => {
                let _ = reply.send(Err(EngineError::TunnelNotActive(
                    self.context.definition.id,
                )));
            }
            TunnelCommand::Terminate => return Exit::Terminate,
        }
        Exit::Idle
    }

    fn stop_when_idle(&self) {
        let status = self.context.shared.status();
        if !is_observation(&status.state) {
            self.context.shared.set_state(TunnelState::Stopped);
        }
    }

    fn apply_observation(&self, occupation: Option<PortOccupation>) {
        let current = self.context.shared.status().state;
        if let Some(next) = state_after_observation(&current, occupation) {
            self.context.shared.set_state(next);
        }
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
            Err(reason) => return self.fail_to_open(reason).await,
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
        let interval = self.context.options.metrics_event_interval;
        match self.context.definition.kind {
            TunnelKind::Local => Ok(ForwardRun::local(self.bind().await?, interval)),
            TunnelKind::Dynamic => Ok(ForwardRun::dynamic(self.bind().await?, interval)),
            TunnelKind::Remote => Ok(ForwardRun::remote(interval)),
        }
    }

    async fn bind(&self) -> Result<TcpListener, FailureReason> {
        let definition = &self.context.definition;
        let settings = self.current_settings();
        let address = definition.effective_listen_address(&settings);
        bind_listener(address, definition.listen_port).await
    }

    async fn fail_to_open(&self, reason: FailureReason) -> RunEnd {
        if matches!(reason, FailureReason::PortInUse { .. }) {
            return self.report_port_in_use(reason).await;
        }
        self.fail(reason)
    }

    async fn report_port_in_use(&self, reason: FailureReason) -> RunEnd {
        let settings = self.current_settings();
        let occupation = self
            .context
            .port_watch
            .occupation_at_start(&self.context.definition, &settings)
            .await;
        match occupation {
            Ok(Some(PortOccupation::External(owner))) => {
                self.context
                    .shared
                    .set_state(TunnelState::External { owner });
            }
            Ok(Some(PortOccupation::Conflict(owner))) => {
                self.context
                    .shared
                    .set_state(TunnelState::Conflict { owner });
            }
            Ok(None) => self
                .context
                .shared
                .set_state(TunnelState::Conflict { owner: None }),
            Err(error) => {
                self.context.shared.log(
                    LogLevel::Warn,
                    format!("cannot find out who holds the port: {error}"),
                );
                return self.fail(reason);
            }
        }
        RunEnd::Failed
    }

    async fn drive(&mut self, run: &mut ForwardRun) -> RunEnd {
        let mut failed_attempts = 0_u32;
        loop {
            let (reason, delay) = match self.connect_and_serve(run).await {
                Step::Interrupted(interrupt) => return RunEnd::Interrupted(interrupt),
                Step::Refused(reason) => return self.fail(reason),
                Step::SessionLost => {
                    failed_attempts = FIRST_ATTEMPT;
                    let delay = self.next_delay(failed_attempts);
                    ("the SSH session was lost".to_owned(), delay)
                }
                Step::ConnectFailed(error) => {
                    failed_attempts += 1;
                    match self.decide_retry(&error, failed_attempts) {
                        RetryDecision::Fail(reason) => return self.fail_to_connect(reason),
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
                inbound = run.inbound.next() => self.refuse_inbound(inbound).await,
                command = self.commands.recv() => {
                    if let Some(interrupt) = self.interrupt_from(command) {
                        return Err(interrupt);
                    }
                }
            }
        }
    }

    async fn refuse_inbound(&self, inbound: Inbound) {
        match inbound {
            Inbound::Client { accepted, .. } => self.refuse_connection(accepted).await,
            Inbound::Forwarded(connection) => self.context.shared.log(
                LogLevel::Warn,
                format!(
                    "connection from {} closed: no SSH session available",
                    connection.originator
                ),
            ),
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
        let registration = match self.begin_forwarding(run, &session).await {
            Ok(registration) => registration,
            Err(step) => return step,
        };
        let step = self.serve_active(run, lease, &session).await;
        self.end_forwarding(&session, registration, &step).await;
        step
    }

    async fn serve_active(
        &mut self,
        run: &mut ForwardRun,
        lease: &SessionLease,
        session: &Arc<SshSession>,
    ) -> Step {
        let shared = Arc::clone(&self.context.shared);
        shared.set_state(TunnelState::Active);
        let mut metrics_ticker = ticker(self.context.options.metrics_event_interval);
        loop {
            tokio::select! {
                () = lease.lost() => {
                    run.close_connections().await;
                    return Step::SessionLost;
                }
                inbound = run.inbound.next() => self.accept_inbound(run, session, inbound).await,
                Some(_) = run.connections.join_next() => {}
                Some(report) = run.reports.recv() => self.apply_report(report),
                _ = metrics_ticker.tick() => self.publish_metrics(run),
                command = self.commands.recv() => {
                    if let Some(interrupt) = self.handle_active_command(run, session, command) {
                        return Step::Interrupted(interrupt);
                    }
                }
            }
        }
    }

    fn handle_active_command(
        &mut self,
        run: &mut ForwardRun,
        session: &Arc<SshSession>,
        command: Option<TunnelCommand>,
    ) -> Option<Interrupt> {
        let Some(TunnelCommand::CheckTarget(reply)) = command else {
            return self.interrupt_from(command);
        };
        run.connections.spawn(check_target(TargetCheckJob {
            id: self.context.definition.id,
            kind: self.context.definition.kind,
            session: Arc::clone(session),
            target: self.forward_target(),
            shared: Arc::clone(&self.context.shared),
            reply,
        }));
        None
    }

    async fn begin_forwarding(
        &mut self,
        run: &mut ForwardRun,
        session: &Arc<SshSession>,
    ) -> Result<Option<RemoteForwardRegistration>, Step> {
        let Some(sender) = run.inbound.forwarded_sender() else {
            return Ok(None);
        };
        let settings = self.current_settings();
        let address = self
            .context
            .definition
            .effective_listen_address(&settings)
            .to_owned();
        let port = self.context.definition.listen_port;
        let request = session.request_remote_forward(&address, port, sender);
        match self.idle_until(run, request).await {
            Err(interrupt) => Err(Step::Interrupted(interrupt)),
            Ok(Ok(registration)) => Ok(Some(registration)),
            Ok(Err(SessionError::ForwardRefused { address, port })) => {
                Err(Step::Refused(FailureReason::RemoteForwardRefused {
                    address: format_socket_address(&address, port),
                }))
            }
            Ok(Err(error)) => {
                self.context.shared.log(
                    LogLevel::Warn,
                    format!("cannot ask the server to listen: {error}"),
                );
                Err(Step::SessionLost)
            }
        }
    }

    async fn end_forwarding(
        &self,
        session: &SshSession,
        registration: Option<RemoteForwardRegistration>,
        step: &Step,
    ) {
        let Some(registration) = registration else {
            return;
        };
        if matches!(step, Step::SessionLost) {
            return;
        }
        let cancellation = timeout(
            CANCEL_FORWARD_TIMEOUT,
            session.cancel_remote_forward(registration),
        )
        .await;
        match cancellation {
            Ok(Ok(())) => {}
            Ok(Err(error)) => self.context.shared.log(
                LogLevel::Warn,
                format!("cannot ask the server to stop listening: {error}"),
            ),
            Err(_) => self.context.shared.log(
                LogLevel::Warn,
                "the server did not confirm that it stopped listening".to_owned(),
            ),
        }
    }

    async fn accept_inbound(
        &self,
        run: &mut ForwardRun,
        session: &Arc<SshSession>,
        inbound: Inbound,
    ) {
        match inbound {
            Inbound::Client {
                accepted: Err(error),
                ..
            } => self.report_accept_failure(&error).await,
            Inbound::Client {
                accepted: Ok((client, peer)),
                protocol,
            } => self.serve_client(run, session, client, peer, protocol),
            Inbound::Forwarded(connection) => self.serve_forwarded(run, connection),
        }
    }

    fn serve_client(
        &self,
        run: &mut ForwardRun,
        session: &Arc<SshSession>,
        client: TcpStream,
        peer: SocketAddr,
        protocol: ClientProtocol,
    ) {
        let session = Arc::clone(session);
        let shared = Arc::clone(&self.context.shared);
        match protocol {
            ClientProtocol::Forward => run.connections.spawn(forward_connection(ConnectionJob {
                client,
                peer,
                session,
                target: self.forward_target(),
                shared,
                reports: run.reports_sender.clone(),
            })),
            ClientProtocol::Socks => run.connections.spawn(socks_connection(SocksJob {
                client,
                peer,
                session,
                shared,
            })),
        };
    }

    fn serve_forwarded(&self, run: &mut ForwardRun, connection: ForwardedConnection) {
        run.connections
            .spawn(serve_forwarded_connection(ForwardedJob {
                connection,
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

    fn publish_metrics(&self, run: &mut ForwardRun) {
        let metrics = self.context.shared.metrics();
        if run.throttle.should_emit_at(Instant::now(), &metrics) {
            self.context.shared.publish_metrics(metrics);
        }
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

    fn fail_to_connect(&self, reason: FailureReason) -> RunEnd {
        self.context
            .shared
            .log(LogLevel::Error, format!("cannot connect: {reason}"));
        self.fail(reason)
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

fn is_observation(state: &TunnelState) -> bool {
    matches!(
        state,
        TunnelState::External { .. } | TunnelState::Conflict { .. }
    )
}

fn is_port_in_use_failure(state: &TunnelState) -> bool {
    matches!(
        state,
        TunnelState::Failed {
            reason: FailureReason::PortInUse { .. }
        }
    )
}

fn state_after_observation(
    current: &TunnelState,
    occupation: Option<PortOccupation>,
) -> Option<TunnelState> {
    let is_replaceable = matches!(current, TunnelState::Stopped)
        || is_observation(current)
        || is_port_in_use_failure(current);
    if !is_replaceable {
        return None;
    }
    Some(match occupation {
        Some(PortOccupation::External(owner)) => TunnelState::External { owner },
        Some(PortOccupation::Conflict(owner)) => TunnelState::Conflict { owner },
        None => TunnelState::Stopped,
    })
}

fn interrupt_for(
    definition: &mut TunnelDefinition,
    command: Option<TunnelCommand>,
) -> Option<Interrupt> {
    match command {
        None | Some(TunnelCommand::Terminate) => Some(Interrupt::Terminate),
        Some(TunnelCommand::Stop) => Some(Interrupt::Stop),
        Some(TunnelCommand::Restart) => Some(Interrupt::Restart),
        Some(TunnelCommand::Start | TunnelCommand::PortObserved(_)) => None,
        Some(TunnelCommand::CheckTarget(reply)) => {
            let _ = reply.send(Err(EngineError::TunnelNotActive(definition.id)));
            None
        }
        Some(TunnelCommand::UpdateDefinition(updated)) => {
            let change = classify_change(definition, &updated);
            *definition = updated;
            (change == DefinitionChange::RequiresRestart).then_some(Interrupt::Restart)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::external::PortOwner;
    use crate::store::{TunnelId, TunnelKind};

    use super::*;

    fn owner() -> PortOwner {
        PortOwner {
            pid: 77,
            process: "ssh".to_owned(),
        }
    }

    fn port_in_use() -> TunnelState {
        TunnelState::Failed {
            reason: FailureReason::PortInUse {
                address: "127.0.0.1:3333".to_owned(),
            },
        }
    }

    fn other_failure() -> TunnelState {
        TunnelState::Failed {
            reason: FailureReason::Connect {
                message: "authentication failed".to_owned(),
            },
        }
    }

    #[test]
    fn should_become_external_when_a_stopped_tunnel_sees_an_external_owner() {
        assert_eq!(
            state_after_observation(
                &TunnelState::Stopped,
                Some(PortOccupation::External(owner()))
            ),
            Some(TunnelState::External { owner: owner() })
        );
    }

    #[test]
    fn should_become_conflict_when_a_stopped_tunnel_sees_another_holder() {
        assert_eq!(
            state_after_observation(
                &TunnelState::Stopped,
                Some(PortOccupation::Conflict(Some(owner())))
            ),
            Some(TunnelState::Conflict {
                owner: Some(owner())
            })
        );
    }

    #[test]
    fn should_become_conflict_when_a_port_in_use_failure_sees_another_holder() {
        assert_eq!(
            state_after_observation(&port_in_use(), Some(PortOccupation::Conflict(None))),
            Some(TunnelState::Conflict { owner: None })
        );
    }

    #[test]
    fn should_become_stopped_when_an_external_tunnel_sees_no_holder() {
        assert_eq!(
            state_after_observation(&TunnelState::External { owner: owner() }, None),
            Some(TunnelState::Stopped)
        );
    }

    #[test]
    fn should_become_stopped_when_a_conflict_sees_no_holder() {
        assert_eq!(
            state_after_observation(&TunnelState::Conflict { owner: None }, None),
            Some(TunnelState::Stopped)
        );
    }

    #[test]
    fn should_become_stopped_when_a_port_in_use_failure_sees_no_holder() {
        assert_eq!(
            state_after_observation(&port_in_use(), None),
            Some(TunnelState::Stopped)
        );
    }

    #[test]
    fn should_keep_the_failure_when_it_is_not_about_the_port() {
        assert_eq!(
            state_after_observation(&other_failure(), Some(PortOccupation::Conflict(None))),
            None
        );
    }

    #[test]
    fn should_keep_the_failure_when_it_is_not_about_the_port_and_nothing_holds_it() {
        assert_eq!(state_after_observation(&other_failure(), None), None);
    }

    #[test]
    fn should_keep_running_states_when_a_holder_is_observed() {
        assert_eq!(
            state_after_observation(&TunnelState::Active, Some(PortOccupation::Conflict(None))),
            None
        );
    }

    #[test]
    fn should_treat_external_and_conflict_as_observations() {
        assert!(
            is_observation(&TunnelState::External { owner: owner() })
                && is_observation(&TunnelState::Conflict { owner: None })
                && !is_observation(&TunnelState::Stopped)
        );
    }

    #[test]
    fn should_ignore_an_observation_when_the_tunnel_runs() {
        let mut current = definition();

        assert_eq!(
            interrupt_for(&mut current, Some(TunnelCommand::PortObserved(None))),
            None
        );
    }

    fn definition() -> TunnelDefinition {
        TunnelDefinition {
            id: TunnelId::generate(),
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
    fn should_keep_running_when_only_the_tags_change() {
        let mut current = definition();
        let renamed = TunnelDefinition {
            tags: vec!["dev".to_owned()],
            ..current.clone()
        };

        let interrupt = interrupt_for(&mut current, Some(TunnelCommand::UpdateDefinition(renamed)));

        assert_eq!(interrupt, None);
    }

    #[test]
    fn should_adopt_the_new_tags_when_only_the_tags_change() {
        let mut current = definition();
        let renamed = TunnelDefinition {
            tags: vec!["dev".to_owned()],
            ..current.clone()
        };

        interrupt_for(&mut current, Some(TunnelCommand::UpdateDefinition(renamed)));

        assert_eq!(current.tags, vec!["dev".to_owned()]);
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
