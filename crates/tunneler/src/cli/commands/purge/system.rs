//! The real file system and daemon behind a purge.

use std::fs;
use std::io;
use std::path::Path;

use async_trait::async_trait;
use ipc::Endpoint;

use super::super::quit::stop_daemon;
use super::execution::{DaemonControl, Filesystem};
use crate::cli::CliError;

/// The file system of the machine.
pub struct SystemFilesystem;

impl Filesystem for SystemFilesystem {
    fn exists(&self, path: &Path) -> bool {
        path.symlink_metadata().is_ok()
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        ignore_missing(fs::remove_file(path))
    }

    fn remove_directory_all(&self, path: &Path) -> io::Result<()> {
        ignore_missing(fs::remove_dir_all(path))
    }

    fn remove_directory_if_empty(&self, path: &Path) -> io::Result<()> {
        match fs::remove_dir(path) {
            Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(()),
            outcome => ignore_missing(outcome),
        }
    }
}

fn ignore_missing(outcome: io::Result<()>) -> io::Result<()> {
    match outcome {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// The daemon listening on an endpoint.
pub struct EndpointDaemon<'a> {
    /// Where the daemon listens.
    pub endpoint: &'a Endpoint,
}

#[async_trait]
impl DaemonControl for EndpointDaemon<'_> {
    async fn is_running(&self) -> bool {
        self.endpoint.is_daemon_alive().await
    }

    async fn stop(&self) -> Result<(), CliError> {
        stop_daemon(self.endpoint).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_remove_a_directory_with_its_content() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("logs");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("tunneler.log"), "line").unwrap();

        SystemFilesystem.remove_directory_all(&target).unwrap();

        assert!(!target.exists());
    }

    #[test]
    fn should_succeed_when_the_file_to_remove_is_already_gone() {
        let directory = tempfile::tempdir().unwrap();

        let outcome = SystemFilesystem.remove_file(&directory.path().join("missing.toml"));

        assert!(outcome.is_ok());
    }

    #[test]
    fn should_succeed_when_the_directory_to_remove_is_already_gone() {
        let directory = tempfile::tempdir().unwrap();

        let outcome = SystemFilesystem.remove_directory_all(&directory.path().join("missing"));

        assert!(outcome.is_ok());
    }

    #[test]
    fn should_remove_an_empty_directory_when_asked_to_remove_it_if_empty() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("tunneler");
        fs::create_dir(&target).unwrap();

        SystemFilesystem.remove_directory_if_empty(&target).unwrap();

        assert!(!target.exists());
    }

    #[test]
    fn should_keep_a_directory_that_still_has_content_when_asked_to_remove_it_if_empty() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("tunneler");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("tunnels.toml"), "version = 1").unwrap();

        let outcome = SystemFilesystem.remove_directory_if_empty(&target);

        assert!(outcome.is_ok());
        assert!(target.join("tunnels.toml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn should_see_a_broken_symlink_as_existing() {
        let directory = tempfile::tempdir().unwrap();
        let link = directory.path().join("link");
        std::os::unix::fs::symlink(directory.path().join("nowhere"), &link).unwrap();

        assert!(SystemFilesystem.exists(&link));
    }
}
