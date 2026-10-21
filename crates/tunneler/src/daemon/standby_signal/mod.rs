//! Tells the user that a standby tunnel woke up or could not.
//!
//! [`spawn_standby_signal_follower`] listens to the engine events
//! `TunnelWoke` and `TunnelWakeFailed` and, according to
//! `settings.standby_notify`, shows a silent desktop notification, plays a
//! sound, or both. The sound is played by the daemon itself, so it stays
//! audible when the system notifications are turned off; the notification is
//! therefore always silent. Failures to signal are logged, never fatal.

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) use linux::{SystemNotifier, SystemSoundPlayer};
#[cfg(not(any(all(unix, not(target_os = "macos")), windows)))]
mod unsupported;
#[cfg(not(any(all(unix, not(target_os = "macos")), windows)))]
pub(crate) use unsupported::{SystemNotifier, SystemSoundPlayer};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::{SystemNotifier, SystemSoundPlayer};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use engine::{EngineEvent, FailureReason, StandbyNotify, TunnelId};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

/// The least time between two wake-failure signals of one tunnel: a browser
/// that retries would otherwise signal on every attempt.
const WAKE_FAILURE_THROTTLE: Duration = Duration::from_secs(30);

const WOKE_SUMMARY: &str = "Tunnel sorti de veille";
const WAKE_FAILED_SUMMARY: &str = "Réveil impossible";

/// What happened to a standby tunnel; also picks the sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundEvent {
    /// The tunnel left standby.
    Woke,
    /// The tunnel could not leave standby.
    WakeFailed,
}

/// Shows a notification on the desktop. The notification is always silent.
#[async_trait]
pub(crate) trait DesktopNotifier: Send + Sync {
    /// Shows `summary` and `body`. A failure is logged by the implementation.
    async fn notify(&self, summary: &str, body: &str);
}

/// Plays a short sound without waiting for it to end.
pub(crate) trait SoundPlayer: Send + Sync {
    /// Starts the sound of `event`. A failure is logged by the implementation.
    fn play(&self, event: SoundEvent);
}

/// What a mode asks for when an event happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Signal {
    pub(crate) shows_notification: bool,
    pub(crate) sound: Option<SoundEvent>,
}

/// The signal `mode` asks for when `event` happens.
pub(crate) fn signal_for(mode: StandbyNotify, event: SoundEvent) -> Signal {
    let (shows_notification, plays_sound) = match mode {
        StandbyNotify::None => (false, false),
        StandbyNotify::Visual => (true, false),
        StandbyNotify::Sound => (false, true),
        StandbyNotify::Both => (true, true),
    };
    Signal {
        shows_notification,
        sound: plays_sound.then_some(event),
    }
}

/// What happened to which tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Occurrence {
    Woke {
        tunnel: TunnelId,
        label: String,
    },
    WakeFailed {
        tunnel: TunnelId,
        label: String,
        reason: String,
    },
}

impl Occurrence {
    fn tunnel(&self) -> TunnelId {
        match self {
            Self::Woke { tunnel, .. } | Self::WakeFailed { tunnel, .. } => *tunnel,
        }
    }

    fn event(&self) -> SoundEvent {
        match self {
            Self::Woke { .. } => SoundEvent::Woke,
            Self::WakeFailed { .. } => SoundEvent::WakeFailed,
        }
    }

    /// The summary and the body of the notification.
    fn text(&self) -> (String, String) {
        match self {
            Self::Woke { label, .. } => (WOKE_SUMMARY.to_owned(), label.clone()),
            Self::WakeFailed { label, reason, .. } => (
                WAKE_FAILED_SUMMARY.to_owned(),
                format!("{label} : {reason}"),
            ),
        }
    }
}

/// Lets one wake-failure signal per tunnel through in each throttle period.
#[derive(Default)]
struct WakeFailureThrottle {
    last_signal: HashMap<TunnelId, Instant>,
}

impl WakeFailureThrottle {
    /// Whether a failure of `tunnel` at `now` is signalled; records it if so.
    fn allows(&mut self, tunnel: TunnelId, now: Instant) -> bool {
        let is_recent = self
            .last_signal
            .get(&tunnel)
            .is_some_and(|last| now.saturating_duration_since(*last) < WAKE_FAILURE_THROTTLE);
        if is_recent {
            return false;
        }
        self.last_signal.insert(tunnel, now);
        true
    }
}

/// Turns occurrences into notifications and sounds.
pub(crate) struct StandbySignals {
    notifier: Arc<dyn DesktopNotifier>,
    sound_player: Arc<dyn SoundPlayer>,
    throttle: WakeFailureThrottle,
}

impl StandbySignals {
    pub(crate) fn new(
        notifier: Arc<dyn DesktopNotifier>,
        sound_player: Arc<dyn SoundPlayer>,
    ) -> Self {
        Self {
            notifier,
            sound_player,
            throttle: WakeFailureThrottle::default(),
        }
    }

