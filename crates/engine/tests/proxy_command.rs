#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use engine::ConnectError;
use russh::MethodKind;
use russh::keys::Algorithm;
use std::sync::Arc;
use support::{ALIAS, ScriptedPrompter, TestServer, TestUser, Workspace, generate_key};

const BRIDGE: &str = env!("CARGO_BIN_EXE_tcp-bridge");
const PASSWORD: &str = "s3cret";
const DEATH_DEADLINE: Duration = Duration::from_secs(3);
const DEATH_POLL_INTERVAL: Duration = Duration::from_millis(50);

async fn password_server() -> TestServer {
    TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(
            support::REMOTE_USER,
            TestUser {
                password: Some(PASSWORD.to_owned()),
                ..TestUser::default()
            },
        )
        .methods(&[MethodKind::Password])
        .start()
        .await
}

fn connector_with_proxy_command(
    workspace: &Workspace,
    port: u16,
    proxy_command: &str,
) -> engine::Connector {
    let line = format!("ProxyCommand {proxy_command}");
    let config_path = workspace.write_config(port, &[line.as_str()]);
    let prompter = Arc::new(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD)]));
    workspace.connector(&config_path, prompter, support::rejecting_verifier())
}

fn is_alive(process_id: &str) -> bool {
    Command::new("kill")
        .args(["-0", process_id])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

async fn wait_until_dead(process_id: &str) -> bool {
    let started = Instant::now();
    while started.elapsed() < DEATH_DEADLINE {
        if !is_alive(process_id) {
            return true;
        }
        tokio::time::sleep(DEATH_POLL_INTERVAL).await;
    }
    false
}

fn read_process_id(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap().trim().to_owned()
}

#[tokio::test]
async fn should_connect_through_the_proxy_command_when_one_is_configured() {
    let server = password_server().await;
    let workspace = Workspace::new();
    let pid_file = workspace.file("bridge.pid");
    let command = format!("{BRIDGE} 127.0.0.1 %p {}", pid_file.display());
    let connector = connector_with_proxy_command(&workspace, server.port, &command);

    let outcome = connector.connect(ALIAS).await;

    assert!(outcome.is_ok(), "{outcome:?}");
    assert!(pid_file.exists());
}

#[tokio::test]
async fn should_fail_the_handshake_when_the_proxy_command_exits_immediately() {
    let server = password_server().await;
    let workspace = Workspace::new();
    let connector = connector_with_proxy_command(&workspace, server.port, "false");

    let outcome = connector.connect(ALIAS).await;

    assert!(
        matches!(outcome, Err(ConnectError::Handshake(_))),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn should_kill_the_proxy_command_when_the_session_disconnects() {
    let server = password_server().await;
    let workspace = Workspace::new();
    let pid_file = workspace.file("bridge.pid");
    let command = format!("{BRIDGE} 127.0.0.1 %p {}", pid_file.display());
    let connector = connector_with_proxy_command(&workspace, server.port, &command);
    let session = connector.connect(ALIAS).await.unwrap();
    let process_id = read_process_id(&pid_file);
    assert!(is_alive(&process_id));

    session.disconnect().await.unwrap();

    assert!(wait_until_dead(&process_id).await);
}

#[tokio::test]
async fn should_kill_the_proxy_command_when_the_session_is_dropped() {
    let server = password_server().await;
    let workspace = Workspace::new();
    let pid_file = workspace.file("bridge.pid");
    let command = format!("{BRIDGE} 127.0.0.1 %p {}", pid_file.display());
    let connector = connector_with_proxy_command(&workspace, server.port, &command);
    let session = connector.connect(ALIAS).await.unwrap();
    let process_id = read_process_id(&pid_file);

    drop(session);

    assert!(wait_until_dead(&process_id).await);
}
