#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Differential tests against `ssh-keygen`, the OpenSSH oracle.
//!
//! `ssh-keyscan` needs a live sshd and is not exercised: the lines it emits
//! have the same `host key-type base64` format as the `.pub`-derived lines
//! built here.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use known_hosts::{Hashing, HostKey, HostLookup, KnownHosts, PresentedKey, Verdict, append_entry};
use pretty_assertions::assert_eq;
use tempfile::TempDir;

const SSH_KEYGEN: &str = "ssh-keygen";
const RSA_BITS: &str = "2048";
const KEY_TYPES: [&str; 3] = ["ed25519", "ecdsa", "rsa"];

struct Workspace {
    directory: TempDir,
    key_counter: usize,
}

impl Workspace {
    fn new() -> Option<Self> {
        if Command::new(SSH_KEYGEN).arg("-?").output().is_err() {
            eprintln!("skipping: `{SSH_KEYGEN}` is not installed");
            return None;
        }
        Some(Self {
            directory: tempfile::tempdir().unwrap(),
            key_counter: 0,
        })
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn generate_key(&mut self, key_type: &str) -> HostKey {
        self.key_counter += 1;
        let private_path = self.path(&format!("key{}", self.key_counter));
        let mut command = Command::new(SSH_KEYGEN);
        command
            .args(["-q", "-t", key_type, "-N", "", "-f"])
            .arg(&private_path);
        if key_type == "rsa" {
            command.args(["-b", RSA_BITS]);
        }
        assert!(command.output().unwrap().status.success());
        read_public_key(&self.path(&format!("key{}.pub", self.key_counter)))
    }

    fn generate_keys(&mut self, count: usize) -> Vec<HostKey> {
        (0..count)
            .map(|index| self.generate_key(KEY_TYPES[index % KEY_TYPES.len()]))
            .collect()
    }
}

fn read_public_key(path: &Path) -> HostKey {
    let content = fs::read_to_string(path).unwrap();
    let mut fields = content.split_whitespace();
    let algorithm = fields.next().unwrap();
    HostKey::from_base64(algorithm, fields.next().unwrap()).unwrap()
}

fn line(hosts: &str, key: &HostKey) -> String {
    format!("{hosts} {} {}\n", key.algorithm, key.to_base64())
}

fn write_known_hosts(workspace: &Workspace, entries: &[(&str, &HostKey)]) -> PathBuf {
    let path = workspace.path("known_hosts");
    let content: String = entries
        .iter()
        .map(|(hosts, key)| line(hosts, key))
        .collect();
    fs::write(&path, content).unwrap();
    path
}

fn find_with_ssh_keygen(path: &Path, name: &str) -> (bool, BTreeSet<usize>) {
    let output: Output = Command::new(SSH_KEYGEN)
        .args(["-F", name, "-f"])
        .arg(path)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    let lines = text
        .lines()
        .filter_map(|text_line| text_line.rsplit_once("found: line "))
        .map(|(_, number)| number.trim().parse().unwrap())
        .collect();
    (output.status.success(), lines)
}

fn literal_lookup(name: &str) -> HostLookup {
    HostLookup {
        host_key_alias: Some(name.to_owned()),
        ..HostLookup::new(name, 22)
    }
}

fn load(path: &Path) -> KnownHosts {
    let loaded = KnownHosts::load(&[path.to_path_buf()], &[]).unwrap();
    assert_eq!(loaded.warnings, vec![]);
    loaded.known_hosts
}

fn file_keys(path: &Path) -> Vec<(usize, HostKey)> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .enumerate()
        .filter_map(|(index, text_line)| {
            let mut fields = text_line.split_whitespace().skip(1);
            let algorithm = fields.next()?;
            let key = HostKey::from_base64(algorithm, fields.next()?).ok()?;
            Some((index + 1, key))
        })
        .collect()
}

fn is_known(known: &KnownHosts, name: &str, probe: &HostKey) -> bool {
    let verdict = known.check(&literal_lookup(name), &PresentedKey::Plain(probe.clone()));
    verdict != Verdict::Unknown
}

fn trusted_lines(known: &KnownHosts, name: &str, keys: &[(usize, HostKey)]) -> BTreeSet<usize> {
    keys.iter()
        .filter_map(|(_, key)| {
            match known.check(&literal_lookup(name), &PresentedKey::Plain(key.clone())) {
                Verdict::Trusted { line, .. } => Some(line),
                _ => None,
            }
        })
        .collect()
}

fn assert_agrees_with_ssh_keygen(
    path: &Path,
    names: &[&str],
    probe: &HostKey,
    compare_lines: bool,
) {
    let known = load(path);
    let keys = file_keys(path);
    for name in names {
        let (found, lines) = find_with_ssh_keygen(path, name);
        assert_eq!(
            is_known(&known, name, probe),
            found,
            "found state for {name}"
        );
        if compare_lines {
            assert_eq!(
                trusted_lines(&known, name, &keys),
                lines,
                "lines for {name}"
            );
        }
    }
}

