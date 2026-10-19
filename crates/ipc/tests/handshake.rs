#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ipc::protocol::{ClientKind, ErrorCode, Hello, PROTOCOL_VERSION, Request};
use ipc::{ClientError, ConnectionNotification, Endpoint, IpcClient};
use pretty_assertions::assert_eq;
use serde_json::json;
use support::{TestDaemon, WAIT};

#[tokio::test]
async fn should_report_the_daemon_version_when_the_handshake_succeeds() {
    let daemon = TestDaemon::start();

    let client = daemon.connect(ClientKind::Cli, false).await;

    assert_eq!(client.daemon_version(), env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn should_answer_hello_with_both_versions_when_the_handshake_succeeds() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw().await;

    client
        .send_line(r#"{"id":1,"method":"hello","params":{"protocolVersion":3,"clientKind":"cli","clientVersion":"x","subscribeEvents":false,"canPrompt":false}}"#)
        .await;

    assert_eq!(
        client.read_value().await.unwrap(),
        json!({"id":1,"result":{"protocolVersion":PROTOCOL_VERSION,"daemonVersion":env!("CARGO_PKG_VERSION")}})
    );
}

#[tokio::test]
async fn should_fail_with_protocol_mismatch_when_the_versions_differ() {
    let daemon = TestDaemon::start();
    let mut hello = Hello::new(ClientKind::Ui, "test", true, true);
    hello.protocol_version = 99;

    let error = IpcClient::connect(&daemon.endpoint, hello)
        .await
        .err()
        .unwrap();

    assert!(matches!(
        error,
        ClientError::ProtocolMismatch {
            daemon: PROTOCOL_VERSION,
            client: 99
        }
    ));
}

#[tokio::test]
async fn should_close_the_connection_when_the_versions_differ() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw().await;
    let mut hello = Hello::new(ClientKind::Ui, "test", true, true);
    hello.protocol_version = 99;

    client.send_request(1, Request::Hello(hello)).await;
    let answer = client.read_value().await.unwrap();
    let after = client.read_value().await;

    assert_eq!(
        (answer["error"]["code"].clone(), after),
        (json!("protocol_mismatch"), None)
    );
}

#[tokio::test]
async fn should_answer_handshake_required_when_a_request_comes_before_hello() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw().await;

    client
        .send_line(r#"{"id":8,"method":"state.snapshot"}"#)
        .await;
    let answer = client.read_value().await.unwrap();

    assert_eq!(
        (answer["id"].clone(), answer["error"]["code"].clone()),
        (json!(8), json!("handshake_required"))
    );
}

#[tokio::test]
async fn should_close_the_connection_when_a_request_comes_before_hello() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw().await;

    client
        .send_line(r#"{"id":8,"method":"state.snapshot"}"#)
        .await;
    client.read_value().await.unwrap();

    assert_eq!(client.read_value().await, None);
}

#[tokio::test]
async fn should_refuse_a_second_hello_without_closing_the_connection() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw_after_hello(false).await;
    let hello = Hello::new(ClientKind::Ui, "test", false, false);

    client.send_request(2, Request::Hello(hello)).await;
    let answer = client.read_value().await.unwrap();
    client.send_request(3, Request::UiShow {}).await;
    let next = client.read_value().await.unwrap();

    assert_eq!(
        (answer["error"]["code"].clone(), next["id"].clone()),
        (json!("invalid_request"), json!(3))
    );
}

#[tokio::test]
async fn should_track_the_client_kind_and_prompt_ability_per_connection() {
    let daemon = TestDaemon::start();
    let hello = Hello::new(ClientKind::Ui, "1.2.3", true, true);
    let _client = IpcClient::connect(&daemon.endpoint, hello).await.unwrap();

    let connections = daemon.handle.connections();

    assert_eq!(
        connections
            .iter()
            .map(|info| (
                info.client_kind,
                info.client_version.as_str(),
                info.subscribes_to_events,
                info.can_prompt
            ))
            .collect::<Vec<_>>(),
        vec![(ClientKind::Ui, "1.2.3", true, true)]
    );
}

#[tokio::test]
#[cfg_attr(
    target_os = "macos",
    ignore = "interprocess reads peer credentials with getpeereid on macOS, which has no process id"
)]
async fn should_report_the_peer_process_when_a_client_connects() {
    let daemon = TestDaemon::start();
    let _client = daemon.connect(ClientKind::Cli, false).await;

    let connections = daemon.handle.connections();

    assert_eq!(connections[0].peer.process_id, Some(std::process::id()));
}

#[tokio::test]
async fn should_notify_when_a_connection_opens_and_closes() {
    let daemon = TestDaemon::start();
    let mut notifications = daemon.handle.subscribe_connections();

    let client = daemon.connect(ClientKind::Cli, false).await;
    let opened = tokio::time::timeout(WAIT, notifications.recv())
        .await
        .unwrap()
        .unwrap();
    drop(client);
    let closed = tokio::time::timeout(WAIT, notifications.recv())
        .await
        .unwrap()
        .unwrap();

    let ConnectionNotification::Opened(info) = opened else {
        panic!("expected an opening, got {opened:?}");
    };
    assert_eq!(closed, ConnectionNotification::Closed(info.id));
}

#[tokio::test]
async fn should_fail_with_daemon_not_running_when_nothing_listens() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = Endpoint::from_path(directory.path().join("absent.sock"));
    let hello = Hello::new(ClientKind::Cli, "test", false, false);

    let error = IpcClient::connect(&endpoint, hello).await.err().unwrap();

    assert!(matches!(error, ClientError::DaemonNotRunning));
}

#[tokio::test]
async fn should_say_invalid_request_when_the_first_line_is_malformed_and_wait_for_hello() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw().await;

    client.send_line("garbage").await;
    let rejection = client.read_value().await.unwrap();
    client
        .send_request(
            1,
            Request::Hello(Hello::new(ClientKind::Cli, "x", false, false)),
        )
        .await;
    let greeting = client.read_value().await.unwrap();

    assert_eq!(
        (rejection["error"]["code"].clone(), greeting["id"].clone()),
        (json!(ErrorCode::InvalidRequest.as_str()), json!(1))
    );
}
