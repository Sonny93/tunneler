use std::sync::Arc;
use std::time::Duration;

use ipc::protocol::{EventMessage, HostKeyRequestEvent, SnapshotDto};
use ipc::{EventReceiver, IpcClient};
use tokio::time::Instant;

use super::super::args::UpArguments;
use super::super::launcher::ensure_daemon_running;
use super::super::progress::{ProgressTracker, describe_step};
use super::super::prompting::{LocalPrompts, describe_host_key_alert};
use super::super::session::{ConnectionMode, connect};
use super::super::style::Style;
use super::super::target::{ResolvedTunnel, TargetInterpretation, resolve_targets};
use super::super::terminal::{StandardTerminal, Terminal, is_interactive};
use super::super::{CliError, Context};
use crate::logging;

/// `tunneler up`: starts tunnels, starting the daemon first when needed, and
/// reports each step until every tunnel is active or has failed.
pub async fn up(context: &Context, arguments: UpArguments) -> Result<(), CliError> {
    logging::init_cli_logging();
    ensure_daemon_running(&context.endpoint, &context.locations).await?;
    let will_wait = !arguments.no_wait;
    let mode = ConnectionMode {
        subscribes_to_events: will_wait,
        can_prompt: will_wait && is_interactive(),
    };
    let mut client = connect(&context.endpoint, mode).await?;
    let events = client.take_events();
    let client = Arc::new(client);
    let snapshot = client.snapshot().await?;
    let targets = resolve(&arguments, &snapshot, context.style)?;
    let tracker = ProgressTracker::new(&targets, &snapshot);
    for label in tracker.already_active() {
        println!("{label}: already active");
    }
    for label in tracker.already_provided() {
        println!("{label}: already provided by an external process");
    }
    request_starts(&client, &targets).await?;
    let Some(events) = events.filter(|_| will_wait) else {
        for target in &targets {
            println!("{}: start requested", target.label);
        }
        return Ok(());
    };
    let prompts = mode
        .can_prompt
        .then(|| LocalPrompts::new(Arc::clone(&client), Arc::new(StandardTerminal)));
    let waiting = Waiting {
        events,
        tracker,
        prompts,
        style: context.style,
        timeout: Duration::from_secs(arguments.timeout),
    };
    waiting.run().await
}

fn resolve(
    arguments: &UpArguments,
    snapshot: &SnapshotDto,
    style: Style,
) -> Result<Vec<ResolvedTunnel>, CliError> {
    let interpretation = if arguments.tag {
        TargetInterpretation::TagOnly
    } else {
        TargetInterpretation::PortEndpointOrTag
    };
    Ok(resolve_targets(
        &arguments.targets,
        interpretation,
        snapshot,
        style,
    )?)
}

async fn request_starts(client: &IpcClient, targets: &[ResolvedTunnel]) -> Result<(), CliError> {
    for target in targets {
        client.start_tunnel(&target.id).await?;
    }
    Ok(())
}

struct Waiting {
    events: EventReceiver,
    tracker: ProgressTracker,
    prompts: Option<LocalPrompts>,
    style: Style,
    timeout: Duration,
}

impl Waiting {
    async fn run(mut self) -> Result<(), CliError> {
        let mut remaining = self.timeout;
        while !self.tracker.is_settled() {
            let is_asking_user = self.prompts.as_ref().is_some_and(LocalPrompts::has_pending);
            let waited_from = Instant::now();
            let event = self.next_event(remaining, is_asking_user).await?;
            if !is_asking_user {
                remaining = remaining.saturating_sub(waited_from.elapsed());
            }
            self.handle(&event);
        }
        match self.tracker.unsuccessful_count() {
            0 => Ok(()),
            failed => Err(CliError::TargetsFailed {
                failed,
                total: self.tracker.total(),
            }),
        }
    }

    async fn next_event(
        &mut self,
        remaining: Duration,
        is_asking_user: bool,
    ) -> Result<EventMessage, CliError> {
        let received = if is_asking_user {
            self.events.recv().await
        } else {
            tokio::time::timeout(remaining, self.events.recv())
                .await
                .map_err(|_| CliError::TimedOutWaiting {
                    waited: self.timeout,
                    waiting_for: self.tracker.waiting_labels(),
                })?
        };
        received.ok_or(CliError::ConnectionLost)
    }

    fn handle(&mut self, event: &EventMessage) {
        if let Some(prompts) = self.prompts.as_mut() {
            prompts.handle_event(event);
        }
        match event {
            EventMessage::TunnelChanged(change) => {
                if let Some(label) = self.tracker.apply(change) {
                    println!("{}", describe_step(label, &change.state, self.style));
                }
            }
            EventMessage::HostKeyRequest(HostKeyRequestEvent::Alert {
                alias,
                reason,
                presented_fingerprint,
            }) => StandardTerminal.write(&describe_host_key_alert(
                alias,
                reason,
                presented_fingerprint,
            )),
            _ => {}
        }
    }
}
