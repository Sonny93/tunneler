#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::sync::Arc;

use engine::{ConnectError, Connector, HostKeyDecision, JumpChainError};
use pretty_assertions::assert_eq;
use russh::MethodKind;
use russh::keys::Algorithm;
use support::{
    HostSpec, ScriptedPrompter, ScriptedVerifier, TestServer, TestUser, Workspace, generate_key,
    rejecting_verifier,
};

const PASSWORD: &str = "s3cret";
const TARGET: &str = "target";
const LOOPBACK: &str = "127.0.0.1";
const UNUSED_PORT: u16 = 1;
const OVERRIDING_USER: &str = "bob";

fn user_with_password(password: &str) -> TestUser {
    TestUser {
        password: Some(password.to_owned()),
        ..TestUser::default()
    }
}

async fn server_for(user_name: &str, password: &str) -> TestServer {
    TestServer::builder()
        .host_key(generate_key(Algorithm::Ed25519))
        .user(user_name, user_with_password(password))
        .methods(&[MethodKind::Password])
        .start()
        .await
}

async fn password_server() -> TestServer {
    server_for(support::REMOTE_USER, PASSWORD).await
}

fn prompter_knowing_the_password() -> Arc<ScriptedPrompter> {
    Arc::new(ScriptedPrompter::default().with_passwords(&[Some(PASSWORD); 4]))
}

fn host<'a>(alias: &'a str, server: &TestServer, extra_lines: &[&'a str]) -> HostSpec<'a> {
    HostSpec {
        alias,
        port: server.port,
        extra_lines: extra_lines.to_vec(),
    }
}

fn connector_for(
    workspace: &Workspace,
    hosts: &[HostSpec],
    prompter: &Arc<ScriptedPrompter>,
    verifier: Arc<ScriptedVerifier>,
) -> Connector {
    let config_path = workspace.write_hosts_config(hosts);
    workspace.connector(&config_path, Arc::clone(prompter) as _, verifier as _)
}

fn root_cause(mut error: ConnectError) -> ConnectError {
    while let ConnectError::Jump { source, .. } = error {
        error = *source;
    }
    error
}

fn targets_of(server: &TestServer) -> Vec<(String, u16)> {
    server.direct_tcpip_targets()
}

fn loopback(port: u16) -> (String, u16) {
    (LOOPBACK.to_owned(), port)
}

#[tokio::test]
async fn should_reach_the_target_through_the_jump_host_when_proxy_jump_names_one() {
    let (jump, target) = (password_server().await, password_server().await);
    let workspace = Workspace::new();
    let hosts = [
        host("jump", &jump, &[]),
        host(TARGET, &target, &["ProxyJump jump"]),
    ];
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        rejecting_verifier(),
    );

    let session = connector.connect(TARGET).await.unwrap();

    assert_eq!(session.jump_chain(), vec!["jump"]);
    assert_eq!(targets_of(&jump), vec![loopback(target.port)]);
}

#[tokio::test]
async fn should_authenticate_every_hop_with_its_own_alias_when_going_through_a_jump_host() {
    let (jump, target) = (password_server().await, password_server().await);
    let workspace = Workspace::new();
    let hosts = [
        host("jump", &jump, &[]),
        host(TARGET, &target, &["ProxyJump jump"]),
    ];
    let prompter = prompter_knowing_the_password();
    let connector = connector_for(&workspace, &hosts, &prompter, rejecting_verifier());

    connector.connect(TARGET).await.unwrap();

    let aliases: Vec<String> = prompter
        .password_requests()
        .into_iter()
        .map(|request| request.alias)
        .collect();
    assert_eq!(aliases, vec!["jump".to_owned(), TARGET.to_owned()]);
}

#[tokio::test]
async fn should_chain_the_tunnels_when_proxy_jump_names_two_hosts() {
    let (first, second, target) = (
        password_server().await,
        password_server().await,
        password_server().await,
    );
    let workspace = Workspace::new();
    let hosts = [
        host("first", &first, &[]),
        host("second", &second, &[]),
        host(TARGET, &target, &["ProxyJump first,second"]),
    ];
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        rejecting_verifier(),
    );

    let session = connector.connect(TARGET).await.unwrap();

    assert_eq!(session.jump_chain(), vec!["first", "second"]);
    assert_eq!(targets_of(&first), vec![loopback(second.port)]);
    assert_eq!(targets_of(&second), vec![loopback(target.port)]);
}

#[tokio::test]
async fn should_apply_the_user_and_port_of_the_spec_when_the_hop_config_differs() {
    let jump = server_for(OVERRIDING_USER, PASSWORD).await;
    let target = password_server().await;
    let workspace = Workspace::new();
    let proxy_jump = format!("ProxyJump {OVERRIDING_USER}@jump:{}", jump.port);
    let hosts = [
        HostSpec {
            alias: "jump",
            port: UNUSED_PORT,
            extra_lines: Vec::new(),
        },
        host(TARGET, &target, &[proxy_jump.as_str()]),
    ];
    let prompter = prompter_knowing_the_password();
    let connector = connector_for(&workspace, &hosts, &prompter, rejecting_verifier());

    connector.connect(TARGET).await.unwrap();

    assert_eq!(prompter.password_requests()[0].user, OVERRIDING_USER);
}

