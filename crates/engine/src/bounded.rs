use std::future::{Future, pending};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::{Instant, sleep_until};

/// Stops the clock of a [`bounded`] operation while something the operation
/// does not control is in progress: a question to the user, or a step that
/// has a bound of its own.
#[derive(Debug)]
pub(crate) struct DeadlinePause {
    active_pauses: AtomicUsize,
    resumed: watch::Sender<()>,
}

impl Default for DeadlinePause {
    fn default() -> Self {
        Self {
            active_pauses: AtomicUsize::new(0),
            resumed: watch::channel(()).0,
        }
    }
}

impl DeadlinePause {
    /// Pauses the clock until the returned guard is dropped.
    pub(crate) fn pause(&self) -> PauseGuard<'_> {
        self.active_pauses.fetch_add(1, Ordering::SeqCst);
        PauseGuard { pause: self }
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.active_pauses.load(Ordering::SeqCst) > 0
    }
}

/// Keeps a [`DeadlinePause`] paused for as long as it lives.
#[derive(Debug)]
pub(crate) struct PauseGuard<'pause> {
    pause: &'pause DeadlinePause,
}

impl Drop for PauseGuard<'_> {
    fn drop(&mut self) {
        self.pause.active_pauses.fetch_sub(1, Ordering::SeqCst);
        self.pause.resumed.send_replace(());
    }
}

/// The operation did not finish within its limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimeLimitExceeded;

/// Runs `operation`, giving up after `limit` of unpaused time.
///
/// While `pause` is paused the clock does not run, and it starts over with a
/// full `limit` each time the pause ends.
pub(crate) async fn bounded<Outcome>(
    operation: impl Future<Output = Outcome>,
    limit: Duration,
    pause: &DeadlinePause,
) -> Result<Outcome, TimeLimitExceeded> {
    tokio::pin!(operation);
    let mut resumed = pause.resumed.subscribe();
    let mut deadline = Instant::now() + limit;
    loop {
        tokio::select! {
            outcome = &mut operation => return Ok(outcome),
            () = sleep_until(deadline) => {
                if !pause.is_paused() {
                    return Err(TimeLimitExceeded);
                }
                deadline = Instant::now() + limit;
            }
            () = pause_ended(&mut resumed) => {
                if !pause.is_paused() {
                    deadline = Instant::now() + limit;
                }
            }
        }
    }
}

async fn pause_ended(resumed: &mut watch::Receiver<()>) {
    if resumed.changed().await.is_err() {
        pending::<()>().await;
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;

    use tokio::time::sleep;

    use super::*;

    const SHORT_LIMIT: Duration = Duration::from_millis(50);

    #[tokio::test]
    async fn should_time_out_when_the_operation_never_completes() {
        let pause = DeadlinePause::default();
        let outcome = bounded(pending::<()>(), SHORT_LIMIT, &pause).await;
        assert_eq!(outcome, Err(TimeLimitExceeded));
    }

    #[tokio::test]
    async fn should_not_time_out_when_the_operation_completes_in_time() {
        let pause = DeadlinePause::default();
        let outcome = bounded(async { 7 }, SHORT_LIMIT, &pause).await;
        assert_eq!(outcome, Ok(7));
    }

    #[tokio::test]
    async fn should_not_time_out_when_paused_past_the_limit() {
        let pause = DeadlinePause::default();
        let slow_decision = async {
            let _guard = pause.pause();
            sleep(SHORT_LIMIT * 3).await;
            7
        };
        let outcome = bounded(slow_decision, SHORT_LIMIT, &pause).await;
        assert_eq!(outcome, Ok(7));
    }

    #[tokio::test]
    async fn should_restart_the_clock_when_the_pause_ends() {
        let pause = DeadlinePause::default();
        let stalls_after_pause = async {
            {
                let _guard = pause.pause();
                sleep(SHORT_LIMIT * 3).await;
            }
            sleep(SHORT_LIMIT / 2).await;
            7
        };
        let outcome = bounded(stalls_after_pause, SHORT_LIMIT, &pause).await;
        assert_eq!(outcome, Ok(7));
    }

    #[tokio::test]
    async fn should_time_out_when_the_operation_stalls_after_a_pause() {
        let pause = DeadlinePause::default();
        let stalls_after_pause = async {
            drop(pause.pause());
            pending::<()>().await;
        };
        let outcome = bounded(stalls_after_pause, SHORT_LIMIT, &pause).await;
        assert_eq!(outcome, Err(TimeLimitExceeded));
    }

    #[test]
    fn should_not_be_paused_when_nothing_paused_it() {
        assert!(!DeadlinePause::default().is_paused());
    }

    #[test]
    fn should_stay_paused_when_one_of_two_guards_is_dropped() {
        let pause = DeadlinePause::default();
        let first = pause.pause();
        let _second = pause.pause();
        drop(first);
        assert!(pause.is_paused());
    }

    #[test]
    fn should_not_be_paused_when_the_guard_is_dropped() {
        let pause = DeadlinePause::default();
        drop(pause.pause());
        assert!(!pause.is_paused());
    }
}
