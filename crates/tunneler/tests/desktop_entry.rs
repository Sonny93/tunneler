//! End-to-end tests of `tunneler desktop-entry` and of what `tunneler purge`
//! removes of it: the real binary, copied next to a `tunneler-ui` file, against
//! throwaway home and XDG directories.
#![cfg(all(unix, not(target_os = "macos")))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use tempfile::TempDir;

const BINARY: &str = env!("CARGO_BIN_EXE_tunneler");
const BUSY_WAIT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const EXIT_ERROR: i32 = 1;
const ENTRY_FILE_NAME: &str = "com.minimalstuff.tunneler.desktop";
const ICON_FILE_NAME: &str = "com.minimalstuff.tunneler.png";
const ICON_SIZES: [&str; 3] = ["32x32", "128x128", "256x256"];
const FOREIGN_ENTRY: &str = "[Desktop Entry]\nName=Someone else\n";
const OWNED_AUTOSTART_ENTRY: &str =
    "[Desktop Entry]\nExec=/old/tunneler daemon\nX-Tunneler-Autostart=true\n";
const STATE_FILE: &str = "version = 1\n";

/// A home directory with its XDG directories.
struct Sandbox {
    root: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    fn config_home(&self) -> PathBuf {
        self.home().join(".config")
    }

    fn data_home(&self) -> PathBuf {
        self.home().join(".local/share")
    }

    fn entry(&self) -> PathBuf {
        self.data_home().join("applications").join(ENTRY_FILE_NAME)
    }

    fn icon(&self, size: &str) -> PathBuf {
        self.data_home()
            .join("icons/hicolor")
            .join(size)
            .join("apps")
            .join(ICON_FILE_NAME)
    }

    fn icons(&self) -> Vec<PathBuf> {
        ICON_SIZES.iter().map(|size| self.icon(size)).collect()
    }

    fn autostart_entry(&self) -> PathBuf {
        self.config_home().join("autostart/tunneler.desktop")
    }

    fn run(&self, installation: &Installation, arguments: &[&str]) -> Output {
        let mut command = Command::new(installation.executable());
        command
            .arg("--socket")
            .arg(self.root.path().join("tunneler.sock"))
            .args(arguments)
            .stdin(Stdio::null())
            .env("HOME", self.home())
            .env("XDG_CONFIG_HOME", self.config_home())
            .env("XDG_STATE_HOME", self.home().join(".local/state"))
            .env("XDG_DATA_HOME", self.data_home())
            .env_remove("TUNNELER_CONFIG")
            .env_remove("TUNNELER_STATE")
            .env_remove("TUNNELER_LOG")
            .env_remove("NO_COLOR");
        retry_while_busy(|| command.output())
    }

    fn install(&self, installation: &Installation) -> Output {
        self.run(installation, &["desktop-entry", "install"])
    }

    fn uninstall(&self, installation: &Installation) -> Output {
        self.run(installation, &["desktop-entry", "uninstall"])
    }

    fn purge(&self, installation: &Installation, arguments: &[&str]) -> Output {
        let mut all_arguments = vec!["purge"];
        all_arguments.extend_from_slice(arguments);
        self.run(installation, &all_arguments)
    }
}

/// A folder holding a copy of `tunneler` and, optionally, a `tunneler-ui`.
struct Installation {
    directory: TempDir,
}

impl Installation {
    fn with_ui_program() -> Self {
        let installation = Self::without_ui_program();
        fs::write(installation.ui_program(), "").unwrap();
        installation
    }

    fn without_ui_program() -> Self {
        let installation = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        fs::copy(BINARY, installation.executable()).unwrap();
        installation
    }

    fn executable(&self) -> PathBuf {
        self.directory.path().join("tunneler")
    }

    fn ui_program(&self) -> PathBuf {
        self.directory.path().join("tunneler-ui")
    }
}

