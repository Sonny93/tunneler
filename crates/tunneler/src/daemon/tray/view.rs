//! What the tray shows, computed from the engine's snapshot: pure functions
//! that know nothing about the platform.

use std::collections::BTreeMap;

use engine::{EngineSnapshot, TargetHealth, TunnelSnapshot, TunnelState};

const TOOLTIP_PREFIX: &str = "Tunneler — ";
const SUMMARY_SEPARATOR: &str = ", ";

/// The worst thing going on across every tunnel, which picks the icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GlobalStatus {
    /// Nothing runs.
    Idle,
    /// Tunnels run and nothing is wrong.
    Healthy,
    /// A tunnel reconnects or a running tunnel cannot reach its target.
    Degraded,
    /// A tunnel failed and stopped.
    Error,
}

/// A tag of the menu, with how many of its tunnels run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagEntry {
    /// The tag.
    pub(crate) tag: String,
    /// How many tunnels carrying the tag are starting, active or reconnecting.
    pub(crate) running: usize,
    /// How many tunnels carry the tag.
    pub(crate) total: usize,
    /// Whether every tunnel of the tag runs, which is what the menu item shows.
    pub(crate) is_checked: bool,
}

impl TagEntry {
    /// The text of the menu item: the state is spelled out, not only checked.
    #[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
    pub(crate) fn label(&self) -> String {
        format!("{} — {} sur {} actifs", self.tag, self.running, self.total)
    }
}

/// Everything the tray displays; two equal views need no redraw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayView {
    /// Picks the icon.
    pub(crate) status: GlobalStatus,
    /// The hover text of the icon.
    pub(crate) tooltip: String,
    /// The first line of the menu.
    pub(crate) summary_line: String,
    /// One entry per tag, in menu order.
    pub(crate) tags: Vec<TagEntry>,
}

/// How many tunnels are in each situation.
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    active: usize,
    starting: usize,
    reconnecting: usize,
    failed: usize,
    unreachable: usize,
}

impl Counts {
    fn of(tunnels: &[TunnelSnapshot]) -> Self {
        tunnels.iter().fold(Self::default(), |mut counts, tunnel| {
            counts.add(tunnel);
            counts
        })
    }

    fn add(&mut self, tunnel: &TunnelSnapshot) {
        match tunnel.state {
            TunnelState::Active => self.active += 1,
            TunnelState::Starting => self.starting += 1,
            TunnelState::Reconnecting { .. } => self.reconnecting += 1,
            TunnelState::Failed { .. } => self.failed += 1,
            TunnelState::Stopped | TunnelState::External { .. } | TunnelState::Conflict { .. } => {}
        }
        if is_unreachable_while_running(tunnel) {
            self.unreachable += 1;
        }
    }

    fn global_status(&self) -> GlobalStatus {
        if self.failed > 0 {
            GlobalStatus::Error
        } else if self.reconnecting > 0 || self.unreachable > 0 {
            GlobalStatus::Degraded
        } else if self.active > 0 || self.starting > 0 {
            GlobalStatus::Healthy
        } else {
            GlobalStatus::Idle
        }
    }

    fn summary(&self) -> String {
        let optional_parts = [
            (self.starting, "en démarrage"),
            (self.reconnecting, "en reconnexion"),
            (self.failed, "en échec"),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, wording)| format!("{count} {wording}"));
        std::iter::once(describe_active(self.active))
            .chain(optional_parts)
            .chain(describe_unreachable(self.unreachable))
            .collect::<Vec<_>>()
            .join(SUMMARY_SEPARATOR)
    }
}

fn is_unreachable_while_running(tunnel: &TunnelSnapshot) -> bool {
    tunnel.state.is_running() && matches!(tunnel.health, TargetHealth::Unreachable { .. })
}

fn describe_active(count: usize) -> String {
    match count {
        0 => "aucun tunnel actif".to_owned(),
        1 => "1 tunnel actif".to_owned(),
        count => format!("{count} tunnels actifs"),
    }
}

fn describe_unreachable(count: usize) -> Option<String> {
    match count {
        0 => None,
        1 => Some("1 cible injoignable".to_owned()),
        count => Some(format!("{count} cibles injoignables")),
    }
}

