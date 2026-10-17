#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Differential tests: every corpus configuration is resolved by this crate
//! and by `ssh -G -F <file> <alias>`, and the modeled keys are compared.
//!
//! The corpus is tuned to Debian's OpenSSH 10.0 (`OpenSSH_10.0`, including
//! Debian's patches). The tests only run on Linux, and are skipped, with a
//! note on stderr, when `ssh` is not on `PATH` or does not report that version.
//!
//! Corpus files live in `tests/corpus/*.conf`. Their header comments drive the
//! test:
//!
//! - `# aliases: a b c` lists the aliases to query;
//! - `# expect: error` states that both resolvers must fail (the error text is
//!   not compared);
//! - `# home: yes` runs `ssh` with `HOME` pointing at `<tempdir>/home` and
//!   resolves with the same home, so relative `Include` paths can be tested
//!   without touching the real `~/.ssh`.
//!
//! A line `#@file <relative path>` starts another file of the case, written
//! under the tempdir. The text before the first marker is the main
//! configuration. `@DIR@` and `@USER@` are replaced by the tempdir and the
//! local user name.
//!
//! Both sides are normalized into `ssh -G`'s `key value` lines (lowercase keys,
//! multi-valued lines in order, `true`/`false` read as `yes`/`no`). Three
//! values are printed unexpanded by `ssh -G`: `identityfile`, `certificatefile`
//! and `proxycommand`. For these, ssh's raw value is expanded with this crate's
//! token expander, using the user, host name, port, host key alias and jump host
//! that `ssh -G` itself reports, before comparing. `ProxyJump` values are
//! compared structurally (`ssh -G` prints only its last hop normalized).
//!
//! Known `ssh -G` quirks handled in the normalization: a `LocalForward` to a
//! Unix socket is also printed as a `dynamicforward`; and Debian patches two
//! behaviors (`ForwardX11Trusted` defaults to `yes`, and `BatchMode` raises an
//! unset `ServerAliveInterval` to 300). `forwardx11trusted` is therefore only
//! compared for the aliases named by a `# forwardx11trusted: a b` header, and
//! the keepalive patch is undone before comparing.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use pretty_assertions::Comparison;
use ssh_config::differential_support::{ExpansionContext, format_jump_chain, parse_jump_chain};
use ssh_config::{ConfigSources, LocalEnvironment, SystemEnvironment, resolve};

const MODELED_KEYS: &[&str] = &[
    "user",
    "hostname",
    "port",
    "addressfamily",
    "batchmode",
    "clearallforwardings",
    "compression",
    "exitonforwardfailure",
    "forwardx11",
    "forwardx11trusted",
    "gatewayports",
    "hashknownhosts",
    "identitiesonly",
    "kbdinteractiveauthentication",
    "passwordauthentication",
    "pubkeyauthentication",
    "stricthostkeychecking",
    "tcpkeepalive",
    "connectionattempts",
    "numberofpasswordprompts",
    "serveralivecountmax",
    "serveraliveinterval",
    "hostkeyalias",
    "identityagent",
    "ignoreunknown",
    "preferredauthentications",
    "tag",
    "dynamicforward",
    "localforward",
    "remoteforward",
    "identityfile",
    "certificatefile",
    "globalknownhostsfile",
    "userknownhostsfile",
    "sendenv",
    "setenv",
    "addkeystoagent",
    "forwardagent",
    "connecttimeout",
    "proxycommand",
    "proxyjump",
];
const BOOLEAN_LIKE_KEYS: &[&str] = &[
    "addressfamily",
    "batchmode",
    "clearallforwardings",
    "compression",
    "exitonforwardfailure",
    "forwardx11",
    "forwardx11trusted",
    "gatewayports",
    "hashknownhosts",
    "identitiesonly",
    "kbdinteractiveauthentication",
    "passwordauthentication",
    "pubkeyauthentication",
    "stricthostkeychecking",
    "tcpkeepalive",
    "addkeystoagent",
    "forwardagent",
];
const EXPECTED_SSH_VERSION_PREFIX: &str = "OpenSSH_10.0";
const SKIP_NOTE: &str = "skipping differential tests";

type Line = (String, String);

struct Case {
    name: String,
    aliases: Vec<String>,
    expects_error: bool,
    uses_home_override: bool,
    forward_x11_trusted_aliases: Vec<String>,
    files: Vec<(String, String)>,
}

struct HomeOverride {
    inner: SystemEnvironment,
    home: PathBuf,
}

impl LocalEnvironment for HomeOverride {
    fn variable(&self, name: &str) -> Option<String> {
        self.inner.variable(name)
    }

    fn user_name(&self) -> String {
        self.inner.user_name()
    }

    fn home_directory(&self) -> PathBuf {
        self.home.clone()
    }

    fn host_name(&self) -> String {
        self.inner.host_name()
    }

    fn user_id(&self) -> u32 {
        self.inner.user_id()
    }

    fn system_config_directory(&self) -> PathBuf {
        self.inner.system_config_directory()
    }

    fn command_succeeds(&self, command: &str) -> Result<bool, ssh_config::EnvironmentError> {
        self.inner.command_succeeds(command)
    }
}

