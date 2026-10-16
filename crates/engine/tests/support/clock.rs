use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use engine::Clock;

const START_SECONDS: u64 = 1_700_000_000;

/// A clock frozen at a known instant.
pub struct ManualClock {
    now: Mutex<SystemTime>,
}

impl ManualClock {
    pub fn new() -> Self {
        Self {
            now: Mutex::new(UNIX_EPOCH + Duration::from_secs(START_SECONDS)),
        }
    }
}

impl Clock for ManualClock {
    fn now(&self) -> SystemTime {
        *self.now.lock().unwrap()
    }
}
