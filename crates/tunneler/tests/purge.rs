//! End-to-end tests of `tunneler purge`: the real binary against throwaway
//! home, XDG and portable directories. The OS keyring is never involved: the
//! seeded `state.toml` lists no passphrase.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use tempfile::TempDir;

const BINARY: &str = env!("CARGO_BIN_EXE_tunneler");
const WAIT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const EXIT_ERROR: i32 = 1;
const EXIT_CONFIRMATION_REQUIRED: i32 = 5;
const AUTOSTART_ENTRY: &str = "[Desktop Entry]\nName=Tunneler\nX-Tunneler-Autostart=true\n";
const FOREIGN_AUTOSTART_ENTRY: &str = "[Desktop Entry]\nName=Tunneler\n";
const STATE_FILE: &str = "version = 1\n";
const TUNNELS_FILE: &str = "version = 1\n";
const SSH_CONFIG: &str = "Host nova\n  HostName example.org\n";

/// A home directory with the files Tunneler leaves in the user's directories.
struct Sandbox {
    root: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let sandbox = Self {
            root: tempfile::tempdir().unwrap(),
        };
        write_file(&sandbox.tunnels_file(), TUNNELS_FILE);
        write_file(&sandbox.state_file(), STATE_FILE);
        write_file(
            &sandbox.log_directory().join("tunneler.2026-01-01.log"),
            "line\n",
        );
        write_file(&sandbox.webview_directory().join("storage"), "cache");
        write_file(&sandbox.autostart_entry(), AUTOSTART_ENTRY);
        write_file(&sandbox.ssh_config(), SSH_CONFIG);
        sandbox
    }

    fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    fn config_home(&self) -> PathBuf {
        self.home().join(".config")
    }

    fn state_home(&self) -> PathBuf {
        self.home().join(".local/state")
    }

    fn data_home(&self) -> PathBuf {
        self.home().join(".local/share")
    }

    fn tunnels_file(&self) -> PathBuf {
        self.config_home().join("tunneler/tunnels.toml")
    }

    fn state_file(&self) -> PathBuf {
        self.state_home().join("tunneler/state.toml")
    }

    fn log_directory(&self) -> PathBuf {
        self.data_home().join("tunneler/logs")
    }

    fn webview_directory(&self) -> PathBuf {
        self.data_home().join("com.minimalstuff.tunneler")
    }

    fn autostart_entry(&self) -> PathBuf {
        self.config_home().join("autostart/tunneler.desktop")
    }

    fn ssh_config(&self) -> PathBuf {
        self.home().join(".ssh/config")
    }

    fn socket(&self) -> PathBuf {
        self.root.path().join("tunneler.sock")
    }

    fn empty_ssh_config(&self) -> PathBuf {
        let path = self.root.path().join("ssh_config");
        fs::write(&path, "").unwrap();
        path
    }

    fn command(&self, binary: &Path, arguments: &[&str]) -> Command {
        let mut command = Command::new(binary);
        command
            .arg("--socket")
            .arg(self.socket())
            .args(arguments)
            .stdin(Stdio::null())
            .env("HOME", self.home())
            .env("XDG_CONFIG_HOME", self.config_home())
            .env("XDG_STATE_HOME", self.state_home())
            .env("XDG_DATA_HOME", self.data_home())
            .env("TUNNELER_NO_TRAY", "1")
            .env_remove("TUNNELER_CONFIG")
            .env_remove("TUNNELER_STATE")
            .env_remove("TUNNELER_LOG")
            .env_remove("NO_COLOR");
        command
    }

    fn purge(&self, arguments: &[&str]) -> Output {
        self.purge_with(Path::new(BINARY), arguments)
    }

    fn purge_with(&self, binary: &Path, arguments: &[&str]) -> Output {
        let mut all_arguments = vec!["purge"];
        all_arguments.extend_from_slice(arguments);
        self.command(binary, &all_arguments).output().unwrap()
    }

    fn spawn_daemon(&self, binary: &Path) -> Daemon {
        let ssh_config = self.empty_ssh_config();
        let child = self
            .command(
                binary,
                &["daemon", "--ssh-config", ssh_config.to_str().unwrap()],
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let daemon = Daemon { child };
        let socket = self.socket();
        wait_until("the daemon answers", || {
            UnixStream::connect(&socket).is_ok()
        });
        daemon
    }
}

/// A spawned daemon, killed when dropped.
struct Daemon {
    child: Child,
}

