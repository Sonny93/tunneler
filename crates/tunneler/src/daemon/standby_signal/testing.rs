//! A sound player that plays nothing, for the unit tests.

use std::sync::Mutex;

use engine::StandbySound;

use super::{SoundEvent, SoundPlayer};

/// Records the sounds it is asked to play instead of playing them.
#[derive(Default)]
pub(crate) struct RecordingSoundPlayer {
    pub(crate) played: Mutex<Vec<(StandbySound, SoundEvent)>>,
}

impl SoundPlayer for RecordingSoundPlayer {
    fn play(&self, sound: StandbySound, event: SoundEvent) {
        self.played.lock().unwrap().push((sound, event));
    }
}