#[tokio::test]
async fn should_honor_the_proxy_jump_of_the_first_hop_when_it_has_one() {
    let (outer, inner, target) = (
        password_server().await,
        password_server().await,
        password_server().await,
    );
    let workspace = Workspace::new();
    let hosts = [
        host("outer", &outer, &[]),
        host("inner", &inner, &["ProxyJump outer"]),
        host(TARGET, &target, &["ProxyJump inner"]),
    ];
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        rejecting_verifier(),
    );

    let session = connector.connect(TARGET).await.unwrap();

    assert_eq!(session.jump_chain(), vec!["outer", "inner"]);
    assert_eq!(targets_of(&outer), vec![loopback(inner.port)]);
}

#[tokio::test]
async fn should_ignore_the_proxy_jump_of_a_later_hop_when_it_has_one() {
    let (first, second, target) = (
        password_server().await,
        password_server().await,
        password_server().await,
    );
    let workspace = Workspace::new();
    let hosts = [
        host("first", &first, &[]),
        host("second", &second, &["ProxyJump unreachable-ghost"]),
        host(TARGET, &target, &["ProxyJump first,second"]),
    ];
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        rejecting_verifier(),
    );

    let session = connector.connect(TARGET).await.unwrap();

    assert_eq!(session.jump_chain(), vec!["first", "second"]);
}

#[tokio::test]
async fn should_report_the_cycle_when_two_hosts_jump_through_each_other() {
    let unused = password_server().await;
    let workspace = Workspace::new();
    let hosts = [
        host("a", &unused, &["ProxyJump b"]),
        host("b", &unused, &["ProxyJump a"]),
    ];
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        rejecting_verifier(),
    );

    let error = connector.connect("a").await.unwrap_err();

    assert!(matches!(
        root_cause(error),
        ConnectError::JumpChain(JumpChainError::Cycle { chain })
            if chain == ["a", "b", "a"]
    ));
}

#[tokio::test]
async fn should_refuse_a_chain_that_nests_too_deeply() {
    let unused = password_server().await;
    let workspace = Workspace::new();
    let aliases: Vec<String> = (0..10).map(|number| format!("h{number}")).collect();
    let proxy_lines: Vec<String> = (1..10)
        .map(|number| format!("ProxyJump h{number}"))
        .collect();
    let hosts: Vec<HostSpec> = aliases
        .iter()
        .enumerate()
        .map(|(index, alias)| match proxy_lines.get(index) {
            Some(line) => host(alias, &unused, &[line.as_str()]),
            None => host(alias, &unused, &[]),
        })
        .collect();
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        rejecting_verifier(),
    );

    let error = connector.connect("h0").await.unwrap_err();

    assert!(matches!(
        root_cause(error),
        ConnectError::JumpChain(JumpChainError::TooDeep { .. })
    ));
}

#[tokio::test]
async fn should_ask_about_the_unknown_key_of_a_hop_with_the_hop_alias() {
    let (jump, target) = (password_server().await, password_server().await);
    let workspace = Workspace::new();
    let hosts = [
        host("jump", &jump, &["StrictHostKeyChecking ask"]),
        host(TARGET, &target, &["ProxyJump jump"]),
    ];
    let verifier =
        Arc::new(ScriptedVerifier::default().with_decisions(&[HostKeyDecision::AcceptOnce]));
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        Arc::clone(&verifier),
    );

    connector.connect(TARGET).await.unwrap();

    let aliases: Vec<String> = verifier
        .unknown_requests()
        .into_iter()
        .map(|request| request.alias)
        .collect();
    assert_eq!(aliases, vec!["jump".to_owned()]);
}

#[tokio::test]
async fn should_report_the_hop_when_authentication_on_a_jump_host_fails() {
    let jump = server_for(support::REMOTE_USER, "another password").await;
    let target = password_server().await;
    let workspace = Workspace::new();
    let hosts = [
        host("jump", &jump, &[]),
        host(TARGET, &target, &["ProxyJump jump"]),
    ];
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        rejecting_verifier(),
    );

    let error = connector.connect(TARGET).await.unwrap_err();

    assert!(matches!(
        error,
        ConnectError::Jump { hop, source }
            if hop == "jump" && matches!(*source, ConnectError::Authentication { .. })
    ));
}

#[tokio::test]
async fn should_close_every_session_when_the_target_disconnects() {
    let (jump, target) = (password_server().await, password_server().await);
    let workspace = Workspace::new();
    let hosts = [
        host("jump", &jump, &[]),
        host(TARGET, &target, &["ProxyJump jump"]),
    ];
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        rejecting_verifier(),
    );
    let session = connector.connect(TARGET).await.unwrap();

    let outcome = session.disconnect().await;

    assert!(outcome.is_ok(), "{outcome:?}");
}

#[tokio::test]
async fn should_resolve_closed_when_the_jump_host_connection_is_cut() {
    let (jump, target) = (password_server().await, password_server().await);
    let workspace = Workspace::new();
    let hosts = [
        host("jump", &jump, &[]),
        host(TARGET, &target, &["ProxyJump jump"]),
    ];
    let connector = connector_for(
        &workspace,
        &hosts,
        &prompter_knowing_the_password(),
        rejecting_verifier(),
    );
    let session = connector.connect(TARGET).await.unwrap();

    jump.cut_connections();

    let outcome = tokio::time::timeout(std::time::Duration::from_secs(3), session.closed()).await;
    assert!(outcome.is_ok());
}
