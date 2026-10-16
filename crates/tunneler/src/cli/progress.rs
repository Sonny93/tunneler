//! Follows the tunnels `up` started until each is active or has failed, and
//! words each step as a line of text.

use ipc::protocol::{SnapshotDto, TunnelChangedEvent, TunnelStateDto};

use super::render::{describe_conflict, describe_external, describe_failure};
use super::style::Style;
use super::target::ResolvedTunnel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetStatus {
    Waiting,
    Active,
    Provided,
    Unsuccessful,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    id: String,
    name: String,
    status: TargetStatus,
}

/// Where each started tunnel stands.
///
/// Only tunnels that are already active or provided by an external process
/// count as done from the start: a stopped or failed one still holds the
/// outcome of an earlier run, so only the events that follow the start request
/// tell how this run went. A tunnel already in conflict counts as unsuccessful,
/// since starting it again reports the same state and so sends no event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressTracker {
    targets: Vec<Target>,
}

impl ProgressTracker {
    /// Tracks `targets`, starting from the states of `snapshot`.
    pub fn new(targets: &[ResolvedTunnel], snapshot: &SnapshotDto) -> Self {
        let targets = targets
            .iter()
            .map(|resolved| Target {
                id: resolved.id.clone(),
                name: resolved.name.clone(),
                status: initial_status(&resolved.id, snapshot),
            })
            .collect();
        Self { targets }
    }

    /// The names of the tunnels that were active before anything was asked.
    pub fn already_active(&self) -> Vec<&str> {
        self.targets
            .iter()
            .filter(|target| target.status == TargetStatus::Active)
            .map(|target| target.name.as_str())
            .collect()
    }

    /// The names of the tunnels an external process provided before anything
    /// was asked.
    pub fn already_provided(&self) -> Vec<&str> {
        self.targets
            .iter()
            .filter(|target| target.status == TargetStatus::Provided)
            .map(|target| target.name.as_str())
            .collect()
    }

    /// Takes a state change into account; returns the name of the tunnel it
    /// concerns, or `None` when it is not one of the tracked tunnels.
    pub fn apply(&mut self, change: &TunnelChangedEvent) -> Option<&str> {
        let target = self
            .targets
            .iter_mut()
            .find(|target| target.id == change.id)?;
        target.status = status_after(&change.state);
        Some(&target.name)
    }

    /// Whether every tunnel is either active, provided or unsuccessful.
    pub fn is_settled(&self) -> bool {
        self.targets
            .iter()
            .all(|target| target.status != TargetStatus::Waiting)
    }

    /// How many tunnels ended up failed or stopped.
    pub fn unsuccessful_count(&self) -> usize {
        self.targets
            .iter()
            .filter(|target| target.status == TargetStatus::Unsuccessful)
            .count()
    }

    /// How many tunnels are tracked.
    pub fn total(&self) -> usize {
        self.targets.len()
    }

    /// The names of the tunnels that are still starting.
    pub fn waiting_names(&self) -> Vec<String> {
        self.targets
            .iter()
            .filter(|target| target.status == TargetStatus::Waiting)
            .map(|target| target.name.clone())
            .collect()
    }
}

fn initial_status(id: &str, snapshot: &SnapshotDto) -> TargetStatus {
    let state = snapshot
        .tunnels
        .iter()
        .find(|tunnel| tunnel.definition.id == id)
        .map(|tunnel| &tunnel.state);
    match state {
        Some(TunnelStateDto::Active) => TargetStatus::Active,
        Some(TunnelStateDto::External { .. }) => TargetStatus::Provided,
        Some(TunnelStateDto::Conflict { .. }) => TargetStatus::Unsuccessful,
        _ => TargetStatus::Waiting,
    }
}

fn status_after(state: &TunnelStateDto) -> TargetStatus {
    match state {
        TunnelStateDto::Active => TargetStatus::Active,
        TunnelStateDto::External { .. } => TargetStatus::Provided,
        TunnelStateDto::Failed { .. }
        | TunnelStateDto::Stopped
        | TunnelStateDto::Conflict { .. } => TargetStatus::Unsuccessful,
        TunnelStateDto::Starting | TunnelStateDto::Reconnecting { .. } => TargetStatus::Waiting,
    }
}

/// The line that announces a step of `tunnel_name`, such as `Adonis: active`.
pub fn describe_step(tunnel_name: &str, state: &TunnelStateDto, style: Style) -> String {
    match state {
        TunnelStateDto::Starting => format!("{tunnel_name}: connecting{}", style.ellipsis()),
        TunnelStateDto::Active => format!("{tunnel_name}: active"),
        TunnelStateDto::Reconnecting { attempt, .. } => {
            format!("{tunnel_name}: reconnecting (attempt {attempt})")
        }
        TunnelStateDto::Failed { reason } => {
            format!("{tunnel_name}: failed: {}", describe_failure(reason))
        }
        TunnelStateDto::Stopped => format!("{tunnel_name}: stopped"),
        TunnelStateDto::External { owner } => {
            format!("{tunnel_name}: {}", describe_external(owner))
        }
        TunnelStateDto::Conflict { owner } => {
            format!("{tunnel_name}: {}", describe_conflict(owner.as_ref()))
        }
    }
}