/// Runs `start` until the copied executable is no longer busy: a child forked
/// by another test thread may hold the write descriptor of the copy until it
/// calls `exec`, and the kernel refuses to run the copy (`ETXTBSY`) meanwhile.
fn retry_while_busy<T>(mut start: impl FnMut() -> io::Result<T>) -> T {
    let deadline = Instant::now() + BUSY_WAIT;
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

#[test]
fn should_write_the_entry_and_the_three_icons_when_installing() {
    let sandbox = Sandbox::new();
    let installation = Installation::with_ui_program();

    let output = sandbox.install(&installation);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let entry = fs::read_to_string(sandbox.entry()).unwrap();
    assert!(
        entry.contains(&format!("\nExec={}\n", installation.ui_program().display())),
        "{entry}"
    );
    assert!(entry.contains("\nX-Tunneler-MenuEntry=true\n"), "{entry}");
    assert!(sandbox.icons().iter().all(|icon| icon.is_file()));
    assert!(stdout(&output).contains(&sandbox.entry().display().to_string()));
}

#[test]
fn should_rewrite_the_exec_when_installing_again_from_another_folder() {
    let sandbox = Sandbox::new();
    sandbox.install(&Installation::with_ui_program());
    let newer = Installation::with_ui_program();

    let output = sandbox.install(&newer);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let entry = fs::read_to_string(sandbox.entry()).unwrap();
    assert!(
        entry.contains(&format!("\nExec={}\n", newer.ui_program().display())),
        "{entry}"
    );
    assert!(
        entry.contains(&format!("\nTryExec={}\n", newer.ui_program().display())),
        "{entry}"
    );
}

#[test]
fn should_refuse_and_leave_a_foreign_entry_byte_identical_when_installing() {
    let sandbox = Sandbox::new();
    write_file(&sandbox.entry(), FOREIGN_ENTRY);

    let output = sandbox.install(&Installation::with_ui_program());

    assert_eq!(code(&output), EXIT_ERROR);
    assert!(stderr(&output).contains("not written by Tunneler"));
    assert_eq!(fs::read(sandbox.entry()).unwrap(), FOREIGN_ENTRY.as_bytes());
    assert!(sandbox.icons().iter().all(|icon| !icon.exists()));
}

#[test]
fn should_write_nothing_when_the_ui_program_is_not_next_to_the_executable() {
    let sandbox = Sandbox::new();

    let output = sandbox.install(&Installation::without_ui_program());

    assert_eq!(code(&output), EXIT_ERROR);
    assert!(
        stderr(&output).contains("tunneler-ui"),
        "{}",
        stderr(&output)
    );
    assert!(!sandbox.entry().exists());
    assert!(sandbox.icons().iter().all(|icon| !icon.exists()));
}

#[test]
fn should_point_an_owned_autostart_entry_at_this_executable_when_installing() {
    let sandbox = Sandbox::new();
    write_file(&sandbox.autostart_entry(), OWNED_AUTOSTART_ENTRY);
    let installation = Installation::with_ui_program();

    let output = sandbox.install(&installation);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let autostart = fs::read_to_string(sandbox.autostart_entry()).unwrap();
    assert!(
        autostart.contains(&format!(
            "\nExec={} daemon\n",
            installation.executable().display()
        )),
        "{autostart}"
    );
}

#[test]
fn should_not_create_the_autostart_entry_when_installing_and_there_is_none() {
    let sandbox = Sandbox::new();

    sandbox.install(&Installation::with_ui_program());

    assert!(!sandbox.autostart_entry().exists());
}

#[test]
fn should_leave_a_foreign_autostart_entry_alone_when_installing() {
    let sandbox = Sandbox::new();
    write_file(&sandbox.autostart_entry(), FOREIGN_ENTRY);

    sandbox.install(&Installation::with_ui_program());

    assert_eq!(
        fs::read_to_string(sandbox.autostart_entry()).unwrap(),
        FOREIGN_ENTRY
    );
}

#[test]
fn should_remove_the_entry_and_the_icons_when_uninstalling() {
    let sandbox = Sandbox::new();
    let installation = Installation::with_ui_program();
    sandbox.install(&installation);

    let output = sandbox.uninstall(&installation);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(!sandbox.entry().exists());
    assert!(sandbox.icons().iter().all(|icon| !icon.exists()));
}

#[test]
fn should_remove_only_the_owned_files_when_uninstalling() {
    let sandbox = Sandbox::new();
    let installation = Installation::with_ui_program();
    sandbox.install(&installation);
    let other_entry = sandbox.data_home().join("applications/other.desktop");
    let other_icon = sandbox.icon("32x32").with_file_name("other.png");
    write_file(&other_entry, FOREIGN_ENTRY);
    write_file(&other_icon, "icon");

    sandbox.uninstall(&installation);

    assert!(other_entry.exists() && other_icon.exists());
}

#[test]
fn should_refuse_and_keep_everything_when_uninstalling_a_foreign_entry() {
    let sandbox = Sandbox::new();
    let installation = Installation::with_ui_program();
    sandbox.install(&installation);
    write_file(&sandbox.entry(), FOREIGN_ENTRY);

    let output = sandbox.uninstall(&installation);

    assert_eq!(code(&output), EXIT_ERROR);
    assert_eq!(fs::read(sandbox.entry()).unwrap(), FOREIGN_ENTRY.as_bytes());
    assert!(sandbox.icons().iter().all(|icon| icon.is_file()));
}

#[test]
fn should_succeed_when_uninstalling_without_an_entry() {
    let sandbox = Sandbox::new();

    let output = sandbox.uninstall(&Installation::with_ui_program());

    assert_eq!(code(&output), 0, "{}", stderr(&output));
}

#[test]
fn should_remove_the_entry_and_the_icons_but_not_the_foreign_files_when_purging() {
    let sandbox = Sandbox::new();
    let installation = Installation::with_ui_program();
    sandbox.install(&installation);
    let other_entry = sandbox.data_home().join("applications/other.desktop");
    let other_icon = sandbox.icon("128x128").with_file_name("other.png");
    write_file(&other_entry, FOREIGN_ENTRY);
    write_file(&other_icon, "icon");
    write_file(
        &sandbox.home().join(".local/state/tunneler/state.toml"),
        STATE_FILE,
    );

    let output = sandbox.purge(&installation, &["--yes"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(!sandbox.entry().exists());
    assert!(sandbox.icons().iter().all(|icon| !icon.exists()));
    assert!(other_entry.exists() && other_icon.exists());
}

#[test]
fn should_remove_the_entry_when_purging_with_keep_config() {
    let sandbox = Sandbox::new();
    let installation = Installation::with_ui_program();
    sandbox.install(&installation);

    let output = sandbox.purge(&installation, &["--keep-config", "--yes"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(!sandbox.entry().exists());
    assert!(sandbox.icons().iter().all(|icon| !icon.exists()));
}

#[test]
fn should_list_the_entry_and_keep_it_when_purging_with_dry_run() {
    let sandbox = Sandbox::new();
    let installation = Installation::with_ui_program();
    sandbox.install(&installation);

    let output = sandbox.purge(&installation, &["--dry-run"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).contains("application menu entry"),
        "{}",
        stdout(&output)
    );
    assert!(sandbox.entry().exists());
}

#[test]
fn should_leave_a_foreign_entry_at_the_same_path_alone_when_purging() {
    let sandbox = Sandbox::new();
    write_file(&sandbox.entry(), FOREIGN_ENTRY);

    let output = sandbox.purge(&Installation::with_ui_program(), &["--yes"]);

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(fs::read(sandbox.entry()).unwrap(), FOREIGN_ENTRY.as_bytes());
}
