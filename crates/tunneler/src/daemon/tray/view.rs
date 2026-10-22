//! What the tray shows, computed from the engine's snapshot: pure functions
//! that know nothing about the platform.

use std::collections::BTreeMap;

use engine::{EngineSnapshot, TargetHealth, TunnelSnapshot, TunnelState};

use crate::daemon::display_language::DisplayLanguage;

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
    pub(crate) fn label(&self, language: DisplayLanguage) -> String {
        let (running, total) = (self.running, self.total);
        match language {
            DisplayLanguage::English => format!("{} — {running} of {total} active", self.tag),
            DisplayLanguage::French => format!("{} — {running} sur {total} actifs", self.tag),
        }
    }
}

/// Everything the tray displays; two equal views need no redraw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayView {
    /// The language of the texts of the menu.
    pub(crate) language: DisplayLanguage,
    /// Picks the icon.
    pub(crate) status: GlobalStatus,
    /// The hover text of the icon.
    pub(crate) tooltip: String,
    /// The first line of the menu.
    pub(crate) summary_line: String,
    /// One entry per tag, in menu order.
    pub(crate) tags: Vec<TagEntry>,
    /// Whether a tunnel is woken from standby, so that the menu can send it
    /// back.
    pub(crate) has_awake_tunnels: bool,
}

/// How many tunnels are in each situation.
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    active: usize,
    standby: usize,
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
            TunnelState::Standby { .. } => self.standby += 1,
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

    fn summary(&self, language: DisplayLanguage) -> String {
        let wordings = SituationWordings::of(language);
        let optional_parts = [
            (self.standby, wordings.standby),
            (self.starting, wordings.starting),
            (self.reconnecting, wordings.reconnecting),
            (self.failed, wordings.failed),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, wording)| format!("{count} {wording}"));
        std::iter::once(describe_active(self.active, language))
            .chain(optional_parts)
            .chain(describe_unreachable(self.unreachable, language))
            .collect::<Vec<_>>()
            .join(SUMMARY_SEPARATOR)
    }
}

fn is_unreachable_while_running(tunnel: &TunnelSnapshot) -> bool {
    tunnel.state.is_running() && matches!(tunnel.health, TargetHealth::Unreachable { .. })
}

/// What follows a count in the summary, for the situations that need no
/// plural.
struct SituationWordings {
    standby: &'static str,
    starting: &'static str,
    reconnecting: &'static str,
    failed: &'static str,
}

impl SituationWordings {
    fn of(language: DisplayLanguage) -> Self {
        match language {
            DisplayLanguage::English => Self {
                standby: "on standby",
                starting: "starting",
                reconnecting: "reconnecting",
                failed: "failed",
            },
            DisplayLanguage::French => Self {
                standby: "en veille",
                starting: "en démarrage",
                reconnecting: "en reconnexion",
                failed: "en échec",
            },
        }
    }
}

fn describe_active(count: usize, language: DisplayLanguage) -> String {
    match (language, count) {
        (DisplayLanguage::English, 0) => "no active tunnel".to_owned(),
        (DisplayLanguage::English, 1) => "1 active tunnel".to_owned(),
        (DisplayLanguage::English, count) => format!("{count} active tunnels"),
        (DisplayLanguage::French, 0) => "aucun tunnel actif".to_owned(),
        (DisplayLanguage::French, 1) => "1 tunnel actif".to_owned(),
        (DisplayLanguage::French, count) => format!("{count} tunnels actifs"),
    }
}

