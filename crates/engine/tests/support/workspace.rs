use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::{
    Connector, ConnectorDependencies, CredentialPrompter, HostKeyVerifier, SshConfigResolver,
};
use russh::keys::PublicKey;
use ssh_config::ConfigSources;
use tempfile::TempDir;

use super::environment::FakeEnvironment;
use super::scripted::{ScriptedPrompter, ScriptedVerifier};

pub const ALIAS: &str = "testhost";
pub const REMOTE_USER: &str = "alice";

/// A throwaway directory holding the client side of a test: keys,
/// `ssh_config` and `known_hosts`.
pub struct Workspace {
    directory: TempDir,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
        }
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.path().join(name)
    }

    pub fn known_hosts_path(&self) -> PathBuf {
        self.file("known_hosts")
    }

    pub fn known_hosts_content(&self) -> String {
        std::fs::read_to_string(self.known_hosts_path()).unwrap_or_default()
    }

    pub fn seed_known_hosts(&self, lookup_name: &str, key: &PublicKey) {
        self.append_known_hosts_line(&format!("{lookup_name} {}", key.to_openssh().unwrap()));
    }

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
        let mut lines = vec![format!("Host {ALIAS}")];
        lines.extend(extra_lines.iter().map(|line| format!("  {line}")));
        lines.push("  HostName 127.0.0.1".to_owned());
        lines.push(format!("  Port {port}"));
        lines.push(format!("  User {REMOTE_USER}"));
        lines.push("  StrictHostKeyChecking no".to_owned());
        lines.push(format!(
            "  UserKnownHostsFile {}",
            self.known_hosts_path().display()
        ));
        lines.push("  GlobalKnownHostsFile none".to_owned());
        let path = self.file("ssh_config");
        std::fs::write(&path, lines.join("\n")).unwrap();
        path
    }

    pub fn connector(
        &self,
        config_path: &Path,
        prompter: Arc<dyn CredentialPrompter>,
        verifier: Arc<dyn HostKeyVerifier>,
    ) -> Connector {
        let environment = FakeEnvironment {
            home: self.path().join("home"),
        };
        let resolver = SshConfigResolver::new(ConfigSources::user_only(config_path), environment);
        Connector::new(ConnectorDependencies {
            resolver: Arc::new(resolver),
            prompter,
            verifier,
        })
    }
}

pub fn quiet_prompter() -> Arc<ScriptedPrompter> {
    Arc::new(ScriptedPrompter::default())
}

pub fn rejecting_verifier() -> Arc<ScriptedVerifier> {
    Arc::new(ScriptedVerifier::default())
}