#[cfg(test)]
mod tests {
    use ipc::protocol::{
        FailureReasonDto, MetricsDto, PortOwnerDto, SettingsDto, TargetHealthDto, TrayStatusDto,
        TunnelDefinitionDto, TunnelDto, TunnelKindDto, TunnelSpecDto,
    };

    use super::*;

    const UNICODE: Style = Style {
        color: false,
        unicode: true,
    };
    const ASCII: Style = Style {
        color: false,
        unicode: false,
    };

    fn tunnel(id: &str, name: &str, state: TunnelStateDto) -> TunnelDto {
        TunnelDto {
            definition: TunnelDefinitionDto {
                id: id.to_owned(),
                spec: TunnelSpecDto {
                    name: name.to_owned(),
                    host: "nova".to_owned(),
                    kind: TunnelKindDto::Local,
                    listen_port: 3333,
                    listen_address: None,
                    target_host: None,
                    target_port: None,
                    tags: Vec::new(),
                },
            },
            state,
            health: TargetHealthDto::Unknown,
            metrics: MetricsDto::default(),
        }
    }

    fn resolved(id: &str, name: &str) -> ResolvedTunnel {
        ResolvedTunnel {
            id: id.to_owned(),
            name: name.to_owned(),
        }
    }

    fn snapshot_of(tunnels: Vec<TunnelDto>) -> SnapshotDto {
        SnapshotDto {
            tray: TrayStatusDto::default(),
            externals: Vec::new(),
            tunnels,
            settings: SettingsDto {
                default_bind_address: "127.0.0.1".to_owned(),
                default_remote_bind_address: "localhost".to_owned(),
                keepalive_interval_ms: 0,
                keepalive_max: 3,
                session_idle_grace_ms: 0,
                reconnect_initial_delay_ms: 0,
                reconnect_max_delay_ms: 0,
                health_probe: true,
                restore_on_launch: true,
                start_on_launch_tags: Vec::new(),
                launch_at_login: false,
                x11_display: None,
            },
        }
    }

    fn change(id: &str, state: TunnelStateDto) -> TunnelChangedEvent {
        TunnelChangedEvent {
            id: id.to_owned(),
            state,
            health: TargetHealthDto::Unknown,
        }
    }

    fn tracker_for_two_stopped_tunnels() -> ProgressTracker {
        ProgressTracker::new(
            &[resolved("1", "Adonis"), resolved("2", "Mail")],
            &snapshot_of(vec![
                tunnel("1", "Adonis", TunnelStateDto::Stopped),
                tunnel("2", "Mail", TunnelStateDto::Stopped),
            ]),
        )
    }

    fn external() -> TunnelStateDto {
        TunnelStateDto::External {
            owner: PortOwnerDto {
                pid: 4242,
                process: "ssh".to_owned(),
            },
        }
    }

    fn conflict() -> TunnelStateDto {
        TunnelStateDto::Conflict {
            owner: Some(PortOwnerDto {
                pid: 4242,
                process: "node".to_owned(),
            }),
        }
    }

    #[test]
    fn should_settle_successfully_when_a_tunnel_becomes_external() {
        let mut tracker = tracker_for_two_stopped_tunnels();

        tracker.apply(&change("1", external()));
        tracker.apply(&change("2", TunnelStateDto::Active));

        assert_eq!(
            (tracker.is_settled(), tracker.unsuccessful_count()),
            (true, 0)
        );
    }

    #[test]
    fn should_count_a_failure_when_a_tunnel_ends_in_conflict() {
        let mut tracker = tracker_for_two_stopped_tunnels();

        tracker.apply(&change("1", TunnelStateDto::Active));
        tracker.apply(&change("2", conflict()));

        assert_eq!(
            (tracker.is_settled(), tracker.unsuccessful_count()),
            (true, 1)
        );
    }

    #[test]
    fn should_consider_a_tunnel_done_when_it_is_already_provided_externally() {
        let tracker = ProgressTracker::new(
            &[resolved("1", "Adonis")],
            &snapshot_of(vec![tunnel("1", "Adonis", external())]),
        );

        assert_eq!(
            (tracker.is_settled(), tracker.already_provided()),
            (true, vec!["Adonis"])
        );
    }

    #[test]
    fn should_count_a_failure_when_a_tunnel_is_already_in_conflict() {
        let tracker = ProgressTracker::new(
            &[resolved("1", "Adonis")],
            &snapshot_of(vec![tunnel("1", "Adonis", conflict())]),
        );

        assert_eq!(
            (tracker.is_settled(), tracker.unsuccessful_count()),
            (true, 1)
        );
    }