    /// Signals `occurrence` the way `mode` asks, at `now` for the throttle.
    pub(crate) async fn signal(
        &mut self,
        mode: StandbyNotify,
        occurrence: &Occurrence,
        now: Instant,
    ) {
        let signal = signal_for(mode, occurrence.event());
        if signal == signal_for(StandbyNotify::None, occurrence.event()) {
            return;
        }
        let is_throttled = matches!(occurrence, Occurrence::WakeFailed { .. })
            && !self.throttle.allows(occurrence.tunnel(), now);
        if is_throttled {
            return;
        }
        if let Some(sound) = signal.sound {
            self.sound_player.play(sound);
        }
        if signal.shows_notification {
            let (summary, body) = occurrence.text();
            self.notifier.notify(&summary, &body).await;
        }
    }
}

/// Signals the wake-ups and wake failures of the engine's events until
/// `shutdown` is cancelled or the engine's bus closes. `mode` and `label_of`
/// are read for each event, so a changed setting applies at once.
pub(crate) fn spawn_standby_signal_follower(
    mut events: broadcast::Receiver<EngineEvent>,
    mode: impl Fn() -> StandbyNotify + Send + Sync + 'static,
    label_of: impl Fn(TunnelId) -> Option<String> + Send + Sync + 'static,
    mut signals: StandbySignals,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                () = shutdown.cancelled() => return,
                event = events.recv() => match event {
                    Ok(event) => {
                        if let Some(occurrence) = occurrence_of(&event, &label_of) {
                            signals.signal(mode(), &occurrence, Instant::now()).await;
                        }
                    }
                    Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => return,
                },
            }
        }
    })
}

fn occurrence_of(
    event: &EngineEvent,
    label_of: &impl Fn(TunnelId) -> Option<String>,
) -> Option<Occurrence> {
    match event {
        EngineEvent::TunnelWoke { id } => Some(Occurrence::Woke {
            tunnel: *id,
            label: label_of(*id).unwrap_or_else(|| id.to_string()),
        }),
        EngineEvent::TunnelWakeFailed { id, reason } => Some(Occurrence::WakeFailed {
            tunnel: *id,
            label: label_of(*id).unwrap_or_else(|| id.to_string()),
            reason: describe(reason),
        }),
        _ => None,
    }
}