fn describe_unreachable(count: usize, language: DisplayLanguage) -> Option<String> {
    match (language, count) {
        (_, 0) => None,
        (DisplayLanguage::English, 1) => Some("1 unreachable target".to_owned()),
        (DisplayLanguage::English, count) => Some(format!("{count} unreachable targets")),
        (DisplayLanguage::French, 1) => Some("1 cible injoignable".to_owned()),
        (DisplayLanguage::French, count) => Some(format!("{count} cibles injoignables")),
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

/// What the tray displays for this snapshot, in `language`.
pub(crate) fn tray_view(snapshot: &EngineSnapshot, language: DisplayLanguage) -> TrayView {
    let counts = Counts::of(&snapshot.tunnels);
    let summary = counts.summary(language);
    TrayView {
        language,
        status: counts.global_status(),
        tooltip: format!("{TOOLTIP_PREFIX}{summary}"),
        summary_line: capitalize_first_letter(&summary),
        tags: tag_entries(&snapshot.tunnels),
        has_awake_tunnels: snapshot.tunnels.iter().any(|tunnel| tunnel.is_awake),
    }
}

/// What the tray displays for this snapshot, in the language its settings
/// ask for.
pub(crate) fn current_tray_view(snapshot: &EngineSnapshot) -> TrayView {
    tray_view(
        snapshot,
        DisplayLanguage::current(snapshot.settings.language),
    )
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
                standby: None,
            },
            state,
            health,
            metrics: TunnelMetrics::default(),
            is_awake: false,
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

    fn view_in(language: DisplayLanguage, tunnels: Vec<TunnelSnapshot>) -> TrayView {
        tray_view(
            &EngineSnapshot {
                tunnels,
                settings: Settings::default(),
                externals: Vec::new(),
                x11_anchors: Vec::new(),
                x11_launchers: Vec::new(),
                openers: Vec::new(),
            },
            language,
        )
    }

    fn view_of(tunnels: Vec<TunnelSnapshot>) -> TrayView {
        view_in(DisplayLanguage::English, tunnels)
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
    fn should_stay_idle_when_the_only_tunnels_are_on_standby() {
        let standby = tunnel(
            TunnelState::Standby { last_failure: None },
            TargetHealth::Unknown,
            &[],
        );

        assert_eq!(view_of(vec![standby]).status, GlobalStatus::Idle);
    }

    #[test]
    fn should_count_the_standby_tunnels_in_the_summary() {
        let standby = || {
            tunnel(
                TunnelState::Standby { last_failure: None },
                TargetHealth::Unknown,
                &[],
            )
        };

        assert_eq!(
            view_of(vec![standby(), standby()]).summary_line,
            "No active tunnel, 2 on standby"
        );
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
    fn should_be_degraded_and_mention_the_unreachable_target_in_the_singular_when_a_running_tunnel_cannot_reach_it()
     {
        let view = view_of(vec![tunnel(TunnelState::Active, unreachable(), &[])]);

        assert_eq!(view.status, GlobalStatus::Degraded);
        assert_eq!(view.summary_line, "1 active tunnel, 1 unreachable target");
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
        assert_eq!(view_of(vec![]).summary_line, "No active tunnel");
    }

    #[test]
    fn should_use_the_singular_when_one_tunnel_is_active() {
        assert_eq!(view_of(vec![active(&[])]).summary_line, "1 active tunnel");
    }

    #[test]
    fn should_use_the_plural_when_several_tunnels_are_active() {
        let tunnels = vec![active(&[]), active(&[]), active(&[])];

        assert_eq!(view_of(tunnels).summary_line, "3 active tunnels");
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
            "3 active tunnels, 1 starting, 1 reconnecting, 1 failed, 2 unreachable targets"
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
            "Tunneler — 3 active tunnels, 1 reconnecting"
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
    fn should_not_check_the_tag_and_spell_its_active_count_when_one_of_its_tunnels_is_stopped() {
        let view = view_of(vec![active(&["dev"]), stopped(&["dev"])]);

        assert!(!view.tags[0].is_checked);
        assert_eq!(view.tags[0].label(view.language), "dev — 1 of 2 active");
    }

    #[test]
    fn should_count_a_reconnecting_tunnel_as_running_for_its_tag() {
        let tunnels = vec![tunnel(reconnecting(), TargetHealth::Unknown, &["dev"])];

        assert!(view_of(tunnels).tags[0].is_checked);
    }

    #[test]
    fn should_count_a_standby_tunnel_as_running_for_its_tag() {
        let tunnels = vec![
            tunnel(
                TunnelState::Standby { last_failure: None },
                TargetHealth::Unknown,
                &["dev"],
            ),
            active(&["dev"]),
        ];

        let tag = view_of(tunnels).tags.remove(0);

        assert_eq!((tag.running, tag.is_checked), (2, true));
    }

    #[test]
    fn should_have_no_awake_tunnel_when_none_is_woken() {
        assert!(!view_of(vec![active(&[])]).has_awake_tunnels);
    }

    #[test]
    fn should_have_an_awake_tunnel_when_one_is_woken() {
        let awake = TunnelSnapshot {
            is_awake: true,
            ..active(&[])
        };

        assert!(view_of(vec![active(&[]), awake]).has_awake_tunnels);
    }

    #[test]
    fn should_report_equal_views_when_nothing_changed() {
        let tunnels = || vec![active(&["dev"])];

        assert_eq!(view_of(tunnels()), view_of(tunnels()));
    }

    #[test]
    fn should_summarize_in_french_when_the_language_is_french() {
        let tunnels = vec![
            active(&[]),
            active(&[]),
            tunnel(
                TunnelState::Standby { last_failure: None },
                TargetHealth::Unknown,
                &[],
            ),
            tunnel(TunnelState::Starting, TargetHealth::Unknown, &[]),
            tunnel(reconnecting(), TargetHealth::Unknown, &[]),
            tunnel(failed(), TargetHealth::Unknown, &[]),
            tunnel(TunnelState::Active, unreachable(), &[]),
        ];

        let view = view_in(DisplayLanguage::French, tunnels);

        assert_eq!(
            view.summary_line,
            "3 tunnels actifs, 1 en veille, 1 en démarrage, 1 en reconnexion, 1 en échec, 1 cible injoignable"
        );
        assert_eq!(
            view.tooltip,
            format!("Tunneler — {}", view.summary_line.to_lowercase())
        );
    }

    #[test]
    fn should_use_the_french_singular_and_zero_forms_when_the_language_is_french() {
        assert_eq!(
            view_in(DisplayLanguage::French, vec![]).summary_line,
            "Aucun tunnel actif"
        );
        assert_eq!(
            view_in(DisplayLanguage::French, vec![active(&[])]).summary_line,
            "1 tunnel actif"
        );
    }

    #[test]
    fn should_label_a_tag_in_french_when_the_language_is_french() {
        let view = view_in(
            DisplayLanguage::French,
            vec![active(&["dev"]), stopped(&["dev"])],
        );

        assert_eq!(view.tags[0].label(view.language), "dev — 1 sur 2 actifs");
    }

    #[test]
    fn should_report_different_views_when_only_the_language_differs() {
        assert_ne!(
            view_in(DisplayLanguage::English, vec![]),
            view_in(DisplayLanguage::French, vec![])
        );
    }
}
