//! Windows: a silent toast from the PowerShell application identity, and the
//! system sounds through `PlaySoundW`.

use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use tauri_winrt_notification::Toast;

use super::{DesktopNotifier, SoundEvent, SoundPlayer};

fn sound_alias(event: SoundEvent) -> &'static str {
    match event {
        SoundEvent::Woke => "SystemNotification",
        SoundEvent::WakeFailed => "SystemExclamation",
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

fn show(summary: &str, body: &str) -> tauri_winrt_notification::Result<()> {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title(summary)
        .text1(body)
        .sound(None)
        .show()
}

#[async_trait]
impl DesktopNotifier for SystemNotifier {
    async fn notify(&self, summary: &str, body: &str) {
        let (summary, body) = (summary.to_owned(), body.to_owned());
        let outcome = tokio::task::spawn_blocking(move || show(&summary, &body)).await;
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
    fn play(&self, event: SoundEvent) {
        if !play_alias(sound_alias(event)) && !self.has_warned.swap(true, Ordering::Relaxed) {
            tracing::warn!("cannot play the system sound");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_use_the_notification_alias_when_the_tunnel_woke() {
        assert_eq!(sound_alias(SoundEvent::Woke), "SystemNotification");
    }

    #[test]
    fn should_use_the_exclamation_alias_when_the_wake_up_failed() {
        assert_eq!(sound_alias(SoundEvent::WakeFailed), "SystemExclamation");
    }
}
