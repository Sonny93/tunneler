#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::sync::Arc;
use std::time::Instant;

use ipc::ClientError;
use ipc::protocol::{ClientKind, ErrorCode, MAX_LINE_BYTES};
use pretty_assertions::assert_eq;
use serde_json::json;
use support::{MISSING_TUNNEL_ID, SLOW_DELAY, SLOW_TUNNEL_ID, TestDaemon};
use tokio::task::JoinSet;

const CONCURRENT_CALLS: u32 = 50;

#[tokio::test]
async fn should_return_the_handler_result_when_a_request_succeeds() {
    let daemon = TestDaemon::start();
    let client = daemon.connect(ClientKind::Cli, false).await;

    let entries = client.tunnel_logs("abc").await.unwrap();

    assert_eq!(entries[0].message, "abc");
}

#[tokio::test]
async fn should_return_unit_when_a_request_has_nothing_to_report() {
    let daemon = TestDaemon::start();
    let client = daemon.connect(ClientKind::Cli, false).await;

    assert!(client.start_tunnel("abc").await.is_ok());
}

#[tokio::test]
async fn should_fail_with_the_remote_error_when_the_handler_refuses() {
    let daemon = TestDaemon::start();
    let client = daemon.connect(ClientKind::Cli, false).await;

    let error = client.delete_tunnel(MISSING_TUNNEL_ID).await.unwrap_err();

    assert!(matches!(
        error,
        ClientError::Remote {
            code: ErrorCode::NotFound,
            ..
        }
    ));
}

#[tokio::test]
async fn should_carry_the_error_data_when_the_handler_refuses_with_details() {
    let daemon = TestDaemon::start();
    let client = daemon.connect(ClientKind::Ui, false).await;

    let error = client.quit(false).await.unwrap_err();

    let ClientError::Remote { code, data, .. } = error else {
        panic!("expected a remote error, got {error:?}");
    };
    assert_eq!((code, data), (ErrorCode::TunnelsRunning, Some(json!(2))));
}

#[tokio::test]
async fn should_correlate_responses_when_calls_overlap() {
    let daemon = TestDaemon::start();
    let client = daemon.connect(ClientKind::Ui, false).await;
    let started = Instant::now();

    let (slow, fast) = tokio::join!(client.tunnel_logs(SLOW_TUNNEL_ID), async {
        let logs = client.tunnel_logs("fast").await;
        (logs, started.elapsed())
    });

    let (fast_logs, fast_elapsed) = fast;
    assert_eq!(
        (
            slow.unwrap()[0].message.clone(),
            fast_logs.unwrap()[0].message.clone(),
            fast_elapsed < SLOW_DELAY
        ),
        (SLOW_TUNNEL_ID.to_owned(), "fast".to_owned(), true)
    );
}

#[tokio::test]
async fn should_answer_every_call_when_many_overlap() {
    let daemon = TestDaemon::start();
    let client = Arc::new(daemon.connect(ClientKind::Ui, false).await);
    let mut calls = JoinSet::new();

    for index in 0..CONCURRENT_CALLS {
        let client = Arc::clone(&client);
        calls.spawn(async move {
            let entries = client.tunnel_logs(&index.to_string()).await.unwrap();
            (index, entries[0].message.clone())
        });
    }
    let mut answers = calls.join_all().await;
    answers.sort();

    assert_eq!(
        answers,
        (0..CONCURRENT_CALLS)
            .map(|index| (index, index.to_string()))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn should_answer_unknown_method_when_the_method_does_not_exist() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw_after_hello(false).await;

    client
        .send_line(r#"{"id":5,"method":"tunnel.explode"}"#)
        .await;
    let answer = client.read_value().await.unwrap();

    assert_eq!(
        (answer["id"].clone(), answer["error"]["code"].clone()),
        (json!(5), json!("unknown_method"))
    );
}

#[tokio::test]
async fn should_keep_the_connection_open_when_a_line_is_malformed() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw_after_hello(false).await;

    client.send_line("{broken").await;
    let rejection = client.read_value().await.unwrap();
    client.send_line(r#"{"id":6,"method":"ui.show"}"#).await;
    let answer = client.read_value().await.unwrap();

    assert_eq!(
        (
            rejection["id"].clone(),
            rejection["error"]["code"].clone(),
            answer["result"].clone()
        ),
        (json!(null), json!("invalid_request"), json!({}))
    );
}

#[tokio::test]
async fn should_answer_with_a_null_id_when_the_request_id_cannot_be_read() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw_after_hello(false).await;

    client
        .send_line(r#"{"id":"seven","method":"ui.show"}"#)
        .await;
    let answer = client.read_value().await.unwrap();

    assert_eq!(answer["id"], json!(null));
}

#[tokio::test]
async fn should_answer_invalid_request_and_close_when_a_line_is_too_long() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw_after_hello(false).await;

    client.send_bytes(&vec![b'a'; MAX_LINE_BYTES + 2]).await;
    let answer = client.read_value().await;

    let answer = answer.unwrap();
    assert_eq!(answer["error"]["code"], json!("invalid_request"));
}

#[tokio::test]
async fn should_close_the_connection_when_a_line_is_too_long() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect_raw_after_hello(false).await;

    client.send_bytes(&vec![b'a'; MAX_LINE_BYTES + 2]).await;
    client.read_value().await;

    assert_eq!(client.read_value().await, None);
}

#[tokio::test]
async fn should_fail_calls_with_connection_closed_when_the_server_shuts_down() {
    let daemon = TestDaemon::start();
    let client = daemon.connect(ClientKind::Cli, false).await;

    daemon.shutdown.cancel();
    daemon.task.await.unwrap();
    let error = client.show_ui().await.unwrap_err();

    assert!(matches!(error, ClientError::ConnectionClosed));
}