const PLAIN_NAMES: [&str; 17] = [
    "alpha",
    "ALPHA",
    "a",
    "b",
    "[b]:2222",
    "[a]:2222",
    "x.example.com",
    "example.com",
    "web1.example.org",
    "web10.example.org",
    "web.example.org",
    "www.example.net",
    "secret.example.net",
    "[gamma]:2200",
    "gamma",
    "unknown",
    "[alpha]:22",
];

fn mixed_file(workspace: &mut Workspace) -> (PathBuf, HostKey) {
    let keys = workspace.generate_keys(7);
    let probe = workspace.generate_key("ed25519");
    let entries = [
        ("alpha", &keys[0]),
        ("a,[b]:2222", &keys[1]),
        ("*.example.com", &keys[2]),
        ("web?.example.org", &keys[3]),
        ("*.example.net,!secret.example.net", &keys[4]),
        ("[gamma]:2200", &keys[5]),
        ("alpha,gamma", &keys[6]),
    ];
    (write_known_hosts(workspace, &entries), probe)
}

#[test]
fn should_agree_with_ssh_keygen_when_file_has_plain_wildcard_negated_and_port_entries() {
    let Some(mut workspace) = Workspace::new() else {
        return;
    };
    let (path, probe) = mixed_file(&mut workspace);

    assert_agrees_with_ssh_keygen(&path, &PLAIN_NAMES, &probe, true);
}

#[test]
fn should_agree_with_ssh_keygen_when_file_was_hashed_by_ssh_keygen() {
    let Some(mut workspace) = Workspace::new() else {
        return;
    };
    let (path, probe) = mixed_file(&mut workspace);
    assert!(
        Command::new(SSH_KEYGEN)
            .arg("-H")
            .arg("-f")
            .arg(&path)
            .output()
            .unwrap()
            .status
            .success()
    );
    let hashed_names: Vec<&str> = PLAIN_NAMES
        .iter()
        .copied()
        .filter(|name| *name != "ALPHA")
        .collect();

    assert!(fs::read_to_string(&path).unwrap().contains("|1|"));
    assert_agrees_with_ssh_keygen(&path, &hashed_names, &probe, false);
}

#[test]
fn should_report_same_fingerprint_as_ssh_keygen_when_key_is_any_supported_type() {
    let Some(mut workspace) = Workspace::new() else {
        return;
    };
    for key_type in KEY_TYPES {
        let key = workspace.generate_key(key_type);
        let public_path = workspace.path(&format!("key{}.pub", workspace.key_counter));
        let output = Command::new(SSH_KEYGEN)
            .arg("-l")
            .arg("-f")
            .arg(&public_path)
            .output()
            .unwrap();

        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        assert_eq!(
            text.split_whitespace().nth(1),
            Some(key.fingerprint_sha256().as_str())
        );
    }
}

fn append_and_find(hashing: Hashing, lookup: &HostLookup, ssh_keygen_name: &str) {
    let Some(mut workspace) = Workspace::new() else {
        return;
    };
    let key = workspace.generate_key("ed25519");
    let path = workspace.path("known_hosts");
    fs::write(&path, "# no trailing newline").unwrap();

    append_entry(&path, lookup, &key, hashing).unwrap();

    let (found, lines) = find_with_ssh_keygen(&path, ssh_keygen_name);
    assert!(found, "ssh-keygen -F {ssh_keygen_name}");
    assert_eq!(lines, BTreeSet::from([2]));
}

#[test]
fn should_be_found_by_ssh_keygen_when_entry_is_appended_plain() {
    append_and_find(
        Hashing::Plain,
        &HostLookup::new("example.com", 22),
        "example.com",
    );
}

#[test]
fn should_be_found_by_ssh_keygen_when_entry_is_appended_hashed() {
    append_and_find(
        Hashing::Hashed,
        &HostLookup::new("example.com", 22),
        "example.com",
    );
}

#[test]
fn should_be_found_by_ssh_keygen_when_non_default_port_entry_is_appended_plain() {
    append_and_find(
        Hashing::Plain,
        &HostLookup::new("example.com", 2222),
        "[example.com]:2222",
    );
}

#[test]
fn should_be_found_by_ssh_keygen_when_non_default_port_entry_is_appended_hashed() {
    append_and_find(
        Hashing::Hashed,
        &HostLookup::new("example.com", 2222),
        "[example.com]:2222",
    );
}

#[test]
fn should_be_found_by_ssh_keygen_when_entry_is_appended_for_an_alias() {
    let lookup = HostLookup {
        host_key_alias: Some("jump".to_owned()),
        ..HostLookup::new("example.com", 2222)
    };

    append_and_find(Hashing::Hashed, &lookup, "jump");
}

#[test]
fn should_trust_appended_key_when_it_is_read_back() {
    let Some(mut workspace) = Workspace::new() else {
        return;
    };
    let key = workspace.generate_key("ecdsa");
    let path = workspace.path("known_hosts");
    let lookup = HostLookup::new("example.com", 2222);

    append_entry(&path, &lookup, &key, Hashing::Hashed).unwrap();

    let verdict = load(&path).check(&lookup, &PresentedKey::Plain(key));
    assert_eq!(verdict, Verdict::Trusted { path, line: 1 });
}
