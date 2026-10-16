use std::time::SystemTime;

/// Where the engine reads the wall-clock time from.
///
/// Only timestamps shown to the user (`next_retry_at`, `since`, last
/// activity, log entries) go through the clock; waiting always goes through
/// `tokio::time`.
pub trait Clock: Send + Sync {
    /// The current wall-clock time.
    fn now(&self) -> SystemTime;
}

/// The operating system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[cfg(test)]
pub(crate) use manual::ManualClock;

#[cfg(test)]
mod manual {
    use std::sync::{Mutex, PoisonError};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::Clock;

    const MANUAL_CLOCK_START_SECONDS: u64 = 1_700_000_000;

    /// A clock that only moves when told to.
    #[derive(Debug)]
    pub(crate) struct ManualClock {
        now: Mutex<SystemTime>,
    }

    impl Default for ManualClock {
        fn default() -> Self {
            Self {
                now: Mutex::new(UNIX_EPOCH + Duration::from_secs(MANUAL_CLOCK_START_SECONDS)),
            }
        }
    }

    impl ManualClock {
        pub(crate) fn advance(&self, duration: Duration) {
            let mut now = self.now.lock().unwrap_or_else(PoisonError::into_inner);
            *now += duration;
        }
    }

    impl Clock for ManualClock {
        fn now(&self) -> SystemTime {
            *self.now.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn should_not_move_when_the_manual_clock_is_not_advanced() {
        let clock = ManualClock::default();

        assert_eq!(clock.now(), clock.now());
    }

    #[test]
    fn should_move_by_the_given_duration_when_the_manual_clock_is_advanced() {
        let clock = ManualClock::default();
        let before = clock.now();

        clock.advance(Duration::from_secs(5));

        assert_eq!(clock.now(), before + Duration::from_secs(5));
    }

    #[test]
    fn should_report_a_recent_time_when_reading_the_system_clock() {
        let reading = SystemClock.now();

        assert!(
            reading
                .elapsed()
                .is_ok_and(|age| age < Duration::from_secs(5))
        );
    }
}
