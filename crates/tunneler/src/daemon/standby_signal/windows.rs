//! Windows: a silent toast from the application identity of Tunneler (the one
//! of PowerShell when it cannot be registered), and the system sounds through
//! `PlaySoundW`, which also plays the synthesized sounds from memory.

use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use engine::StandbySound;
use tauri_winrt_notification::Toast;

use super::synthesis::synthesized_wav;
use super::{DesktopNotifier, SoundEvent, SoundPlayer};
use crate::daemon::notification_identity::{
    ToastIdentity, register_for_toasts, system_notification_identity,
};
use crate::layout::APPLICATION_IDENTIFIER;

fn sound_alias(event: SoundEvent) -> &'static str {
    match event {
        SoundEvent::Woke => "SystemNotification",
        SoundEvent::WakeFailed => "SystemExclamation",
    }
}

pub(crate) struct SystemNotifier {
    app_id: &'static str,
    has_warned: AtomicBool,
}

impl SystemNotifier {
    /// Registers the notification identity of the daemon, which is idempotent,
    /// and shows toasts under it, or under PowerShell's when that fails.
    pub(crate) fn new() -> Self {
        Self {
            app_id: app_id(register_for_toasts(system_notification_identity().as_ref())),
            has_warned: AtomicBool::new(false),
        }
    }
}

fn app_id(identity: ToastIdentity) -> &'static str {
    match identity {
        ToastIdentity::Tunneler => APPLICATION_IDENTIFIER,
        ToastIdentity::PowerShell => Toast::POWERSHELL_APP_ID,
    }
}

fn show(app_id: &str, summary: &str, body: &str) -> tauri_winrt_notification::Result<()> {
    Toast::new(app_id)
        .title(summary)
        .text1(body)
        .sound(None)
        .show()
}

#[async_trait]
impl DesktopNotifier for SystemNotifier {
    async fn notify(&self, summary: &str, body: &str) {
        let (app_id, summary, body) = (self.app_id, summary.to_owned(), body.to_owned());
        let outcome = tokio::task::spawn_blocking(move || show(app_id, &summary, &body)).await;
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

pub(crate) struct SystemSoundPlayer {
    has_warned: AtomicBool,
}

impl SystemSoundPlayer {
    pub(crate) fn new() -> Self {
        Self {
            has_warned: AtomicBool::new(false),
        }
    }
}

impl SoundPlayer for SystemSoundPlayer {
    fn play(&self, sound: StandbySound, event: SoundEvent, volume: u8) {
        let is_started = match synthesized_wav(sound, event, volume) {
            Some(wav) => play_wav(wav),
            None => play_alias(sound_alias(event)),
        };
        if !is_started && !self.has_warned.swap(true, Ordering::Relaxed) {
            tracing::warn!("cannot play the sound");
        }
    }
}

#[allow(unsafe_code)]
fn play_alias(alias: &str) -> bool {
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};

    let wide_alias: Vec<u16> = alias.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: `wide_alias` is a NUL-terminated UTF-16 string that lives past
    // the call, a system alias needs no module handle, and SND_ALIAS makes the
    // call read the string as a registry alias, not as a buffer to keep.
    unsafe {
        PlaySoundW(
            wide_alias.as_ptr(),
            std::ptr::null_mut(),
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
        ) != 0
    }
}

/// Starts `wav` and returns without waiting for it to end.
#[allow(unsafe_code)]
fn play_wav(wav: &'static [u8]) -> bool {
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

    // SAFETY: with SND_MEMORY the first argument points to a WAV file image
    // in memory, not to a string; `wav` is a complete one. SND_ASYNC makes
    // the call return at once while the system keeps reading the buffer, which
    // is `'static` and never freed or modified, so it outlives the playback.
    // No module handle is needed for a buffer.
    unsafe {
        PlaySoundW(
            wav.as_ptr().cast(),
            std::ptr::null_mut(),
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        ) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_show_toasts_as_tunneler_when_the_identity_is_registered() {
        assert_eq!(app_id(ToastIdentity::Tunneler), "com.minimalstuff.tunneler");
    }

    #[test]
    fn should_show_toasts_as_powershell_when_the_identity_is_not_registered() {
        assert_eq!(app_id(ToastIdentity::PowerShell), Toast::POWERSHELL_APP_ID);
    }

    #[test]
    fn should_use_the_notification_alias_when_the_tunnel_woke() {
        assert_eq!(sound_alias(SoundEvent::Woke), "SystemNotification");
    }

    #[test]
    fn should_use_the_exclamation_alias_when_the_wake_up_failed() {
        assert_eq!(sound_alias(SoundEvent::WakeFailed), "SystemExclamation");
    }
}
