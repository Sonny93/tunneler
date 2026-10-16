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
const EXIT_DAEMON_NOT_RUNNING: i32 = 3;
const EXIT_TARGETS_FAILED: i32 = 4;
const EXIT_CONFIRMATION_REQUIRED: i32 = 5;
const ESCAPE: char = '\x1b';
const MISSING_UI_BINARY_NAME: &str = "no-such-tunneler-ui";
const FAKE_UI_SCRIPT_NAME: &str = "fake-tunneler-ui";
const UI_MARKER_NAME: &str = "ui-marker";

struct Fixture {
    workspace: Workspace,
    listen_ports: [u16; 3],
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
        ("Alpha", listen_ports[0], r#"["dev"]"#),
        ("Beta", listen_ports[1], r#"["dev"]"#),
        ("Gamma", listen_ports[2], "[]"),
    ];
    let tunnels = definitions
        .iter()
        .map(|(name, listen_port, tags)| {
            format!(
                "\n[[tunnels]]\nname = \"{name}\"\nhost = \"{ALIAS}\"\nkind = \"local\"\n\
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

fn tunnel_states(status: &Value) -> Vec<(String, String)> {
    status["tunnels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tunnel| {
            (
                tunnel["name"].as_str().unwrap().to_owned(),
                tunnel["state"]["kind"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
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
            && errors.contains("hint: start it with `tunneler daemon` or run `tunneler up <name>`"),
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
        steps.contains("Alpha: active") && steps.contains("Beta: active"),
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
            ("Alpha".to_owned(), "active".to_owned()),
            ("Beta".to_owned(), "active".to_owned()),
            ("Gamma".to_owned(), "stopped".to_owned()),
        ]
    );
    assert!(status["daemonVersion"].is_string());
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
        text.contains("active  Alpha") && text.contains("stopped  Gamma"),
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
    fixture.run(&["up", "Gamma"]).await;

    let output = fixture.run(&["list"]).await;

    let text = stdout(&output);
    assert!(
        text.contains("Alpha  local") && text.contains("[dev]") && text.contains("Gamma"),
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
    fixture.run(&["up", "dev", "Gamma"]).await;

    let output = fixture.run(&["down", "--all"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    for port in fixture.listen_ports {
        wait_until_port_is_closed(port).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_1_and_a_hint_when_up_targets_an_unknown_name() {
    let fixture = Fixture::start().await;

    let output = fixture.run(&["up", "nothing"]).await;

    assert_eq!(code(&output), EXIT_ERROR);
    let errors = stderr(&output);
    assert!(
        errors.contains("error: no tunnel or tag is named \"nothing\"")
            && errors.contains("hint: run `tunneler list`"),
        "{errors}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_exit_with_4_when_a_target_fails_to_start() {
    let fixture = Fixture::start().await;
    let _occupant = std::net::TcpListener::bind(("127.0.0.1", fixture.listen_ports[0])).unwrap();

    let output = fixture.run(&["up", "Alpha"]).await;

    assert_eq!(code(&output), EXIT_TARGETS_FAILED);
    assert!(
        stdout(&output).contains("Alpha: failed: port already in use"),
        "{}",
        stdout(&output)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn should_return_at_once_when_up_is_given_no_wait() {
    let fixture = Fixture::start().await;

    let output = fixture.run(&["up", "--no-wait", "Alpha"]).await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(stdout(&output).contains("Alpha: start requested"));
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
    fixture.run(&["up", "Gamma"]).await;

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
    fixture.run(&["up", "Gamma"]).await;

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
    fixture.run(&["up", "Gamma"]).await;

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
