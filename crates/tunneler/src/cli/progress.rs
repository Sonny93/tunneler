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
    label: String,
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
                label: resolved.label.clone(),
                status: initial_status(&resolved.id, snapshot),
            })
            .collect();
        Self { targets }
    }

    /// The labels of the tunnels that were active before anything was asked.
    pub fn already_active(&self) -> Vec<&str> {
        self.targets
            .iter()
            .filter(|target| target.status == TargetStatus::Active)
            .map(|target| target.label.as_str())
            .collect()
    }

    /// The labels of the tunnels an external process provided before anything
    /// was asked.
    pub fn already_provided(&self) -> Vec<&str> {
        self.targets
            .iter()
            .filter(|target| target.status == TargetStatus::Provided)
            .map(|target| target.label.as_str())
            .collect()
    }

    /// Takes a state change into account; returns the label of the tunnel it
    /// concerns, or `None` when it is not one of the tracked tunnels.
    pub fn apply(&mut self, change: &TunnelChangedEvent) -> Option<&str> {
        let target = self
            .targets
            .iter_mut()
            .find(|target| target.id == change.id)?;
        target.status = status_after(&change.state);
        Some(&target.label)
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

    /// The labels of the tunnels that are still starting.
    pub fn waiting_labels(&self) -> Vec<String> {
        self.targets
            .iter()
            .filter(|target| target.status == TargetStatus::Waiting)
            .map(|target| target.label.clone())
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

/// The line that announces a step of `tunnel_label`, such as
/// `3333 → nova:3333: active`.
pub fn describe_step(tunnel_label: &str, state: &TunnelStateDto, style: Style) -> String {
    match state {
        TunnelStateDto::Starting => format!("{tunnel_label}: connecting{}", style.ellipsis()),
        TunnelStateDto::Active => format!("{tunnel_label}: active"),
        TunnelStateDto::Reconnecting { attempt, .. } => {
            format!("{tunnel_label}: reconnecting (attempt {attempt})")
        }
        TunnelStateDto::Failed { reason } => {
            format!("{tunnel_label}: failed: {}", describe_failure(reason))
        }
        TunnelStateDto::Stopped => format!("{tunnel_label}: stopped"),
        TunnelStateDto::External { owner } => {
            format!("{tunnel_label}: {}", describe_external(owner))
        }
        TunnelStateDto::Conflict { owner } => {
            format!("{tunnel_label}: {}", describe_conflict(owner.as_ref()))
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

    fn tunnel(id: &str, listen_port: u16, state: TunnelStateDto) -> TunnelDto {
        TunnelDto {
            definition: TunnelDefinitionDto {
                id: id.to_owned(),
                spec: TunnelSpecDto {
                    host: "nova".to_owned(),
                    kind: TunnelKindDto::Local,
                    listen_port,
                    listen_address: None,
                    target_host: None,
                    target_port: None,
                    tags: Vec::new(),
                },
            },
            state,
            health: TargetHealthDto::Unknown,
            metrics: MetricsDto::default(),
            is_temporary: false,
        }
    }

    fn resolved(id: &str, label: &str) -> ResolvedTunnel {
        ResolvedTunnel {
            id: id.to_owned(),
            label: label.to_owned(),
        }
    }

    fn snapshot_of(tunnels: Vec<TunnelDto>) -> SnapshotDto {
        SnapshotDto {
            tray: TrayStatusDto::default(),
            externals: Vec::new(),
            x11_anchors: Vec::new(),
            x11_launchers: Vec::new(),
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
                use_os_keyring: false,
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
            &[
                resolved("1", "3333 → nova:3333"),
                resolved("2", "5432 → nova:5432"),
            ],
            &snapshot_of(vec![
                tunnel("1", 3333, TunnelStateDto::Stopped),
                tunnel("2", 5432, TunnelStateDto::Stopped),
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
            &[resolved("1", "3333 → nova:3333")],
            &snapshot_of(vec![tunnel("1", 3333, external())]),
        );

        assert_eq!(
            (tracker.is_settled(), tracker.already_provided()),
            (true, vec!["3333 → nova:3333"])
        );
    }

    #[test]
    fn should_count_a_failure_when_a_tunnel_is_already_in_conflict() {
        let tracker = ProgressTracker::new(
            &[resolved("1", "3333 → nova:3333")],
            &snapshot_of(vec![tunnel("1", 3333, conflict())]),
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
                describe_step("3333 → nova:3333", &external(), UNICODE),
                describe_step("3333 → nova:3333", &conflict(), UNICODE),
                describe_step(
                    "3333 → nova:3333",
                    &TunnelStateDto::Conflict { owner: None },
                    UNICODE
                ),
            ],
            [
                "3333 → nova:3333: provided by external ssh (pid 4242)",
                "3333 → nova:3333: port held by node (pid 4242)",
                "3333 → nova:3333: port held by an unknown process",
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
            &[resolved("1", "3333 → nova:3333")],
            &snapshot_of(vec![tunnel("1", 3333, TunnelStateDto::Active)]),
        );

        assert_eq!(
            (tracker.is_settled(), tracker.already_active()),
            (true, vec!["3333 → nova:3333"])
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
            &[resolved("1", "3333 → nova:3333")],
            &snapshot_of(vec![tunnel("1", 3333, failure)]),
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

        assert_eq!(
            tracker.waiting_labels(),
            vec!["5432 → nova:5432".to_owned()]
        );
    }

    #[test]
    fn should_ignore_a_change_of_a_tunnel_that_is_not_tracked() {
        let mut tracker = tracker_for_two_stopped_tunnels();

        assert_eq!(tracker.apply(&change("9", TunnelStateDto::Active)), None);
    }

    #[test]
    fn should_label_the_tunnel_when_applying_a_change() {
        let mut tracker = tracker_for_two_stopped_tunnels();

        assert_eq!(
            tracker.apply(&change("2", TunnelStateDto::Starting)),
            Some("5432 → nova:5432")
        );
    }

    #[test]
    fn should_word_each_step_of_a_start() {
        assert_eq!(
            [
                describe_step("3333 → nova:3333", &TunnelStateDto::Starting, UNICODE),
                describe_step("3333 → nova:3333", &TunnelStateDto::Active, UNICODE),
                describe_step(
                    "3333 → nova:3333",
                    &TunnelStateDto::Reconnecting {
                        attempt: 2,
                        next_retry_at: 0
                    },
                    UNICODE
                ),
                describe_step(
                    "3333 → nova:3333",
                    &TunnelStateDto::Failed {
                        reason: FailureReasonDto::Connect {
                            message: "authentication failed".to_owned()
                        }
                    },
                    UNICODE
                ),
            ],
            [
                "3333 → nova:3333: connecting…",
                "3333 → nova:3333: active",
                "3333 → nova:3333: reconnecting (attempt 2)",
                "3333 → nova:3333: failed: authentication failed",
            ]
        );
    }

    #[test]
    fn should_end_the_connecting_step_with_three_dots_when_unicode_is_off() {
        assert_eq!(
            describe_step("3333 → nova:3333", &TunnelStateDto::Starting, ASCII),
            "3333 → nova:3333: connecting..."
        );
    }
}
