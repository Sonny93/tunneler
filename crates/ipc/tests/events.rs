#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::time::Duration;

use ipc::protocol::{ClientKind, EventMessage, PromptResolvedEvent};
use ipc::{ConnectionNotification, SendError, ServerOptions};
use pretty_assertions::assert_eq;
use serde_json::json;
use support::{QUIET_PERIOD, TestDaemon, WAIT, large_event};
use tokio::time::{sleep, timeout};

const SMALL_QUEUE_CAPACITY: usize = 4;
const LARGE_EVENT_KILOBYTES: usize = 64;
const POLL_INTERVAL: Duration = Duration::from_millis(5);

fn resolved(prompt_id: &str) -> EventMessage {
    EventMessage::PromptResolved(PromptResolvedEvent {
        prompt_id: prompt_id.to_owned(),
    })
}

#[tokio::test]
async fn should_deliver_events_to_subscribed_clients() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect(ClientKind::Ui, true).await;
    let mut events = client.take_events().unwrap();

    daemon.events.send(resolved("p1")).unwrap();

    let received = timeout(WAIT, events.recv()).await.unwrap();
    assert_eq!(received, Some(resolved("p1")));
}

#[tokio::test]
async fn should_not_deliver_events_to_clients_that_did_not_subscribe() {
    let daemon = TestDaemon::start();
    let mut quiet = daemon.connect_raw_after_hello(false).await;
    let mut subscribed = daemon.connect_raw_after_hello(true).await;

    daemon.events.send(resolved("p1")).unwrap();
    subscribed.read_value().await.unwrap();

    assert_eq!(quiet.read_value_within(QUIET_PERIOD).await, None);
}

#[tokio::test]
async fn should_keep_serving_requests_when_a_client_did_not_subscribe() {
    let daemon = TestDaemon::start();
    let client = daemon.connect(ClientKind::Cli, false).await;

    daemon.events.send(resolved("p1")).unwrap();

    assert!(client.show_ui().await.is_ok());
}

#[tokio::test]
async fn should_deliver_events_in_order() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect(ClientKind::Ui, true).await;
    let mut events = client.take_events().unwrap();

    for index in 0..5 {
        daemon.events.send(resolved(&index.to_string())).unwrap();
    }

    let mut received = Vec::new();
    for _ in 0..5 {
        received.push(timeout(WAIT, events.recv()).await.unwrap().unwrap());
    }
    assert_eq!(
        received,
        (0..5)
            .map(|index| resolved(&index.to_string()))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn should_reach_only_the_targeted_client_when_sending_to_one_connection() {
    let daemon = TestDaemon::start();
    let mut notifications = daemon.handle.subscribe_connections();
    let mut first = daemon.connect_raw_after_hello(true).await;
    let mut second = daemon.connect_raw_after_hello(true).await;
    let first_id = loop {
        let ConnectionNotification::Opened(info) = notifications.recv().await.unwrap() else {
            continue;
        };
        break info.id;
    };

    daemon
        .handle
        .send_to(first_id, &resolved("only-first"))
        .unwrap();

    let received = first.read_value().await.unwrap();
    assert_eq!(
        (
            received["data"]["promptId"].clone(),
            second.read_value_within(QUIET_PERIOD).await
        ),
        (json!("only-first"), None)
    );
}

#[tokio::test]
async fn should_reach_a_client_that_did_not_subscribe_when_sending_to_it() {
    let daemon = TestDaemon::start();
    let mut quiet = daemon.connect_raw_after_hello(false).await;
    let id = daemon.handle.connections()[0].id;

    daemon.handle.send_to(id, &resolved("direct")).unwrap();

    let received = quiet.read_value().await.unwrap();
    assert_eq!(received["event"], json!("prompt.resolved"));
}

#[tokio::test]
async fn should_fail_with_not_connected_when_sending_to_an_unknown_connection() {
    let daemon = TestDaemon::start();
    let client = daemon.connect(ClientKind::Cli, false).await;
    let id = daemon.handle.connections()[0].id;
    drop(client);
    while !daemon.handle.connections().is_empty() {
        sleep(POLL_INTERVAL).await;
    }

    let error = daemon.handle.send_to(id, &resolved("late")).unwrap_err();

    assert!(matches!(error, SendError::NotConnected));
}

#[tokio::test]
async fn should_end_the_event_stream_when_the_server_shuts_down() {
    let daemon = TestDaemon::start();
    let mut client = daemon.connect(ClientKind::Ui, true).await;
    let mut events = client.take_events().unwrap();

    daemon.shutdown.cancel();

    assert_eq!(timeout(WAIT, events.recv()).await.unwrap(), None);
}

#[tokio::test]
async fn should_drop_a_slow_client_without_stalling_the_others() {
    let daemon = TestDaemon::start_with(ServerOptions {
        outbound_queue_capacity: SMALL_QUEUE_CAPACITY,
        ..ServerOptions::default()
    });
    let _stalled = daemon.connect_raw_after_hello(true).await;
    let mut healthy = daemon.connect(ClientKind::Ui, true).await;
    let mut healthy_events = healthy.take_events().unwrap();
    let drain = tokio::spawn(async move { while healthy_events.recv().await.is_some() {} });

    let stalled_dropped = timeout(WAIT, async {
        while daemon.handle.connections().len() > 1 {
            let _ = daemon.events.send(large_event(LARGE_EVENT_KILOBYTES));
            sleep(POLL_INTERVAL).await;
        }
    })
    .await;

    drain.abort();
    assert!(stalled_dropped.is_ok());
}

#[tokio::test]
async fn should_keep_serving_the_healthy_client_after_dropping_a_slow_one() {
    let daemon = TestDaemon::start_with(ServerOptions {
        outbound_queue_capacity: SMALL_QUEUE_CAPACITY,
        ..ServerOptions::default()
    });
    let _stalled = daemon.connect_raw_after_hello(true).await;
    let mut healthy = daemon.connect(ClientKind::Ui, true).await;
    let mut healthy_events = healthy.take_events().unwrap();
    let drain = tokio::spawn(async move { while healthy_events.recv().await.is_some() {} });
    timeout(WAIT, async {
        while daemon.handle.connections().len() > 1 {
            let _ = daemon.events.send(large_event(LARGE_EVENT_KILOBYTES));
            sleep(POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap();

    let outcome = healthy.show_ui().await;

    drain.abort();
    assert!(outcome.is_ok());
}
