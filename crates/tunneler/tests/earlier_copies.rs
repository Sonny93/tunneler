//! End-to-end tests of the copy of an earlier portable copy at daemon start:
//! the real binary, in a folder next to a folder that looks like an earlier
//! portable copy, against throwaway home and XDG directories. The OS keyring
//! is never involved: the seeded `state.toml` lists no passphrase.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use tempfile::TempDir;

const BINARY: &str = env!("CARGO_BIN_EXE_tunneler");
const WAIT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const SERVING_MESSAGE: &str = "the daemon is serving";
const EARLIER_TUNNELS: &str = "version = 1\n# earlier copy\n";
const EARLIER_STATE: &str = "version = 1\n# earlier state\n";
const USER_TUNNELS: &str = "version = 1\n# already in the user directory\n";

/// A folder holding the copy of the binary that runs (`0.6.0`), an earlier
/// portable copy next to it (`0.5.0`) and the home directory.
struct Scenario {
    root: TempDir,
}

impl Scenario {
    fn new() -> Self {
        let scenario = Self {
            root: tempfile::tempdir().unwrap(),
        };
        set_mode(scenario.root.path(), 0o700);
        fs::create_dir_all(scenario.running_directory()).unwrap();
        fs::copy(BINARY, scenario.executable()).unwrap();
        fs::create_dir_all(scenario.earlier_directory().join("data")).unwrap();
        set_mode(&scenario.earlier_directory(), 0o755);
        write_file(&scenario.earlier_directory().join("portable"), "");
        write_file(&scenario.earlier_tunnels(), EARLIER_TUNNELS);
        write_file(&scenario.earlier_state(), EARLIER_STATE);
        scenario
    }

    fn running_directory(&self) -> PathBuf {
        self.root.path().join("0.6.0")
    }

    fn executable(&self) -> PathBuf {
        self.running_directory().join("tunneler")
    }

    fn earlier_directory(&self) -> PathBuf {
        self.root.path().join("0.5.0")
    }

    fn earlier_tunnels(&self) -> PathBuf {
        self.earlier_directory().join("tunnels.toml")
    }

    fn earlier_state(&self) -> PathBuf {
        self.earlier_directory().join("data/state.toml")
    }

    fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    fn user_tunnels(&self) -> PathBuf {
        self.home().join(".config/tunneler/tunnels.toml")
    }

    fn user_state(&self) -> PathBuf {
        self.home().join(".local/state/tunneler/state.toml")
    }

    fn socket(&self) -> PathBuf {
        self.root.path().join("tunneler.sock")
    }

    fn log_mentions(&self, text: &str) -> bool {
        let log_directory = self.home().join(".local/share/tunneler/logs");
        let Ok(entries) = fs::read_dir(log_directory) else {
            return false;
        };
        entries
            .filter_map(Result::ok)
            .filter_map(|entry| fs::read_to_string(entry.path()).ok())
            .any(|content| content.contains(text))
    }

    fn spawn_daemon(&self) -> Daemon {
        let ssh_config = self.root.path().join("ssh_config");
        fs::write(&ssh_config, "").unwrap();
        let mut command = Command::new(self.executable());
        command
            .arg("--socket")
            .arg(self.socket())
            .arg("daemon")
            .arg("--ssh-config")
            .arg(&ssh_config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .env("HOME", self.home())
            .env("XDG_CONFIG_HOME", self.home().join(".config"))
            .env("XDG_STATE_HOME", self.home().join(".local/state"))
            .env("XDG_DATA_HOME", self.home().join(".local/share"))
            .env("TUNNELER_NO_TRAY", "1")
            .env_remove("TUNNELER_CONFIG")
            .env_remove("TUNNELER_STATE")
            .env_remove("TUNNELER_LOG");
        let child = retry_while_busy(|| command.spawn());
        let daemon = Daemon { child };
        let socket = self.socket();
        wait_until("the daemon answers", || {
            UnixStream::connect(&socket).is_ok()
        });
        wait_until("the daemon serves", || self.log_mentions(SERVING_MESSAGE));
        daemon
    }
}

/// A spawned daemon, killed when dropped.
struct Daemon {
    child: Child,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Runs `start` until the copied executable is no longer busy: while a test
/// thread writes a copy of the binary, a child forked by another test thread
/// inherits that write descriptor until it calls `exec`.
fn retry_while_busy<T>(mut start: impl FnMut() -> io::Result<T>) -> T {
    let deadline = Instant::now() + WAIT;
    loop {
        match start() {
            Err(error)
                if error.kind() == io::ErrorKind::ExecutableFileBusy
                    && Instant::now() < deadline =>
            {
                sleep(POLL_INTERVAL);
            }
            outcome => return outcome.unwrap(),
        }
    }
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

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).unwrap();
    set_mode(path, 0o644);
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn should_copy_the_tunnels_and_the_state_of_the_earlier_copy_when_the_daemon_starts() {
    let scenario = Scenario::new();

    let _daemon = scenario.spawn_daemon();

    assert_eq!(
        (read(&scenario.user_tunnels()), read(&scenario.user_state())),
        (EARLIER_TUNNELS.to_owned(), EARLIER_STATE.to_owned())
    );
}

#[test]
fn should_leave_the_earlier_copy_untouched_when_the_daemon_copies_it() {
    let scenario = Scenario::new();

    let _daemon = scenario.spawn_daemon();

    assert_eq!(
        (
            read(&scenario.earlier_tunnels()),
            read(&scenario.earlier_state())
        ),
        (EARLIER_TUNNELS.to_owned(), EARLIER_STATE.to_owned())
    );
}

#[test]
fn should_keep_the_existing_tunnels_file_when_the_daemon_starts_next_to_an_earlier_copy() {
    let scenario = Scenario::new();
    fs::create_dir_all(scenario.user_tunnels().parent().unwrap()).unwrap();
    fs::write(scenario.user_tunnels(), USER_TUNNELS).unwrap();

    let _daemon = scenario.spawn_daemon();

    assert_eq!(
        (
            read(&scenario.user_tunnels()),
            scenario.user_state().exists()
        ),
        (USER_TUNNELS.to_owned(), false)
    );
}
