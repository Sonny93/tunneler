//! Linux: the notification goes through `org.freedesktop.Notifications` and
//! the sound through the first sound player found on the machine. The
//! synthesized sounds go to the standard input of a player; the sound of the
//! system is a file of the sound theme.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use engine::StandbySound;
use zbus::Connection;
use zbus::Variant;

use super::synthesis::synthesized_wav;
use super::{DesktopNotifier, SoundEvent, SoundPlayer};
use crate::daemon::launch::{spawn_detached, spawn_detached_with_input};

const NOTIFICATIONS_SERVICE: &str = "org.freedesktop.Notifications";
const NOTIFICATIONS_PATH: &str = "/org/freedesktop/Notifications";
const APPLICATION_NAME: &str = "Tunneler";
const NO_REPLACED_NOTIFICATION: u32 = 0;
const SERVER_DEFAULT_EXPIRY: i32 = -1;
const NETWORK_CATEGORY: &str = "network";
const SOUND_THEME_DIRECTORY: &str = "/usr/share/sounds/freedesktop/stereo";

const CANBERRA_PLAYER: &str = "canberra-gtk-play";
const FILE_PLAYERS: [&str; 2] = ["pw-play", "paplay"];
/// The players that read a WAV file from their standard input, with the
/// arguments that ask for it.
const STDIN_PLAYERS: [(&str, &[&str]); 2] = [("pw-play", &["-"]), ("paplay", &[])];

fn sound_id(event: SoundEvent) -> &'static str {
    match event {
        SoundEvent::Woke => "complete",
        SoundEvent::WakeFailed => "dialog-warning",
    }
}

/// A way to play a sound: a program, its arguments and the sound file it
/// needs, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SoundCandidate {
    program: &'static str,
    args: Vec<String>,
    required_file: Option<PathBuf>,
}

/// The ways to play `event`, best first.
fn sound_candidates(event: SoundEvent) -> Vec<SoundCandidate> {
    let id = sound_id(event);
    let file = Path::new(SOUND_THEME_DIRECTORY).join(format!("{id}.oga"));
    let canberra = SoundCandidate {
        program: CANBERRA_PLAYER,
        args: vec!["-i".to_owned(), id.to_owned()],
        required_file: None,
    };
    let file_players = FILE_PLAYERS.map(|program| SoundCandidate {
        program,
        args: vec![file.display().to_string()],
        required_file: Some(file.clone()),
    });
    std::iter::once(canberra).chain(file_players).collect()
}

pub(crate) struct SystemSoundPlayer {
    has_warned: AtomicBool,
}

impl SystemSoundPlayer {
    pub(crate) fn new() -> Self {
        Self {
            has_warned: AtomicBool::new(false),
        }
    }

    /// Feeds `wav` to the first player that reads its standard input; false
    /// when none can run.
    fn play_from_stdin(wav: &'static [u8]) -> bool {
        for (program, args) in STDIN_PLAYERS {
            let args: Vec<String> = args.iter().map(|argument| (*argument).to_owned()).collect();
            match spawn_detached_with_input(program, &args, wav) {
                Ok(()) => return true,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => tracing::warn!(program, %error, "cannot play the sound"),
            }
        }
        false
    }

    fn play_system_sound(&self, event: SoundEvent) {
        for candidate in sound_candidates(event) {
            match Self::try_candidate(&candidate) {
                Ok(()) => return,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    tracing::warn!(program = candidate.program, %error, "cannot play the sound");
                }
            }
        }
        if !self.has_warned.swap(true, Ordering::Relaxed) {
            tracing::warn!("no sound player found (canberra-gtk-play, pw-play, paplay)");
        }
    }

    fn try_candidate(candidate: &SoundCandidate) -> io::Result<()> {
        if let Some(file) = &candidate.required_file
            && !file.exists()
        {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        spawn_detached(candidate.program, &candidate.args)
    }
}

impl SoundPlayer for SystemSoundPlayer {
    fn play(&self, sound: StandbySound, event: SoundEvent, volume: u8) {
        if let Some(wav) = synthesized_wav(sound, event, volume)
            && Self::play_from_stdin(wav)
        {
            return;
        }
        self.play_system_sound(event);
    }
}

pub(crate) struct SystemNotifier {
    has_warned: AtomicBool,
}

impl SystemNotifier {
    pub(crate) fn new() -> Self {
        Self {
            has_warned: AtomicBool::new(false),
        }
    }
}

fn send(summary: &str, body: &str) -> zbus::Result<()> {
    zbus::block_on(send_async(summary, body))
}

async fn send_async(summary: &str, body: &str) -> zbus::Result<()> {
    let connection = Connection::session().await?;
    let hints = HashMap::from([
        ("suppress-sound", Variant::from(true)),
        ("category", Variant::from(NETWORK_CATEGORY)),
    ]);
    connection
        .call_method(
            Some(NOTIFICATIONS_SERVICE),
            NOTIFICATIONS_PATH,
            Some(NOTIFICATIONS_SERVICE),
            "Notify",
            &(
                APPLICATION_NAME,
                NO_REPLACED_NOTIFICATION,
                "",
                summary,
                body,
                Vec::<&str>::new(),
                hints,
                SERVER_DEFAULT_EXPIRY,
            ),
        )
        .await?;
    Ok(())
}

#[async_trait]
impl DesktopNotifier for SystemNotifier {
    async fn notify(&self, summary: &str, body: &str) {
        let (summary, body) = (summary.to_owned(), body.to_owned());
        let outcome = tokio::task::spawn_blocking(move || send(&summary, &body)).await;
        let error = match outcome {
            Ok(Ok(())) => return,
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };
        if !self.has_warned.swap(true, Ordering::Relaxed) {
            tracing::warn!(%error, "cannot show a desktop notification");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_try_canberra_first_with_the_event_sound_id() {
        let candidates = sound_candidates(SoundEvent::Woke);

        assert_eq!(
            (candidates[0].program, candidates[0].args.clone()),
            (
                CANBERRA_PLAYER,
                vec!["-i".to_owned(), "complete".to_owned()]
            )
        );
    }

    #[test]
    fn should_fall_back_to_pw_play_then_paplay_on_the_theme_file() {
        let candidates = sound_candidates(SoundEvent::WakeFailed);

        assert_eq!(
            candidates[1..]
                .iter()
                .map(|candidate| (candidate.program, candidate.args.clone()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "pw-play",
                    vec!["/usr/share/sounds/freedesktop/stereo/dialog-warning.oga".to_owned()]
                ),
                (
                    "paplay",
                    vec!["/usr/share/sounds/freedesktop/stereo/dialog-warning.oga".to_owned()]
                ),
            ]
        );
    }

    #[test]
    fn should_read_the_sound_from_standard_input_with_pw_play_then_paplay() {
        assert_eq!(
            STDIN_PLAYERS,
            [("pw-play", &["-"][..]), ("paplay", &[][..])]
        );
    }

    #[test]
    fn should_not_require_a_file_for_canberra() {
        assert_eq!(sound_candidates(SoundEvent::Woke)[0].required_file, None);
    }
}