impl Daemon {
    fn wait_until_exited(&mut self) {
        let deadline = Instant::now() + WAIT;
        while self.child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "the daemon is still running");
            sleep(POLL_INTERVAL);
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A copy of the binary in its own directory, with the marker that turns
/// portable mode on, and the files a portable installation leaves.
struct PortableInstallation {
    directory: TempDir,
}

impl PortableInstallation {
    fn new() -> Self {
        let installation = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        fs::copy(BINARY, installation.executable()).unwrap();
        fs::write(installation.marker(), "").unwrap();
        fs::write(installation.tunnels_file(), TUNNELS_FILE).unwrap();
        fs::create_dir_all(installation.data().join("logs")).unwrap();
        fs::create_dir_all(installation.data().join("webview")).unwrap();
        fs::write(installation.data().join("state.toml"), STATE_FILE).unwrap();
        fs::write(installation.data().join("logs/tunneler.log"), "line\n").unwrap();
        fs::write(installation.data().join("webview/storage"), "cache").unwrap();
        installation
    }

    fn executable(&self) -> PathBuf {
        self.directory.path().join("tunneler")
    }

    fn marker(&self) -> PathBuf {
        self.directory.path().join("portable")
    }

    fn tunnels_file(&self) -> PathBuf {
        self.directory.path().join("tunnels.toml")
    }