fn is_validated_ssh_available() -> bool {
    let Ok(output) = Command::new("ssh").arg("-V").output() else {
        eprintln!("{SKIP_NOTE}: `ssh` is not on PATH");
        return false;
    };
    let version = String::from_utf8_lossy(&output.stderr);
    if version.starts_with(EXPECTED_SSH_VERSION_PREFIX) {
        return true;
    }
    eprintln!(
        "{SKIP_NOTE}: expected {EXPECTED_SSH_VERSION_PREFIX}, found {}",
        version.trim()
    );
    false
}

fn header_value<'text>(text: &'text str, key: &str) -> Option<&'text str> {
    let prefix = format!("# {key}:");
    text.lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(str::trim)
}

fn split_files(text: &str) -> Vec<(String, String)> {
    const FILE_MARKER: &str = "#@file ";
    let mut files = vec![("main.conf".to_owned(), String::new())];
    for line in text.lines() {
        if let Some(name) = line.strip_prefix(FILE_MARKER) {
            files.push((name.trim().to_owned(), String::new()));
            continue;
        }
        if let Some((_, content)) = files.last_mut() {
            content.push_str(line);
            content.push('\n');
        }
    }
    files
}

fn load_case(path: &Path) -> Case {
    let text = fs::read_to_string(path).unwrap();
    let name = path.file_stem().unwrap().to_string_lossy().into_owned();
    let aliases = header_value(&text, "aliases")
        .unwrap_or_else(|| panic!("{name}: missing `# aliases:` header"))
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    Case {
        expects_error: header_value(&text, "expect") == Some("error"),
        uses_home_override: header_value(&text, "home") == Some("yes"),
        forward_x11_trusted_aliases: header_value(&text, "forwardx11trusted")
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_owned)
            .collect(),
        files: split_files(&text),
        aliases,
        name,
    }
}

fn load_cases() -> Vec<Case> {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut paths: Vec<PathBuf> = fs::read_dir(corpus)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "conf")
        })
        .collect();
    paths.sort();
    paths.iter().map(|path| load_case(path)).collect()
}

fn materialize(case: &Case, directory: &Path, user_name: &str) -> PathBuf {
    for (relative_path, content) in &case.files {
        let substituted = content
            .replace("@DIR@", &directory.to_string_lossy())
            .replace("@USER@", user_name);
        let target = directory.join(relative_path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, substituted).unwrap();
    }
    fs::create_dir_all(directory.join("home/.ssh")).unwrap();
    directory.join("main.conf")
}

fn run_ssh(config: &Path, alias: &str, home: Option<&Path>) -> Result<String, String> {
    let mut command = Command::new("ssh");
    command
        .args(["-G", "-F"])
        .arg(config)
        .arg(alias)
        .stdin(Stdio::null());
    if let Some(home) = home {
        command.env("HOME", home);
    }
    let output = command.output().unwrap();
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(String::from_utf8_lossy(&output.stderr).into_owned())
}

fn parse_ssh_output(output: &str) -> Vec<Line> {
    output
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

fn first_value<'lines>(lines: &'lines [Line], key: &str) -> Option<&'lines str> {
    lines
        .iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value.as_str())
}

fn expansion_context(alias: &str, ssh_lines: &[Line]) -> ExpansionContext {
    let jump_chain = first_value(ssh_lines, "proxyjump")
        .and_then(parse_jump_chain)
        .unwrap_or_default();
    ExpansionContext {
        original_host: alias.to_owned(),
        hostname: first_value(ssh_lines, "hostname").unwrap().to_owned(),
        port: first_value(ssh_lines, "port").unwrap().parse().unwrap(),
        user: first_value(ssh_lines, "user").unwrap().to_owned(),
        host_key_alias: first_value(ssh_lines, "hostkeyalias").map(str::to_owned),
        jump_host: jump_chain
            .last()
            .map(|jump| jump.host.clone())
            .unwrap_or_default(),
    }
}

fn normalize_ssh_value(
    key: &str,
    value: &str,
    context: &ExpansionContext,
    environment: &impl LocalEnvironment,
) -> Option<String> {
    match key {
        "identityfile" if value.eq_ignore_ascii_case("none") => None,
        "identityfile" | "certificatefile" => {
            Some(context.expand_path(value, environment).unwrap())
        }
        "proxycommand" => Some(context.expand_proxy_command(value, environment).unwrap()),
        "proxyjump" => Some(format_jump_chain(&parse_jump_chain(value).unwrap())),
        _ if BOOLEAN_LIKE_KEYS.contains(&key) => Some(map_boolean_words(value)),
        _ => Some(value.to_owned()),
    }
}