fn describe(reason: &FailureReason) -> String {
    reason.to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct FakeNotifier {
        shown: Mutex<Vec<(String, String)>>,
    }

    #[async_trait]
    impl DesktopNotifier for FakeNotifier {
        async fn notify(&self, summary: &str, body: &str) {
            self.shown
                .lock()
                .unwrap()
                .push((summary.to_owned(), body.to_owned()));
        }
    }

    #[derive(Default)]
    struct FakeSoundPlayer {
        played: Mutex<Vec<SoundEvent>>,
    }

    impl SoundPlayer for FakeSoundPlayer {
        fn play(&self, event: SoundEvent) {
            self.played.lock().unwrap().push(event);
        }
    }

    struct Fixture {
        notifier: Arc<FakeNotifier>,
        sound_player: Arc<FakeSoundPlayer>,
        signals: StandbySignals,
    }

    fn fixture() -> Fixture {
        let notifier = Arc::new(FakeNotifier::default());
        let sound_player = Arc::new(FakeSoundPlayer::default());
        let signals = StandbySignals::new(notifier.clone(), sound_player.clone());
        Fixture {
            notifier,
            sound_player,
            signals,
        }
    }

    fn woke(tunnel: TunnelId) -> Occurrence {
        Occurrence::Woke {
            tunnel,
            label: "3333 → nova:3333".to_owned(),
        }
    }

    fn wake_failed(tunnel: TunnelId) -> Occurrence {
        Occurrence::WakeFailed {
            tunnel,
            label: "3333 → nova:3333".to_owned(),
            reason: "authentication failed".to_owned(),
        }
    }

    #[test]
    fn should_signal_nothing_when_the_mode_is_none() {
        assert_eq!(
            signal_for(StandbyNotify::None, SoundEvent::Woke),
            Signal {
                shows_notification: false,
                sound: None
            }
        );
    }

    #[test]
    fn should_only_notify_when_the_mode_is_visual() {
        assert_eq!(
            signal_for(StandbyNotify::Visual, SoundEvent::WakeFailed),
            Signal {
                shows_notification: true,
                sound: None
            }
        );
    }

    #[test]
    fn should_only_play_the_sound_of_the_event_when_the_mode_is_sound() {
        assert_eq!(
            signal_for(StandbyNotify::Sound, SoundEvent::WakeFailed),
            Signal {
                shows_notification: false,
                sound: Some(SoundEvent::WakeFailed)
            }
        );
    }

    #[test]
    fn should_notify_and_play_the_sound_when_the_mode_is_both() {
        assert_eq!(
            signal_for(StandbyNotify::Both, SoundEvent::Woke),
            Signal {
                shows_notification: true,
                sound: Some(SoundEvent::Woke)
            }
        );
    }

    #[tokio::test]
    async fn should_name_the_tunnel_in_the_notification_when_it_woke() {
        let mut fixture = fixture();

        fixture
            .signals
            .signal(
                StandbyNotify::Visual,
                &woke(TunnelId::generate()),
                Instant::now(),
            )
            .await;

        assert_eq!(
            *fixture.notifier.shown.lock().unwrap(),
            vec![(
                "Tunnel sorti de veille".to_owned(),
                "3333 → nova:3333".to_owned()
            )]
        );
    }

    #[tokio::test]
    async fn should_give_the_reason_in_the_notification_when_the_wake_up_failed() {
        let mut fixture = fixture();

        fixture
            .signals
            .signal(
                StandbyNotify::Visual,
                &wake_failed(TunnelId::generate()),
                Instant::now(),
            )
            .await;

        assert_eq!(
            *fixture.notifier.shown.lock().unwrap(),
            vec![(
                "Réveil impossible".to_owned(),
                "3333 → nova:3333 : authentication failed".to_owned()
            )]
        );
    }

    #[tokio::test]
    async fn should_not_notify_when_the_mode_is_sound() {
        let mut fixture = fixture();

        fixture
            .signals
            .signal(
                StandbyNotify::Sound,
                &woke(TunnelId::generate()),
                Instant::now(),
            )
            .await;

        assert!(fixture.notifier.shown.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn should_play_the_failure_sound_when_the_wake_up_failed_and_the_mode_is_sound() {
        let mut fixture = fixture();

        fixture
            .signals
            .signal(
                StandbyNotify::Sound,
                &wake_failed(TunnelId::generate()),
                Instant::now(),
            )
            .await;

        assert_eq!(
            *fixture.sound_player.played.lock().unwrap(),
            vec![SoundEvent::WakeFailed]
        );
    }

    #[tokio::test]
    async fn should_signal_nothing_when_the_mode_is_none_whatever_happens() {
        let mut fixture = fixture();
        let tunnel = TunnelId::generate();

        fixture
            .signals
            .signal(StandbyNotify::None, &woke(tunnel), Instant::now())
            .await;
        fixture
            .signals
            .signal(StandbyNotify::None, &wake_failed(tunnel), Instant::now())
            .await;

        assert!(
            fixture.notifier.shown.lock().unwrap().is_empty()
                && fixture.sound_player.played.lock().unwrap().is_empty()
        );
    }

    #[tokio::test]
    async fn should_signal_a_second_wake_failure_of_a_tunnel_only_after_the_throttle_period() {
        let mut fixture = fixture();
        let tunnel = TunnelId::generate();
        let start = Instant::now();

        for offset in [0, 10, 29, 30] {
            fixture
                .signals
                .signal(
                    StandbyNotify::Visual,
                    &wake_failed(tunnel),
                    start + Duration::from_secs(offset),
                )
                .await;
        }

        assert_eq!(fixture.notifier.shown.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn should_throttle_each_tunnel_on_its_own() {
        let mut fixture = fixture();
        let now = Instant::now();

        for _ in 0..2 {
            fixture
                .signals
                .signal(
                    StandbyNotify::Visual,
                    &wake_failed(TunnelId::generate()),
                    now,
                )
                .await;
        }

        assert_eq!(fixture.notifier.shown.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn should_not_throttle_the_wake_ups() {
        let mut fixture = fixture();
        let tunnel = TunnelId::generate();
        let now = Instant::now();

        for _ in 0..3 {
            fixture
                .signals
                .signal(StandbyNotify::Visual, &woke(tunnel), now)
                .await;
        }

        assert_eq!(fixture.notifier.shown.lock().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn should_signal_the_engine_events_when_following_them() {
        let (sender, receiver) = broadcast::channel(8);
        let fixture = fixture();
        let notifier = fixture.notifier.clone();
        let tunnel = TunnelId::generate();
        let shutdown = CancellationToken::new();
        let task = spawn_standby_signal_follower(
            receiver,
            || StandbyNotify::Visual,
            |_| Some("3333 → nova:3333".to_owned()),
            fixture.signals,
            shutdown.clone(),
        );

        sender.send(EngineEvent::TunnelWoke { id: tunnel }).unwrap();
        drop(sender);
        task.await.unwrap();

        assert_eq!(
            *notifier.shown.lock().unwrap(),
            vec![(
                "Tunnel sorti de veille".to_owned(),
                "3333 → nova:3333".to_owned()
            )]
        );
    }

    #[tokio::test]
    async fn should_use_the_tunnel_id_as_label_when_the_tunnel_is_gone() {
        let (sender, receiver) = broadcast::channel(8);
        let fixture = fixture();
        let notifier = fixture.notifier.clone();
        let tunnel = TunnelId::generate();
        let task = spawn_standby_signal_follower(
            receiver,
            || StandbyNotify::Visual,
            |_| None,
            fixture.signals,
            CancellationToken::new(),
        );

        sender.send(EngineEvent::TunnelWoke { id: tunnel }).unwrap();
        drop(sender);
        task.await.unwrap();

        assert_eq!(notifier.shown.lock().unwrap()[0].1, tunnel.to_string());
    }
}