    fn data(&self) -> PathBuf {
        self.directory.path().join("data")
    }
}

fn write_file(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
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

fn wait_until(description: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        sleep(POLL_INTERVAL);
    }
}

#[test]
fn should_remove_everything_tunneler_left_when_purging_with_yes() {
    let sandbox = Sandbox::new();

    let output = sandbox.purge(&["--yes"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(!sandbox.tunnels_file().exists());
    assert!(!sandbox.state_file().exists());
    assert!(!sandbox.log_directory().exists());
    assert!(!sandbox.webview_directory().exists());
    assert!(!sandbox.autostart_entry().exists());
}

#[test]
fn should_remove_the_tunneler_directories_left_empty_when_purging() {
    let sandbox = Sandbox::new();

    sandbox.purge(&["--yes"]);

    assert!(!sandbox.config_home().join("tunneler").exists());
    assert!(!sandbox.state_home().join("tunneler").exists());
    assert!(!sandbox.data_home().join("tunneler").exists());
}

#[test]
fn should_leave_the_ssh_directory_and_the_other_autostart_entries_alone_when_purging() {
    let sandbox = Sandbox::new();
    let other_entry = sandbox.config_home().join("autostart/other.desktop");
    write_file(&other_entry, "[Desktop Entry]\nName=Other\n");

    sandbox.purge(&["--yes"]);

    assert_eq!(
        fs::read_to_string(sandbox.ssh_config()).unwrap(),
        SSH_CONFIG
    );
    assert!(other_entry.exists());
}

#[test]
fn should_leave_an_autostart_entry_that_is_not_ours_alone_when_purging() {
    let sandbox = Sandbox::new();
    write_file(&sandbox.autostart_entry(), FOREIGN_AUTOSTART_ENTRY);

    let output = sandbox.purge(&["--yes"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(sandbox.autostart_entry().exists());
}

#[test]
fn should_keep_the_tunnels_and_the_state_when_purging_with_keep_config() {
    let sandbox = Sandbox::new();

    let output = sandbox.purge(&["--keep-config", "--yes"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(sandbox.tunnels_file()).unwrap(),
        TUNNELS_FILE
    );
    assert_eq!(
        fs::read_to_string(sandbox.state_file()).unwrap(),
        STATE_FILE
    );
}

#[test]
fn should_still_remove_the_logs_the_webview_and_the_login_entry_when_purging_with_keep_config() {
    let sandbox = Sandbox::new();

    sandbox.purge(&["--keep-config", "--yes"]);

    assert!(!sandbox.log_directory().exists());
    assert!(!sandbox.webview_directory().exists());
    assert!(!sandbox.autostart_entry().exists());
    assert!(!sandbox.data_home().join("tunneler").exists());
}

#[test]
fn should_list_what_would_be_removed_and_remove_nothing_when_purging_with_dry_run() {
    let sandbox = Sandbox::new();

    let output = sandbox.purge(&["--dry-run"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let listing = stdout(&output);
    assert!(listing.contains("Would remove"), "{listing}");
    assert!(
        listing.contains(&sandbox.tunnels_file().display().to_string()),
        "{listing}"
    );
    assert!(sandbox.tunnels_file().exists());
    assert!(sandbox.state_file().exists());
    assert!(sandbox.log_directory().exists());
    assert!(sandbox.webview_directory().exists());
    assert!(sandbox.autostart_entry().exists());
}

#[test]
fn should_exit_with_5_and_remove_nothing_when_purging_without_a_terminal_and_without_yes() {
    let sandbox = Sandbox::new();

    let output = sandbox.purge(&[]);

    assert_eq!(code(&output), EXIT_CONFIRMATION_REQUIRED);
    assert!(stderr(&output).contains("hint:"), "{}", stderr(&output));
    assert!(sandbox.tunnels_file().exists());
    assert!(sandbox.log_directory().exists());
}

#[test]
fn should_leave_the_files_given_with_config_and_state_alone_when_purging() {
    let sandbox = Sandbox::new();
    let custom_config = sandbox.root.path().join("mine.toml");
    let custom_state = sandbox.root.path().join("mine-state.toml");
    fs::write(&custom_config, TUNNELS_FILE).unwrap();
    fs::write(&custom_state, STATE_FILE).unwrap();

    let output = sandbox.purge(&[
        "--yes",
        "--config",
        custom_config.to_str().unwrap(),
        "--state",
        custom_state.to_str().unwrap(),
    ]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(custom_config.exists() && custom_state.exists());
    let listing = stdout(&output);
    assert!(
        listing.contains(&format!(
            "Left alone, given with --config or --state: {}",
            custom_config.display()
        )),
        "{listing}"
    );
}

#[test]
fn should_exit_with_1_and_still_remove_the_rest_when_one_removal_fails() {
    let sandbox = Sandbox::new();
    fs::remove_file(sandbox.tunnels_file()).unwrap();
    fs::create_dir_all(sandbox.tunnels_file().join("not-a-file")).unwrap();

    let output = sandbox.purge(&["--yes"]);

    assert_eq!(code(&output), EXIT_ERROR, "{}", stderr(&output));
    let errors = stderr(&output);
    assert!(
        errors.contains(&format!(
            "cannot remove {}",
            sandbox.tunnels_file().display()
        )),
        "{errors}"
    );
    assert!(!sandbox.log_directory().exists());
    assert!(!sandbox.state_file().exists());
}

#[test]
fn should_stop_the_running_daemon_before_removing_when_purging() {
    let sandbox = Sandbox::new();
    let mut daemon = sandbox.spawn_daemon(Path::new(BINARY));

    let output = sandbox.purge(&["--yes"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    daemon.wait_until_exited();
    assert!(!sandbox.log_directory().exists());
}

#[test]
fn should_purge_the_files_next_to_the_executable_when_portable() {
    let installation = PortableInstallation::new();
    let sandbox = Sandbox::new();

    let output = sandbox.purge_with(&installation.executable(), &["--yes"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(!installation.tunnels_file().exists());
    assert!(!installation.data().exists());
}

#[test]
fn should_keep_the_executable_and_the_marker_when_purging_a_portable_installation() {
    let installation = PortableInstallation::new();
    let sandbox = Sandbox::new();

    sandbox.purge_with(&installation.executable(), &["--yes"]);

    assert!(installation.executable().exists());
    assert!(installation.marker().exists());
}

#[test]
fn should_not_touch_the_user_directories_when_purging_a_portable_installation() {
    let installation = PortableInstallation::new();
    let sandbox = Sandbox::new();

    sandbox.purge_with(&installation.executable(), &["--yes"]);

    assert!(sandbox.tunnels_file().exists());
    assert!(sandbox.state_file().exists());
    assert!(sandbox.log_directory().exists());
}

#[test]
fn should_keep_the_portable_tunnels_and_state_when_purging_with_keep_config() {
    let installation = PortableInstallation::new();
    let sandbox = Sandbox::new();

    sandbox.purge_with(&installation.executable(), &["--keep-config", "--yes"]);

    assert!(installation.tunnels_file().exists());
    assert!(installation.data().join("state.toml").exists());
    assert!(!installation.data().join("logs").exists());
    assert!(!installation.data().join("webview").exists());
}

#[test]
fn should_write_the_daemon_logs_next_to_the_executable_when_portable() {
    let installation = PortableInstallation::new();
    fs::remove_dir_all(installation.data()).unwrap();
    let sandbox = Sandbox::new();
    let _daemon = sandbox.spawn_daemon(&installation.executable());

    wait_until("a log file in the portable data directory", || {
        fs::read_dir(installation.data().join("logs"))
            .is_ok_and(|mut entries| entries.next().is_some())
    });

    let user_logs = fs::read_dir(sandbox.log_directory()).unwrap().count();
    assert_eq!(
        user_logs, 1,
        "only the seeded log file is in the user directory"
    );
}