    #[test]
    fn should_word_the_external_and_conflict_steps() {
        assert_eq!(
            [
                describe_step("Adonis", &external(), UNICODE),
                describe_step("Adonis", &conflict(), UNICODE),
                describe_step("Adonis", &TunnelStateDto::Conflict { owner: None }, UNICODE),
            ],
            [
                "Adonis: provided by external ssh (pid 4242)",
                "Adonis: port held by node (pid 4242)",
                "Adonis: port held by an unknown process",
            ]
        );
    }

    #[test]
    fn should_wait_when_the_tunnels_are_stopped() {
        assert!(!tracker_for_two_stopped_tunnels().is_settled());
    }

    #[test]
    fn should_consider_a_tunnel_done_when_it_is_already_active() {
        let tracker = ProgressTracker::new(
            &[resolved("1", "Adonis")],
            &snapshot_of(vec![tunnel("1", "Adonis", TunnelStateDto::Active)]),
        );

        assert_eq!(
            (tracker.is_settled(), tracker.already_active()),
            (true, vec!["Adonis"])
        );
    }

    #[test]
    fn should_still_wait_when_the_snapshot_holds_an_old_failure() {
        let failure = TunnelStateDto::Failed {
            reason: FailureReasonDto::Connect {
                message: "old".to_owned(),
            },
        };
        let tracker = ProgressTracker::new(
            &[resolved("1", "Adonis")],
            &snapshot_of(vec![tunnel("1", "Adonis", failure)]),
        );

        assert!(!tracker.is_settled());
    }

    #[test]
    fn should_settle_when_every_tunnel_becomes_active() {
        let mut tracker = tracker_for_two_stopped_tunnels();

        tracker.apply(&change("1", TunnelStateDto::Starting));
        tracker.apply(&change("1", TunnelStateDto::Active));
        tracker.apply(&change("2", TunnelStateDto::Active));

        assert_eq!(
            (tracker.is_settled(), tracker.unsuccessful_count()),
            (true, 0)
        );
    }

    #[test]
    fn should_count_a_failure_when_a_tunnel_fails() {
        let mut tracker = tracker_for_two_stopped_tunnels();

        tracker.apply(&change("1", TunnelStateDto::Active));
        tracker.apply(&change(
            "2",
            TunnelStateDto::Failed {
                reason: FailureReasonDto::PortInUse {
                    address: "127.0.0.1:3333".to_owned(),
                },
            },
        ));

        assert_eq!(
            (tracker.is_settled(), tracker.unsuccessful_count()),
            (true, 1)
        );
    }

    #[test]
    fn should_keep_waiting_when_a_tunnel_is_reconnecting() {
        let mut tracker = tracker_for_two_stopped_tunnels();

        tracker.apply(&change("1", TunnelStateDto::Active));
        tracker.apply(&change(
            "2",
            TunnelStateDto::Reconnecting {
                attempt: 1,
                next_retry_at: 0,
            },
        ));

        assert_eq!(tracker.waiting_names(), vec!["Mail".to_owned()]);
    }

    #[test]
    fn should_ignore_a_change_of_a_tunnel_that_is_not_tracked() {
        let mut tracker = tracker_for_two_stopped_tunnels();

        assert_eq!(tracker.apply(&change("9", TunnelStateDto::Active)), None);
    }

    #[test]
    fn should_name_the_tunnel_when_applying_a_change() {
        let mut tracker = tracker_for_two_stopped_tunnels();

        assert_eq!(
            tracker.apply(&change("2", TunnelStateDto::Starting)),
            Some("Mail")
        );
    }

    #[test]
    fn should_word_each_step_of_a_start() {
        assert_eq!(
            [
                describe_step("Adonis", &TunnelStateDto::Starting, UNICODE),
                describe_step("Adonis", &TunnelStateDto::Active, UNICODE),
                describe_step(
                    "Adonis",
                    &TunnelStateDto::Reconnecting {
                        attempt: 2,
                        next_retry_at: 0
                    },
                    UNICODE
                ),
                describe_step(
                    "Adonis",
                    &TunnelStateDto::Failed {
                        reason: FailureReasonDto::Connect {
                            message: "authentication failed".to_owned()
                        }
                    },
                    UNICODE
                ),
            ],
            [
                "Adonis: connecting…",
                "Adonis: active",
                "Adonis: reconnecting (attempt 2)",
                "Adonis: failed: authentication failed",
            ]
        );
    }

    #[test]
    fn should_end_the_connecting_step_with_three_dots_when_unicode_is_off() {
        assert_eq!(
            describe_step("Adonis", &TunnelStateDto::Starting, ASCII),
            "Adonis: connecting..."
        );
    }
}
