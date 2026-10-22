//! Platforms without a notification or sound backend signal nothing.

use async_trait::async_trait;
use engine::StandbySound;

use super::{DesktopNotifier, SoundEvent, SoundPlayer};

pub(crate) struct SystemNotifier;

#[async_trait]
impl DesktopNotifier for SystemNotifier {
    async fn notify(&self, _summary: &str, _body: &str) {}
}

pub(crate) struct SystemSoundPlayer;

impl SoundPlayer for SystemSoundPlayer {
    fn play(&self, _sound: StandbySound, _event: SoundEvent) {}
}
