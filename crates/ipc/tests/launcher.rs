#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::process::Command;
use std::time::Duration;

use ipc::{Endpoint, LaunchError, ensure_daemon_running};
use support::TestDaemon;

const SHORT_TIMEOUT: Duration = Duration::from_millis(200);
const LONG_TIMEOUT: Duration = Duration::from_secs(5);

fn dead_endpoint() -> (tempfile::TempDir, Endpoint) {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = Endpoint::from_path(directory.path().join("tunneler.sock"));
    (directory, endpoint)
}

fn shell(script: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    command
}

#[tokio::test]
async fn should_not_spawn_anything_when_a_daemon_already_answers() {
    let daemon = TestDaemon::start();
    let command = Command::new("/nonexistent/tunneler-daemon");

    let outcome = ensure_daemon_running(&daemon.endpoint, command, LONG_TIMEOUT).await;

    assert!(outcome.is_ok());
}

#[tokio::test]
async fn should_fail_with_spawn_when_the_program_does_not_exist() {
    let (_directory, endpoint) = dead_endpoint();
    let command = Command::new("/nonexistent/tunneler-daemon");

    let outcome = ensure_daemon_running(&endpoint, command, LONG_TIMEOUT).await;

    assert!(matches!(outcome, Err(LaunchError::Spawn(_))));
}

#[tokio::test]
async fn should_fail_with_exited_when_the_command_ends_right_away() {
    let (_directory, endpoint) = dead_endpoint();

    let outcome = ensure_daemon_running(&endpoint, shell("exit 3"), LONG_TIMEOUT).await;

    assert!(matches!(outcome, Err(LaunchError::Exited { status }) if status.contains('3')));
}

#[tokio::test]
async fn should_fail_with_timed_out_when_the_command_never_answers() {
    let (_directory, endpoint) = dead_endpoint();

    let outcome = ensure_daemon_running(&endpoint, shell("sleep 1"), SHORT_TIMEOUT).await;

    assert!(matches!(
        outcome,
        Err(LaunchError::TimedOut { waited }) if waited == SHORT_TIMEOUT
    ));
}
