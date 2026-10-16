use std::panic::resume_unwind;

/// Runs blocking work (file I/O, key derivation) off the async runtime.
///
/// A panic in the work is propagated to the caller.
pub(crate) async fn run_blocking<Output>(work: impl FnOnce() -> Output + Send + 'static) -> Output
where
    Output: Send + 'static,
{
    match tokio::task::spawn_blocking(work).await {
        Ok(output) => output,
        Err(error) => match error.try_into_panic() {
            Ok(payload) => resume_unwind(payload),
            Err(cancelled) => panic!("a blocking task was cancelled: {cancelled}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_return_the_output_when_the_work_completes() {
        assert_eq!(run_blocking(|| 21 * 2).await, 42);
    }
}
