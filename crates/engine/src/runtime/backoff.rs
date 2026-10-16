use std::time::Duration;

use tracing::warn;

const FALLBACK_JITTER_FRACTION: f64 = 0.5;
const FIRST_ATTEMPT: u32 = 1;

/// The delay before reconnection attempt number `attempt` (1-based):
/// `min(initial × 2^(attempt − 1), maximum)` with equal jitter, half of it
/// fixed and the other half scaled by `jitter_fraction` in `[0, 1]`.
pub(crate) fn backoff_delay(
    attempt: u32,
    initial: Duration,
    maximum: Duration,
    jitter_fraction: f64,
) -> Duration {
    let doublings = attempt.max(FIRST_ATTEMPT) - FIRST_ATTEMPT;
    let growth = 2_u32.saturating_pow(doublings);
    let ceiling = initial.saturating_mul(growth).min(maximum);
    let fixed = ceiling / 2;
    let variable = ceiling.saturating_sub(fixed);
    fixed + variable.mul_f64(sanitize_fraction(jitter_fraction))
}

fn sanitize_fraction(fraction: f64) -> f64 {
    if fraction.is_nan() {
        return 0.0;
    }
    fraction.clamp(0.0, 1.0)
}

/// A uniformly random fraction in `[0, 1)`.
///
/// Falls back to the middle of the range when the system has no entropy, so
/// that reconnection keeps working.
pub(crate) fn random_jitter_fraction() -> f64 {
    match getrandom::u32() {
        Ok(value) => f64::from(value) / (f64::from(u32::MAX) + 1.0),
        Err(error) => {
            warn!(%error, "no entropy for the reconnection jitter");
            FALLBACK_JITTER_FRACTION
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INITIAL: Duration = Duration::from_millis(1000);
    const MAXIMUM: Duration = Duration::from_millis(60_000);

    fn delay(attempt: u32, jitter_fraction: f64) -> Duration {
        backoff_delay(attempt, INITIAL, MAXIMUM, jitter_fraction)
    }

    #[test]
    fn should_wait_half_the_initial_delay_when_the_jitter_is_zero_on_the_first_attempt() {
        assert_eq!(delay(1, 0.0), Duration::from_millis(500));
    }

    #[test]
    fn should_wait_the_initial_delay_when_the_jitter_is_one_on_the_first_attempt() {
        assert_eq!(delay(1, 1.0), INITIAL);
    }

    #[test]
    fn should_double_the_ceiling_when_the_attempt_increases() {
        assert_eq!(delay(2, 1.0), Duration::from_millis(2000));
        assert_eq!(delay(3, 1.0), Duration::from_millis(4000));
    }

    #[test]
    fn should_stop_growing_when_the_maximum_is_reached() {
        assert_eq!(delay(10, 1.0), MAXIMUM);
    }

    #[test]
    fn should_keep_half_the_maximum_when_the_jitter_is_zero_at_the_cap() {
        assert_eq!(delay(10, 0.0), MAXIMUM / 2);
    }

    #[test]
    fn should_not_overflow_when_the_attempt_is_huge() {
        assert_eq!(delay(u32::MAX, 1.0), MAXIMUM);
    }

    #[test]
    fn should_treat_attempt_zero_like_the_first_attempt() {
        assert_eq!(delay(0, 1.0), delay(1, 1.0));
    }

    #[test]
    fn should_clamp_the_jitter_when_it_is_out_of_range() {
        assert_eq!(delay(1, 7.0), delay(1, 1.0));
        assert_eq!(delay(1, -3.0), delay(1, 0.0));
    }

    #[test]
    fn should_not_panic_when_the_jitter_is_not_a_number() {
        assert_eq!(delay(1, f64::NAN), delay(1, 0.0));
    }

    #[test]
    fn should_use_the_maximum_when_it_is_below_the_initial_delay() {
        let capped = backoff_delay(1, INITIAL, Duration::from_millis(100), 1.0);

        assert_eq!(capped, Duration::from_millis(100));
    }

    #[test]
    fn should_return_a_fraction_below_one_when_drawing_a_random_jitter() {
        let fraction = random_jitter_fraction();

        assert!((0.0..1.0).contains(&fraction));
    }
}