fn map_boolean_words(value: &str) -> String {
    value
        .split(' ')
        .map(|word| match word {
            "true" => "yes",
            "false" => "no",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn drop_unix_socket_dynamic_echoes(lines: Vec<Line>) -> Vec<Line> {
    let echoed_listens: Vec<String> = lines
        .iter()
        .filter(|(key, value)| key == "localforward" && is_unix_socket_target(value))
        .filter_map(|(_, value)| value.split(' ').next().map(str::to_owned))
        .collect();
    let mut remaining_echoes = echoed_listens;
    lines
        .into_iter()
        .filter(|(key, value)| {
            if key != "dynamicforward" {
                return true;
            }
            match remaining_echoes.iter().position(|listen| listen == value) {
                Some(index) => {
                    remaining_echoes.remove(index);
                    false
                }
                None => true,
            }
        })
        .collect()
}

fn is_unix_socket_target(forward_value: &str) -> bool {
    forward_value
        .split(' ')
        .nth(1)
        .is_some_and(|target| target.starts_with('/'))
}

fn normalize_ssh(
    alias: &str,
    output: &str,
    environment: &impl LocalEnvironment,
    compared_keys: &BTreeSet<&str>,
) -> Vec<Line> {
    let raw_lines = drop_unix_socket_dynamic_echoes(parse_ssh_output(output));
    let context = expansion_context(alias, &raw_lines);
    raw_lines
        .iter()
        .filter(|(key, _)| compared_keys.contains(key.as_str()))
        .filter_map(|(key, value)| {
            normalize_ssh_value(key, value, &context, environment)
                .map(|normalized| (key.clone(), normalized))
        })
        .collect()
}

fn into_sorted_text(mut lines: Vec<Line>) -> String {
    lines.sort_by(|left, right| left.0.cmp(&right.0));
    lines
        .iter()
        .map(|(key, value)| format!("{key} {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn compared_keys_for(case: &Case, alias: &str) -> BTreeSet<&'static str> {
    let mut keys: BTreeSet<&'static str> = MODELED_KEYS.iter().copied().collect();
    if !case
        .forward_x11_trusted_aliases
        .iter()
        .any(|candidate| candidate == alias)
    {
        keys.remove("forwardx11trusted");
    }
    keys
}

/// Debian raises an unset `ServerAliveInterval` to 300 seconds under `BatchMode`.
fn undo_batch_mode_keepalive_patch(lines: &mut [Line]) {
    let is_batch_mode = lines
        .iter()
        .any(|(key, value)| key == "batchmode" && value == "yes");
    if !is_batch_mode {
        return;
    }
    for (key, value) in lines.iter_mut() {
        if key == "serveraliveinterval" && value == "300" {
            "0".clone_into(value);
        }
    }
}

fn check_alias(
    case: &Case,
    config: &Path,
    alias: &str,
    system: &SystemEnvironment,
    home: &Path,
) -> Result<(), String> {
    let label = format!("{}/{alias}", case.name);
    let ssh_result = run_ssh(config, alias, case.uses_home_override.then_some(home));
    let sources = ConfigSources::user_only(config);
    let environment = HomeOverride {
        inner: system.clone(),
        home: if case.uses_home_override {
            home.to_path_buf()
        } else {
            system.home_directory()
        },
    };
    let our_result = resolve(&sources, alias, &environment);
    if case.expects_error {
        return match (ssh_result, our_result) {
            (Err(_), Err(_)) => Ok(()),
            (Ok(_), _) => Err(format!("{label}: ssh unexpectedly succeeded")),
            (_, Ok(_)) => Err(format!("{label}: resolve unexpectedly succeeded")),
        };
    }
    let output = ssh_result.map_err(|stderr| format!("{label}: ssh failed: {stderr}"))?;
    let resolution = our_result.map_err(|error| format!("{label}: resolve failed: {error}"))?;
    let compared_keys = compared_keys_for(case, alias);
    let mut ssh_lines = normalize_ssh(alias, &output, &environment, &compared_keys);
    undo_batch_mode_keepalive_patch(&mut ssh_lines);
    let ssh_text = into_sorted_text(ssh_lines);
    let our_text = into_sorted_text(
        resolution
            .config
            .ssh_g_options()
            .into_iter()
            .filter(|(key, _)| compared_keys.contains(key.as_str()))
            .collect(),
    );
    if our_text == ssh_text {
        return Ok(());
    }
    Err(format!(
        "{label}: left is this crate, right is ssh -G\n{}",
        Comparison::new(
            &our_text.lines().collect::<Vec<_>>(),
            &ssh_text.lines().collect::<Vec<_>>()
        )
    ))
}

#[test]
fn should_match_ssh_when_resolving_every_corpus_configuration() {
    if !is_validated_ssh_available() {
        return;
    }
    let system = SystemEnvironment::new().unwrap();
    let cases = load_cases();
    assert!(!cases.is_empty(), "the corpus is empty");
    let mut failures = Vec::new();
    let mut compared_count = 0;
    for case in &cases {
        let directory = tempfile::tempdir().unwrap();
        let config = materialize(case, directory.path(), &system.user_name());
        let home = directory.path().join("home");
        for alias in &case.aliases {
            compared_count += 1;
            if let Err(failure) = check_alias(case, &config, alias, &system, &home) {
                failures.push(failure);
            }
        }
    }
    eprintln!(
        "differential: {} files, {compared_count} (file, alias) cases",
        cases.len()
    );
    assert!(failures.is_empty(), "\n\n{}", failures.join("\n\n"));
}
