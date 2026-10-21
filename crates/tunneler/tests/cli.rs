//! End-to-end tests: the real `tunneler` binary against an in-process SSH
//! server. Every spawned daemon is stopped when its fixture is dropped.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Output, Stdio};
use std::time::Duration;

use russh::keys::Algorithm;
use serde_json::Value;
use test_support::{
    ALIAS, EchoServer, REMOTE_USER, TestServer, TestUser, Workspace, free_port, generate_key,
    write_private_key,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::time::{Instant, sleep, timeout};

const BINARY: &str = env!("CARGO_BIN_EXE_tunneler");
const WAIT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const ECHO_MESSAGE: &[u8] = b"ping";
const EXIT_ERROR: i32 = 1;
const EXIT_USAGE: i32 = 2;
const EXIT_DAEMON_NOT_RUNNING: i32 = 3;
const EXIT_TARGETS_FAILED: i32 = 4;
const EXIT_CONFIRMATION_REQUIRED: i32 = 5;
const EXIT_UNKNOWN_OPENER: i32 = 9;
const ESCAPE: char = '\x1b';
const MISSING_UI_BINARY_NAME: &str = "no-such-tunneler-ui";
const FAKE_UI_SCRIPT_NAME: &str = "fake-tunneler-ui";
const UI_MARKER_NAME: &str = "ui-marker";

struct Fixture {
    workspace: Workspace,
    listen_ports: [u16; 3],
    echo_port: u16,
    _server: TestServer,
    _echo: EchoServer,
}

impl Fixture {
    async fn start() -> Self {
        let echo = EchoServer::start().await;
        let key = generate_key(Algorithm::Ed25519);
        let server = TestServer::builder()
            .host_key(generate_key(Algorithm::Ed25519))
            .user(
                REMOTE_USER,
                TestUser {
                    authorized_keys: vec![key.public_key().clone()],
                    ..TestUser::default()
                },
            )
            .start()
            .await;
        let workspace = Workspace::new();
        let key_path = write_private_key(workspace.path(), "id_ed25519", &key, None);
        let identity_line = format!("IdentityFile {}", key_path.display());
        workspace.write_config(
            server.port,
            &[
                &identity_line,
                "IdentityAgent none",
                "IdentitiesOnly yes",
                "StrictHostKeyChecking accept-new",
            ],
        );
        let listen_ports = distinct_free_ports().await;
        std::fs::write(
            workspace.file("tunnels.toml"),
            tunnels_file(listen_ports, echo.port),
        )
        .unwrap();
        Self {
            workspace,
            listen_ports,
            echo_port: echo.port,
            _server: server,
            _echo: echo,
        }
    }

    fn socket_path(&self) -> PathBuf {
        self.workspace.file("tunneler.sock")
    }

    fn global_arguments(&self) -> Vec<String> {
        vec![
            "--socket".to_owned(),
            self.socket_path().display().to_string(),
            "--config".to_owned(),
            self.workspace.file("tunnels.toml").display().to_string(),
            "--ssh-config".to_owned(),
            self.workspace.file("ssh_config").display().to_string(),
        ]
    }

    fn missing_ui_binary(&self) -> PathBuf {
        self.workspace.file(MISSING_UI_BINARY_NAME)
    }

    fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(BINARY);
        command
            .args(self.global_arguments())
            .args(arguments)
            .stdin(Stdio::null())
            .env("HOME", self.workspace.file("home"))
            .env("XDG_DATA_HOME", self.workspace.file("data"))
            .env("TUNNELER_NO_TRAY", "1")
            .env("TUNNELER_UI_BINARY", self.missing_ui_binary())
            .env_remove("SSH_AUTH_SOCK")
            .env_remove("TUNNELER_LOG")
            .env_remove("NO_COLOR")
            .kill_on_drop(true);
        command
    }

    async fn run(&self, arguments: &[&str]) -> Output {
        timeout(WAIT * 3, self.command(arguments).output())
            .await
            .expect("the command did not finish")
            .unwrap()
    }

    async fn run_with_environment(&self, arguments: &[&str], variables: &[(&str, &str)]) -> Output {
        let mut command = self.command(arguments);
        command.envs(variables.iter().copied());
        timeout(WAIT * 3, command.output())
            .await
            .expect("the command did not finish")
            .unwrap()
    }

    fn spawn_daemon(&self) -> Child {
        self.spawn_daemon_with_environment(&[])
    }

    fn spawn_daemon_with_environment(&self, variables: &[(&str, &Path)]) -> Child {
        let mut command = self.command(&["daemon"]);
        command.envs(variables.iter().copied());
        command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    fn write_fake_ui(&self) -> (PathBuf, PathBuf) {
        use std::os::unix::fs::PermissionsExt;

        let script = self.workspace.file(FAKE_UI_SCRIPT_NAME);
        let marker = self.workspace.file(UI_MARKER_NAME);
        let body = format!(
            "#!/bin/sh\nprintf '%s' \"$TUNNELER_SOCKET\" > '{}'\n",
            marker.display()
        );
        std::fs::write(&script, body).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        (script, marker)
    }

    async fn wait_until_daemon_answers(&self) {
        let socket = self.socket_path();
        wait_until("the daemon answers", || {
            std::os::unix::net::UnixStream::connect(&socket).is_ok()
        })
        .await;
    }

    async fn status_json(&self) -> Value {
        let output = self.run(&["status", "--json"]).await;
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        serde_json::from_str(&stdout(&output)).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = StdCommand::new(BINARY)
            .args(self.global_arguments())
            .args(["quit", "--yes"])
            .env("HOME", self.workspace.file("home"))
            .env("XDG_DATA_HOME", self.workspace.file("data"))
            .env("TUNNELER_NO_TRAY", "1")
            .env("TUNNELER_UI_BINARY", self.missing_ui_binary())
            .stdin(Stdio::null())
            .output();
        let _ = StdCommand::new("pkill")
            .args(["-9", "-f"])
            .arg(self.socket_path())
            .status();
    }
}

async fn distinct_free_ports() -> [u16; 3] {
    let mut ports = Vec::new();
    while ports.len() < 3 {
        let port = free_port().await;
        if !ports.contains(&port) {
            ports.push(port);
        }
    }
    [ports[0], ports[1], ports[2]]
}

fn tunnels_file(listen_ports: [u16; 3], target_port: u16) -> String {
    let definitions = [
        (listen_ports[0], r#"["dev", "alpha"]"#),
        (listen_ports[1], r#"["dev", "beta"]"#),
        (listen_ports[2], r#"["gamma"]"#),
    ];
    let tunnels = definitions
        .iter()
        .map(|(listen_port, tags)| {
            format!(
                "\n[[tunnels]]\nhost = \"{ALIAS}\"\nkind = \"local\"\n\
                 listen_port = {listen_port}\ntarget_port = {target_port}\ntags = {tags}\n"
            )
        })
        .collect::<Vec<_>>()
        .concat();
    format!("version = 1\n{tunnels}")
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

async fn wait_until(description: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        sleep(POLL_INTERVAL).await;
    }
}

async fn assert_echoes(port: u16) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream.write_all(ECHO_MESSAGE).await.unwrap();
    let mut echoed = [0_u8; 4];
    timeout(WAIT, stream.read_exact(&mut echoed))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(echoed, ECHO_MESSAGE);
}

async fn wait_until_port_is_closed(port: u16) {
    let deadline = Instant::now() + WAIT;
    while TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
        assert!(Instant::now() < deadline, "port {port} is still open");
        sleep(POLL_INTERVAL).await;
    }
}

fn tunnel_states(status: &Value) -> Vec<(u64, String)> {
    status["tunnels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tunnel| {
            (
                tunnel["listenPort"].as_u64().unwrap(),
                tunnel["state"]["kind"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn has_step(output: &str, listen_port: u16, step: &str) -> bool {
    output.lines().any(|line| {
        line.starts_with(&format!("{listen_port} ")) && line.ends_with(&format!(": {step}"))
    })
}

fn socket_exists(path: &Path) -> bool {
    path.exists()
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_3_and_a_hint_when_status_runs_without_a_daemon() {
    let fixture = Fixture::start().await;

    let output = fixture.run(&["status"]).await;

    assert_eq!(code(&output), EXIT_DAEMON_NOT_RUNNING);
    let errors = stderr(&output);
    assert!(
        errors.contains("error: the Tunneler daemon is not running")
            && errors.contains("hint: start it with `tunneler daemon` or run `tunneler up <tag>`"),
        "{errors}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_start_the_daemon_and_forward_traffic_when_up_targets_a_tag() {
    let fixture = Fixture::start().await;

    let output = fixture.run(&["up", "dev"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let steps = stdout(&output);
    assert!(
        has_step(&steps, fixture.listen_ports[0], "active")
            && has_step(&steps, fixture.listen_ports[1], "active"),
        "{steps}"
    );
    assert_echoes(fixture.listen_ports[0]).await;
    assert_echoes(fixture.listen_ports[1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn should_report_the_tagged_tunnels_as_active_when_status_prints_json() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "dev"]).await;

    let status = fixture.status_json().await;

    assert_eq!(
        tunnel_states(&status),
        vec![
            (u64::from(fixture.listen_ports[0]), "active".to_owned()),
            (u64::from(fixture.listen_ports[1]), "active".to_owned()),
            (u64::from(fixture.listen_ports[2]), "stopped".to_owned()),
        ]
    );
    assert!(status["daemonVersion"].is_string());
}

fn make_first_tunnel_standby(fixture: &Fixture) {
    let path = fixture.workspace.file("tunnels.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    let updated = text.replacen(
        "kind = \"local\"\n",
        "kind = \"local\"\nstandby = true\n",
        1,
    );
    std::fs::write(path, updated).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn should_report_a_standby_tunnel_as_success_when_up_targets_it() {
    let fixture = Fixture::start().await;
    make_first_tunnel_standby(&fixture);

    let output = fixture.run(&["up", "alpha"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let steps = stdout(&output);
    assert!(
        has_step(&steps, fixture.listen_ports[0], "en veille"),
        "{steps}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_carry_the_standby_state_when_status_prints_json() {
    let fixture = Fixture::start().await;
    make_first_tunnel_standby(&fixture);
    fixture.run(&["up", "alpha"]).await;

    let status = fixture.status_json().await;

    assert_eq!(
        tunnel_states(&status)[0],
        (u64::from(fixture.listen_ports[0]), "standby".to_owned())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_print_the_word_veille_without_escape_sequences_when_status_runs_without_color() {
    let fixture = Fixture::start().await;
    make_first_tunnel_standby(&fixture);
    fixture.run(&["up", "alpha"]).await;

    let output = fixture
        .run_with_environment(&["status"], &[("NO_COLOR", "1")])
        .await;

    let text = stdout(&output);
    assert!(!text.contains(ESCAPE), "{text}");
    assert!(
        text.contains(&format!("veille  {} ", fixture.listen_ports[0])),
        "{text}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_print_words_without_escape_sequences_when_status_runs_without_a_terminal() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "dev"]).await;

    let output = fixture
        .run_with_environment(&["status"], &[("NO_COLOR", "1")])
        .await;

    let text = stdout(&output);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(!text.contains(ESCAPE), "{text}");
    assert!(
        text.contains(&format!("active  {} ", fixture.listen_ports[0]))
            && text.contains(&format!("stopped  {} ", fixture.listen_ports[2])),
        "{text}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_color_the_state_when_color_is_forced() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "dev"]).await;

    let output = fixture.run(&["status", "--color", "always"]).await;

    assert!(stdout(&output).contains(ESCAPE), "{}", stdout(&output));
}

#[tokio::test(flavor = "multi_thread")]
async fn should_list_the_configured_tunnels_when_the_daemon_runs() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "gamma"]).await;

    let output = fixture.run(&["list"]).await;

    let text = stdout(&output);
    assert!(
        text.contains(&format!("local  {} ", fixture.listen_ports[0]))
            && text.contains("[dev, alpha]")
            && text.contains("[gamma]"),
        "{text}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_close_the_ports_when_down_stops_a_tag() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "dev"]).await;

    let output = fixture.run(&["down", "dev"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    wait_until_port_is_closed(fixture.listen_ports[0]).await;
    wait_until_port_is_closed(fixture.listen_ports[1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn should_stop_every_tunnel_when_down_is_given_all() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "dev", "gamma"]).await;

    let output = fixture.run(&["down", "--all"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    for port in fixture.listen_ports {
        wait_until_port_is_closed(port).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_1_and_a_hint_when_up_targets_an_unknown_tag() {
    let fixture = Fixture::start().await;

    let output = fixture.run(&["up", "nothing"]).await;

    assert_eq!(code(&output), EXIT_ERROR);
    let errors = stderr(&output);
    assert!(
        errors.contains("error: no tunnel carries the tag \"nothing\"")
            && errors.contains("hint: run `tunneler list`"),
        "{errors}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_4_when_a_target_fails_to_start() {
    let fixture = Fixture::start().await;
    let _occupant = std::net::TcpListener::bind(("127.0.0.1", fixture.listen_ports[0])).unwrap();

    let output = fixture.run(&["up", "alpha"]).await;

    assert_eq!(code(&output), EXIT_TARGETS_FAILED);
    assert!(
        stdout(&output).lines().any(|line| line
            .starts_with(&format!("{} ", fixture.listen_ports[0]))
            && line.contains(": port held by")),
        "{}",
        stdout(&output)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_return_at_once_when_up_is_given_no_wait() {
    let fixture = Fixture::start().await;

    let output = fixture.run(&["up", "--no-wait", "alpha"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(has_step(
        &stdout(&output),
        fixture.listen_ports[0],
        "start requested"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn should_report_the_target_reachable_when_check_runs_on_an_active_tunnel() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "alpha"]).await;

    let output = fixture.run(&["check", "alpha"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).contains(": target reachable"),
        "{}",
        stdout(&output)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_1_and_a_hint_when_check_runs_on_a_stopped_tunnel() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "alpha"]).await;

    let output = fixture.run(&["check", "gamma"]).await;

    assert_eq!(code(&output), EXIT_ERROR);
    assert!(
        stderr(&output).contains("only a running local or remote tunnel can be checked"),
        "{}",
        stderr(&output)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_5_when_quit_needs_a_confirmation_and_there_is_no_terminal() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "dev"]).await;

    let output = fixture.run(&["quit"]).await;

    assert_eq!(code(&output), EXIT_CONFIRMATION_REQUIRED);
    assert!(stderr(&output).contains("hint:"), "{}", stderr(&output));
    assert_eq!(code(&fixture.run(&["status"]).await), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn should_stop_the_daemon_and_remove_the_socket_when_quit_is_given_yes() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "dev"]).await;

    let output = fixture.run(&["quit", "--yes"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        code(&fixture.run(&["status"]).await),
        EXIT_DAEMON_NOT_RUNNING
    );
    assert!(!socket_exists(&fixture.socket_path()));
    wait_until_port_is_closed(fixture.listen_ports[0]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn should_refuse_to_start_when_a_daemon_already_runs() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "gamma"]).await;

    let output = fixture.run(&["daemon"]).await;

    assert_eq!(code(&output), EXIT_ERROR);
    assert!(
        stderr(&output).contains("error: Tunneler is already running"),
        "{}",
        stderr(&output)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_1_and_a_hint_when_the_ui_program_cannot_be_launched() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "gamma"]).await;

    let output = fixture.run(&[]).await;

    assert_eq!(code(&output), EXIT_ERROR);
    assert!(
        stderr(&output).contains("hint: the UI program `tunneler-ui` must sit next to `tunneler`"),
        "{}",
        stderr(&output)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_name_the_missing_ui_program_when_it_cannot_be_launched() {
    let fixture = Fixture::start().await;
    fixture.run(&["up", "gamma"]).await;

    let output = fixture.run(&[]).await;

    assert!(
        stderr(&output).contains(MISSING_UI_BINARY_NAME),
        "{}",
        stderr(&output)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_launch_the_ui_program_with_the_socket_when_launched_without_a_command() {
    let fixture = Fixture::start().await;
    let (script, marker) = fixture.write_fake_ui();
    let mut daemon = fixture.spawn_daemon_with_environment(&[("TUNNELER_UI_BINARY", &script)]);
    fixture.wait_until_daemon_answers().await;

    let output = fixture.run(&[]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    wait_until("the UI program wrote its marker", || {
        std::fs::read_to_string(&marker).is_ok_and(|content| !content.is_empty())
    })
    .await;
    assert_eq!(
        std::fs::read_to_string(&marker).unwrap(),
        fixture.socket_path().display().to_string()
    );
    daemon.kill().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn should_reclaim_the_socket_when_the_daemon_was_killed() {
    let fixture = Fixture::start().await;
    let mut daemon = fixture.spawn_daemon();
    fixture.wait_until_daemon_answers().await;
    daemon.kill().await.unwrap();
    assert!(socket_exists(&fixture.socket_path()));

    let output = fixture.run(&["up", "dev"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_echoes(fixture.listen_ports[0]).await;
}

fn find_tunnel(status: &Value, listen_port: u16) -> Option<&Value> {
    status["tunnels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tunnel| tunnel["listenPort"].as_u64() == Some(u64::from(listen_port)))
}

async fn start_temporary_forward(fixture: &Fixture) -> (u16, Output) {
    let port = free_port().await;
    let forward = format!("{port}:127.0.0.1:{}", fixture.echo_port);
    let output = fixture.run(&["up", ALIAS, "-L", &forward]).await;
    (port, output)
}

#[tokio::test(flavor = "multi_thread")]
async fn should_start_a_temporary_tunnel_and_say_how_to_stop_it_when_up_is_given_a_forward() {
    let fixture = Fixture::start().await;

    let (port, output) = start_temporary_forward(&fixture).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let steps = stdout(&output);
    assert!(
        steps
            .lines()
            .any(|line| line.contains("(temporaire): active")),
        "{steps}"
    );
    assert!(
        steps.contains(&format!("tunneler down {ALIAS}:{port}")),
        "{steps}"
    );
    assert_echoes(port).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn should_never_write_the_temporary_tunnel_to_the_tunnels_file() {
    let fixture = Fixture::start().await;
    let before = std::fs::read_to_string(fixture.workspace.file("tunnels.toml")).unwrap();

    let (port, _) = start_temporary_forward(&fixture).await;
    fixture.run(&["up", "gamma"]).await;
    fixture.run(&["down", "gamma"]).await;

    let after = std::fs::read_to_string(fixture.workspace.file("tunnels.toml")).unwrap();
    assert!(!after.contains(&port.to_string()), "{after}");
    assert_eq!(
        after.matches("[[tunnels]]").count(),
        before.matches("[[tunnels]]").count()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_mark_the_temporary_tunnel_when_status_and_list_run() {
    let fixture = Fixture::start().await;
    let (port, _) = start_temporary_forward(&fixture).await;

    let status = fixture.run(&["status"]).await;
    let list = fixture.run(&["list"]).await;

    assert!(
        stdout(&status).contains("(temporaire)") && stdout(&list).contains("(temporaire)"),
        "{}\n{}",
        stdout(&status),
        stdout(&list)
    );
    assert!(!stdout(&list).contains("tunneler") && stdout(&list).contains(&port.to_string()));
}

#[tokio::test(flavor = "multi_thread")]
async fn should_flag_the_temporary_tunnel_when_status_prints_json() {
    let fixture = Fixture::start().await;
    let (port, _) = start_temporary_forward(&fixture).await;

    let status = fixture.status_json().await;

    assert_eq!(
        (
            find_tunnel(&status, port).map(|tunnel| tunnel["isTemporary"].clone()),
            find_tunnel(&status, fixture.listen_ports[0])
                .map(|tunnel| tunnel["isTemporary"].clone())
        ),
        (Some(Value::Bool(true)), Some(Value::Bool(false)))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_flag_the_temporary_tunnel_when_list_prints_json() {
    let fixture = Fixture::start().await;
    let (port, _) = start_temporary_forward(&fixture).await;

    let output = fixture.run(&["list", "--json"]).await;

    let listed: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(
        find_tunnel(&listed, port).map(|tunnel| tunnel["isTemporary"].clone()),
        Some(Value::Bool(true))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_remove_the_temporary_tunnel_when_down_stops_it() {
    let fixture = Fixture::start().await;
    let (port, _) = start_temporary_forward(&fixture).await;

    let output = fixture.run(&["down", &port.to_string()]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    wait_until_port_is_closed(port).await;
    let status = fixture.status_json().await;
    assert!(find_tunnel(&status, port).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn should_start_the_saved_tunnel_when_the_forward_matches_one() {
    let fixture = Fixture::start().await;
    let port = fixture.listen_ports[0];
    let forward = format!("{port}:127.0.0.1:{}", fixture.echo_port);

    let output = fixture.run(&["up", ALIAS, "-L", &forward]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).contains("already a saved tunnel"),
        "{}",
        stdout(&output)
    );
    let status = fixture.status_json().await;
    assert_eq!(status["tunnels"].as_array().unwrap().len(), 3);
    assert_echoes(port).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn should_start_a_temporary_socks_proxy_when_up_is_given_a_dynamic_forward() {
    let fixture = Fixture::start().await;
    let port = free_port().await;

    let output = fixture.run(&["up", ALIAS, "-D", &port.to_string()]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let status = fixture.status_json().await;
    assert_eq!(
        find_tunnel(&status, port).map(|tunnel| tunnel["kind"].clone()),
        Some(Value::String("dynamic".to_owned()))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_2_and_a_hint_when_the_forward_is_malformed() {
    let fixture = Fixture::start().await;

    let output = fixture.run(&["up", ALIAS, "-L", "nonsense"]).await;

    assert_eq!(code(&output), EXIT_USAGE);
    let errors = stderr(&output);
    assert!(
        errors.contains("not a valid -L specification") && errors.contains("tunneler up --help"),
        "{errors}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_2_when_a_forward_is_given_several_hosts() {
    let fixture = Fixture::start().await;

    let output = fixture.run(&["up", ALIAS, "other", "-L", "5000"]).await;

    assert_eq!(code(&output), EXIT_USAGE);
}

#[tokio::test(flavor = "multi_thread")]
async fn should_show_the_forward_examples_when_up_prints_its_help() {
    let fixture = Fixture::start().await;

    let output = fixture.run(&["up", "--help"]).await;

    assert!(
        stdout(&output).contains("tunneler up nova -L 5000"),
        "{}",
        stdout(&output)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_1_and_say_why_when_open_targets_a_dynamic_tunnel() {
    let fixture = Fixture::start().await;
    let port = free_port().await.to_string();
    fixture.run(&["up", ALIAS, "-D", &port]).await;

    let output = fixture.run(&["open", &port]).await;

    assert_eq!(code(&output), EXIT_ERROR);
    assert!(
        stderr(&output).contains("cannot be opened"),
        "{}",
        stderr(&output)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_start_the_tunnel_and_hand_its_url_to_the_system_opener_when_open_targets_a_stopped_tunnel()
 {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::start().await;
    let bin_directory = fixture.workspace.file("bin");
    std::fs::create_dir_all(&bin_directory).unwrap();
    let marker = fixture.workspace.file("opened-url");
    let script = bin_directory.join("xdg-open");
    std::fs::write(
        &script,
        format!("#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin_directory.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let port = fixture.listen_ports[2];

    let output = fixture
        .run_with_environment(&["open", &port.to_string()], &[("PATH", &path)])
        .await;

    let expected_url = format!("http://localhost:{port}/");
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).contains(&format!("Opening {expected_url}")),
        "{}",
        stdout(&output)
    );
    wait_until("the system opener received the url", || {
        std::fs::read_to_string(&marker).is_ok_and(|content| content == expected_url)
    })
    .await;
    assert_echoes(port).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_9_and_list_the_available_ids_when_open_targets_an_unknown_opener() {
    let fixture = Fixture::start().await;
    let port = fixture.listen_ports[2].to_string();

    let output = fixture.run(&["open", &port, "--with", "ghost"]).await;

    assert_eq!(code(&output), EXIT_UNKNOWN_OPENER);
    assert!(stderr(&output).contains("browser"), "{}", stderr(&output));
}

#[tokio::test(flavor = "multi_thread")]
async fn should_run_the_opener_of_the_user_with_the_substituted_arguments_when_open_uses_with() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::start().await;
    let marker = fixture.workspace.file("opener-arguments");
    let script = fixture.workspace.file("fake-viewer");
    std::fs::write(
        &script,
        format!("#!/bin/sh\nprintf '%s' \"$*\" > '{}'\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let tunnels_path = fixture.workspace.file("tunnels.toml");
    let opener = format!(
        "\n[[openers]]\nid = \"viewer\"\nname = \"Viewer\"\ncommand = \"{}\"\nargs = [\"--host={{host}}\", \"--port={{port}}\"]\n",
        script.display()
    );
    let text = std::fs::read_to_string(&tunnels_path).unwrap() + &opener;
    std::fs::write(&tunnels_path, text).unwrap();
    let port = fixture.listen_ports[2];

    let output = fixture
        .run(&["open", &port.to_string(), "--with", "viewer"])
        .await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    wait_until("the opener received its arguments", || {
        std::fs::read_to_string(&marker)
            .is_ok_and(|content| content == format!("--host=localhost --port={port}"))
    })
    .await;
    assert_echoes(port).await;
}
