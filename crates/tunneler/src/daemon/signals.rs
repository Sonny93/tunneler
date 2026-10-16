use std::io;

/// Completes when the process is asked to terminate: Ctrl+C, and `SIGTERM` on
/// Unix.
#[cfg(unix)]
pub(crate) async fn wait_for_termination() -> io::Result<()> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        interrupted = tokio::signal::ctrl_c() => interrupted,
        _ = terminate.recv() => Ok(()),
    }
}

/// Completes when the process is asked to terminate with Ctrl+C.
#[cfg(not(unix))]
pub(crate) async fn wait_for_termination() -> io::Result<()> {
    tokio::signal::ctrl_c().await
}
