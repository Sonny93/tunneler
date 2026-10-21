//! A launcher that starts nothing, for the unit tests.

use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;

use super::{LaunchError, Launcher};

/// Records what it is asked to open instead of opening it.
pub(crate) struct RecordingLauncher {
    urls: Mutex<Vec<String>>,
    programs: Mutex<Vec<(String, Vec<String>)>>,
    is_failing: bool,
}

impl RecordingLauncher {
    pub(crate) fn succeeding() -> Arc<Self> {
        Arc::new(Self::new(false))
    }

    pub(crate) fn failing() -> Arc<Self> {
        Arc::new(Self::new(true))
    }

    fn new(is_failing: bool) -> Self {
        Self {
            urls: Mutex::new(Vec::new()),
            programs: Mutex::new(Vec::new()),
            is_failing,
        }
    }

    pub(crate) fn opened_urls(&self) -> Vec<String> {
        self.urls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn spawned_programs(&self) -> Vec<(String, Vec<String>)> {
        self.programs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn outcome(&self, target: &str) -> Result<(), LaunchError> {
        if self.is_failing {
            return Err(LaunchError {
                target: target.to_owned(),
                source: io::Error::from(io::ErrorKind::NotFound),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl Launcher for RecordingLauncher {
    async fn open_url(&self, url: &str) -> Result<(), LaunchError> {
        self.urls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(url.to_owned());
        self.outcome(url)
    }

    async fn spawn_program(&self, program: &str, args: &[String]) -> Result<(), LaunchError> {
        self.programs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((program.to_owned(), args.to_vec()));
        self.outcome(program)
    }
}