fn capitalize_first_letter(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

fn tag_entries(tunnels: &[TunnelSnapshot]) -> Vec<TagEntry> {
    let mut totals: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for tunnel in tunnels {
        for tag in &tunnel.definition.tags {
            let (running, total) = totals.entry(tag.as_str()).or_default();
            *total += 1;
            *running += usize::from(tunnel.state.is_running());
        }
    }
    let mut entries: Vec<TagEntry> = totals
        .into_iter()
        .map(|(tag, (running, total))| TagEntry {
            tag: tag.to_owned(),
            running,
            total,
            is_checked: running == total,
        })
        .collect();
    entries.sort_by_cached_key(|entry| (entry.tag.to_lowercase(), entry.tag.clone()));
    entries
}

/// What the tray displays for this snapshot.
pub(crate) fn tray_view(snapshot: &EngineSnapshot) -> TrayView {
    let counts = Counts::of(&snapshot.tunnels);
    let summary = counts.summary();
    TrayView {
        status: counts.global_status(),
        tooltip: format!("{TOOLTIP_PREFIX}{summary}"),
        summary_line: capitalize_first_letter(&summary),
        tags: tag_entries(&snapshot.tunnels),
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use engine::{FailureReason, Settings, TunnelDefinition, TunnelId, TunnelKind, TunnelMetrics};
    use pretty_assertions::assert_eq;

    use super::*;

    fn tunnel(state: TunnelState, health: TargetHealth, tags: &[&str]) -> TunnelSnapshot {
        TunnelSnapshot {
            definition: TunnelDefinition {
                id: TunnelId::generate(),
                host: "nova".to_owned(),
                kind: TunnelKind::Local,
                listen_port: 8080,
                listen_address: None,
                target_host: None,
                target_port: None,
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                open: engine::OpenOptions::default(),
            },
            state,
            health,
            metrics: TunnelMetrics::default(),
            is_temporary: false,
        }
    }

    fn reconnecting() -> TunnelState {
        TunnelState::Reconnecting {
            attempt: 1,
            next_retry_at: SystemTime::UNIX_EPOCH,
        }
    }

    fn failed() -> TunnelState {
        TunnelState::Failed {
            reason: FailureReason::Connect {
                message: "refused".to_owned(),
            },
        }
    }

    fn unreachable() -> TargetHealth {
        TargetHealth::Unreachable {
            since: SystemTime::UNIX_EPOCH,
            error: "refused".to_owned(),
        }
    }

    fn view_of(tunnels: Vec<TunnelSnapshot>) -> TrayView {
        tray_view(&EngineSnapshot {
            tunnels,
            settings: Settings::default(),
            externals: Vec::new(),
            x11_anchors: Vec::new(),
            x11_launchers: Vec::new(),
            openers: Vec::new(),
        })
    }

    fn active(tags: &[&str]) -> TunnelSnapshot {
        tunnel(TunnelState::Active, TargetHealth::Reachable, tags)
    }

    fn stopped(tags: &[&str]) -> TunnelSnapshot {
        tunnel(TunnelState::Stopped, TargetHealth::Unknown, tags)
    }

    #[test]
    fn should_be_idle_when_there_is_no_tunnel() {
        assert_eq!(view_of(vec![]).status, GlobalStatus::Idle);
    }

    #[test]
    fn should_be_idle_when_every_tunnel_is_stopped() {
        assert_eq!(view_of(vec![stopped(&[])]).status, GlobalStatus::Idle);
    }

    #[test]
    fn should_be_idle_when_the_tunnels_are_external_or_in_conflict() {
        let external = TunnelState::External {
            owner: engine::PortOwner {
                pid: 4242,
                process: "ssh".to_owned(),
            },
        };
        let conflict = TunnelState::Conflict { owner: None };
        let tunnels = vec![
            tunnel(external, TargetHealth::Unknown, &[]),
            tunnel(conflict, TargetHealth::Unknown, &[]),
        ];

        assert_eq!(view_of(tunnels).status, GlobalStatus::Idle);
    }

    #[test]
    fn should_be_healthy_when_a_tunnel_is_active() {
        assert_eq!(view_of(vec![active(&[])]).status, GlobalStatus::Healthy);
    }

    #[test]
    fn should_be_healthy_when_a_tunnel_is_starting() {
        let starting = tunnel(TunnelState::Starting, TargetHealth::Unknown, &[]);

        assert_eq!(view_of(vec![starting]).status, GlobalStatus::Healthy);
    }

    #[test]
    fn should_be_degraded_when_a_tunnel_reconnects() {
        let tunnels = vec![
            active(&[]),
            tunnel(reconnecting(), TargetHealth::Unknown, &[]),
        ];

        assert_eq!(view_of(tunnels).status, GlobalStatus::Degraded);
    }

    #[test]
    fn should_be_degraded_when_a_running_tunnel_cannot_reach_its_target() {
        let tunnels = vec![tunnel(TunnelState::Active, unreachable(), &[])];

        assert_eq!(view_of(tunnels).status, GlobalStatus::Degraded);
    }

    #[test]
    fn should_stay_idle_when_a_stopped_tunnel_remembers_an_unreachable_target() {
        let tunnels = vec![tunnel(TunnelState::Stopped, unreachable(), &[])];

        assert_eq!(view_of(tunnels).status, GlobalStatus::Idle);
    }

    #[test]
    fn should_be_an_error_when_a_tunnel_failed_even_if_others_are_degraded() {
        let tunnels = vec![
            tunnel(reconnecting(), TargetHealth::Unknown, &[]),
            tunnel(failed(), TargetHealth::Unknown, &[]),
        ];

        assert_eq!(view_of(tunnels).status, GlobalStatus::Error);
    }

    #[test]
    fn should_say_that_no_tunnel_is_active_when_nothing_runs() {
        assert_eq!(view_of(vec![]).summary_line, "Aucun tunnel actif");
    }

    #[test]
    fn should_use_the_singular_when_one_tunnel_is_active() {
        assert_eq!(view_of(vec![active(&[])]).summary_line, "1 tunnel actif");
    }

    #[test]
    fn should_use_the_plural_when_several_tunnels_are_active() {
        let tunnels = vec![active(&[]), active(&[]), active(&[])];

        assert_eq!(view_of(tunnels).summary_line, "3 tunnels actifs");
    }

    #[test]
    fn should_list_every_non_zero_count_in_order_when_summarizing() {
        let tunnels = vec![
            active(&[]),
            tunnel(TunnelState::Starting, TargetHealth::Unknown, &[]),
            tunnel(reconnecting(), TargetHealth::Unknown, &[]),
            tunnel(failed(), TargetHealth::Unknown, &[]),
            tunnel(TunnelState::Active, unreachable(), &[]),
            tunnel(TunnelState::Active, unreachable(), &[]),
        ];

        assert_eq!(
            view_of(tunnels).summary_line,
            "3 tunnels actifs, 1 en démarrage, 1 en reconnexion, 1 en échec, 2 cibles injoignables"
        );
    }

    #[test]
    fn should_mention_a_single_unreachable_target_in_the_singular() {
        let tunnels = vec![tunnel(TunnelState::Active, unreachable(), &[])];

        assert_eq!(
            view_of(tunnels).summary_line,
            "1 tunnel actif, 1 cible injoignable"
        );
    }

    #[test]
    fn should_prefix_the_tooltip_with_the_program_name_and_keep_the_summary_lowercase() {
        let tunnels = vec![
            active(&[]),
            active(&[]),
            active(&[]),
            tunnel(reconnecting(), TargetHealth::Unknown, &[]),
        ];

        assert_eq!(
            view_of(tunnels).tooltip,
            "Tunneler — 3 tunnels actifs, 1 en reconnexion"
        );
    }

    #[test]
    fn should_list_each_tag_once_sorted_ignoring_case() {
        let tunnels = vec![
            stopped(&["prod", "Dev"]),
            stopped(&["dev"]),
            stopped(&["alpha", "prod"]),
        ];

        let tags: Vec<String> = view_of(tunnels)
            .tags
            .into_iter()
            .map(|entry| entry.tag)
            .collect();

        assert_eq!(tags, vec!["alpha", "Dev", "dev", "prod"]);
    }

    #[test]
    fn should_count_the_running_and_total_tunnels_of_a_tag() {
        let tunnels = vec![active(&["dev"]), stopped(&["dev"]), active(&["dev"])];

        let entry = view_of(tunnels).tags.remove(0);

        assert_eq!((entry.running, entry.total), (2, 3));
    }

    #[test]
    fn should_check_a_tag_when_all_its_tunnels_run() {
        let tunnels = vec![active(&["dev"]), active(&["dev"])];

        assert!(view_of(tunnels).tags[0].is_checked);
    }

    #[test]
    fn should_not_check_a_tag_when_one_of_its_tunnels_is_stopped() {
        let tunnels = vec![active(&["dev"]), stopped(&["dev"])];

        assert!(!view_of(tunnels).tags[0].is_checked);
    }

    #[test]
    fn should_count_a_reconnecting_tunnel_as_running_for_its_tag() {
        let tunnels = vec![tunnel(reconnecting(), TargetHealth::Unknown, &["dev"])];

        assert!(view_of(tunnels).tags[0].is_checked);
    }

    #[test]
    fn should_spell_the_active_count_in_the_label_of_a_tag() {
        let tunnels = vec![active(&["dev"]), stopped(&["dev"])];

        assert_eq!(view_of(tunnels).tags[0].label(), "dev — 1 sur 2 actifs");
    }

    #[test]
    fn should_report_equal_views_when_nothing_changed() {
        let tunnels = || vec![active(&["dev"])];

        assert_eq!(view_of(tunnels()), view_of(tunnels()));
    }
}
