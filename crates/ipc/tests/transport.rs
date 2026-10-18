#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;

use ipc::{BindError, Endpoint, IpcServer};
use pretty_assertions::assert_eq;
use support::{TestDaemon, private_tempdir};

#[tokio::test]
async fn should_fail_with_address_in_use_when_a_second_server_binds() {
    let daemon = TestDaemon::start();

    let error = IpcServer::bind(&daemon.endpoint).err().unwrap();

    assert!(matches!(error, BindError::AddressInUse));
}

#[tokio::test]
async fn should_find_the_daemon_alive_when_a_server_serves() {
    let daemon = TestDaemon::start();

    assert!(daemon.endpoint.is_daemon_alive().await);
}

#[tokio::test]
async fn should_find_no_daemon_when_nothing_was_ever_bound() {
    let directory = private_tempdir();
    let endpoint = Endpoint::from_path(directory.path().join("tunneler.sock"));

    assert!(!endpoint.is_daemon_alive().await);
}

#[tokio::test]
async fn should_find_no_daemon_when_the_server_shut_down() {
    let daemon = TestDaemon::start();

    daemon.shutdown.cancel();
    daemon.task.await.unwrap();

    assert!(!daemon.endpoint.is_daemon_alive().await);
}

#[tokio::test]
async fn should_restrict_the_socket_to_its_owner() {
    let daemon = TestDaemon::start();

    let mode = std::fs::metadata(daemon.endpoint.socket_path().unwrap())
        .unwrap()
        .permissions()
        .mode();

    assert_eq!(mode & 0o777, 0o600);
}

fn leave_stale_socket(endpoint: &Endpoint) {
    drop(UnixListener::bind(endpoint.socket_path().unwrap()).unwrap());
}

#[tokio::test]
async fn should_fail_with_address_in_use_when_a_stale_socket_is_left_behind() {
    let directory = private_tempdir();
    let endpoint = Endpoint::from_path(directory.path().join("tunneler.sock"));
    leave_stale_socket(&endpoint);

    let error = IpcServer::bind(&endpoint).err().unwrap();

    assert!(matches!(error, BindError::AddressInUse));
}

#[tokio::test]
async fn should_find_no_daemon_when_a_stale_socket_is_left_behind() {
    let directory = private_tempdir();
    let endpoint = Endpoint::from_path(directory.path().join("tunneler.sock"));
    leave_stale_socket(&endpoint);

    assert!(!endpoint.is_daemon_alive().await);
}

#[tokio::test]
async fn should_bind_again_when_a_stale_socket_was_reclaimed() {
    let directory = private_tempdir();
    let endpoint = Endpoint::from_path(directory.path().join("tunneler.sock"));
    leave_stale_socket(&endpoint);

    let reclaimed = endpoint.reclaim_stale().unwrap();

    assert!(reclaimed && IpcServer::bind(&endpoint).is_ok());
}

#[tokio::test]
async fn should_not_reclaim_the_socket_when_a_daemon_is_alive() {
    let daemon = TestDaemon::start();

    let reclaimed = daemon.endpoint.reclaim_stale().unwrap();

    assert!(!reclaimed && daemon.endpoint.is_daemon_alive().await);
}

#[tokio::test]
async fn should_bind_again_when_the_previous_server_was_dropped() {
    let directory = private_tempdir();
    let endpoint = Endpoint::from_path(directory.path().join("tunneler.sock"));
    drop(IpcServer::bind(&endpoint).unwrap());

    assert!(IpcServer::bind(&endpoint).is_ok());
}
