//! An opener that launches nothing, for the unit tests.

use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;

use super::{UrlOpenError, UrlOpener};

/// Records the addresses instead of opening them.
pub(crate) struct RecordingUrlOpener {
    opened: Mutex<Vec<String>>,
    is_failing: bool,
}

impl RecordingUrlOpener {
    pub(crate) fn succeeding() -> Arc<Self> {
        Arc::new(Self {
            opened: Mutex::new(Vec::new()),
            is_failing: false,
        })
    }

    pub(crate) fn failing() -> Arc<Self> {
        Arc::new(Self {
            opened: Mutex::new(Vec::new()),
            is_failing: true,
        })
    }

    pub(crate) fn opened(&self) -> Vec<String> {
        self.opened
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl UrlOpener for RecordingUrlOpener {
    async fn open(&self, url: &str) -> Result<(), UrlOpenError> {
        self.opened
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(url.to_owned());
        if self.is_failing {
            return Err(UrlOpenError {
                target: url.to_owned(),
                source: io::Error::from(io::ErrorKind::NotFound),
            });
        }
        Ok(())
    }
}
