use std::path::{Path, PathBuf};

use russh::keys::PublicKey;
use tempfile::TempDir;

/// The host alias [`Workspace::write_config`] defines.
pub const ALIAS: &str = "testhost";
/// The remote login name the generated `ssh_config` uses.
pub const REMOTE_USER: &str = "alice";

/// One `Host` block of a multi-host `ssh_config`.
pub struct HostSpec<'a> {
    /// The alias the block defines.
    pub alias: &'a str,
    /// The loopback port the alias connects to.
    pub port: u16,
    /// Directives written before the defaults, so that they win.
    pub extra_lines: Vec<&'a str>,
}

/// A throwaway directory holding the client side of a test: keys,
/// `ssh_config` and `known_hosts`.
pub struct Workspace {
    directory: TempDir,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Workspace {
    /// Creates an empty directory, removed when the workspace is dropped.
    pub fn new() -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
        }
    }

    /// The directory.
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// A path inside the directory.
    pub fn file(&self, name: &str) -> PathBuf {
        self.path().join(name)
    }

    /// The `known_hosts` file the generated `ssh_config` uses.
    pub fn known_hosts_path(&self) -> PathBuf {
        self.file("known_hosts")
    }

    /// The content of the `known_hosts` file, empty when it does not exist.
    pub fn known_hosts_content(&self) -> String {
        std::fs::read_to_string(self.known_hosts_path()).unwrap_or_default()
    }

    /// Records `key` for `lookup_name` in the `known_hosts` file.
    pub fn seed_known_hosts(&self, lookup_name: &str, key: &PublicKey) {
        self.append_known_hosts_line(&format!("{lookup_name} {}", key.to_openssh().unwrap()));
    }

    /// Records a `@revoked` line for `key` and `lookup_name`.
    pub fn seed_revoked_key(&self, lookup_name: &str, key: &PublicKey) {
        self.append_known_hosts_line(&format!(
            "@revoked {lookup_name} {}",
            key.to_openssh().unwrap()
        ));
    }

    fn append_known_hosts_line(&self, line: &str) {
        let mut content = self.known_hosts_content();
        content.push_str(line);
        content.push('\n');
        std::fs::write(self.known_hosts_path(), content).unwrap();
    }

    /// Writes an `ssh_config` for [`ALIAS`] pointing at `127.0.0.1:port`.
    ///
    /// `extra_lines` come first and win over the defaults that follow
    /// (`StrictHostKeyChecking no`, a workspace `known_hosts`, no global one).
    pub fn write_config(&self, port: u16, extra_lines: &[&str]) -> PathBuf {
        self.write_hosts_config(&[HostSpec {
            alias: ALIAS,
            port,
            extra_lines: extra_lines.to_vec(),
        }])
    }

    /// Writes an `ssh_config` with one block per host, each built like
    /// [`Workspace::write_config`] builds its single block.
    pub fn write_hosts_config(&self, hosts: &[HostSpec]) -> PathBuf {
        let blocks: Vec<String> = hosts.iter().map(|host| self.host_block(host)).collect();
        let path = self.file("ssh_config");
        std::fs::write(&path, blocks.join("\n")).unwrap();
        path
    }

    fn host_block(&self, host: &HostSpec) -> String {
        let mut lines = vec![format!("Host {}", host.alias)];
        lines.extend(host.extra_lines.iter().map(|line| format!("  {line}")));
        lines.push("  HostName 127.0.0.1".to_owned());
        lines.push(format!("  Port {}", host.port));
        lines.push(format!("  User {REMOTE_USER}"));
        lines.push("  StrictHostKeyChecking no".to_owned());
        lines.push(format!(
            "  UserKnownHostsFile {}",
            self.known_hosts_path().display()
        ));
        lines.push("  GlobalKnownHostsFile none".to_owned());
        lines.join("\n")
    }
}
