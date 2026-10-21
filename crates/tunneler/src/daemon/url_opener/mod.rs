//! Opening an address in the user's default program.

#[cfg(test)]
#[cfg(unix)]
pub(crate) mod testing;

use std::io;

use async_trait::async_trait;
use thiserror::Error;

/// Why an address could not be opened.
#[derive(Debug, Error)]
#[error("cannot open {target}: {source}")]
pub(crate) struct UrlOpenError {
    /// What was to be opened.
    pub(crate) target: String,
    /// Why it failed.
    #[source]
    pub(crate) source: io::Error,
}

/// Hands an address to the program that opens it.
#[async_trait]
pub(crate) trait UrlOpener: Send + Sync {
    /// Opens `url` without waiting for the program to end.
    async fn open(&self, url: &str) -> Result<(), UrlOpenError>;
}

/// Opens addresses with the system default program, detached from the daemon.
pub(crate) struct SystemUrlOpener;

#[async_trait]
impl UrlOpener for SystemUrlOpener {
    async fn open(&self, url: &str) -> Result<(), UrlOpenError> {
        let target = url.to_owned();
        let outcome = tokio::task::spawn_blocking({
            let target = target.clone();
            move || open::that_detached(target)
        })
        .await
        .unwrap_or_else(|error| Err(io::Error::other(error)));
        outcome.map_err(|source| UrlOpenError { target, source })
    }
}
