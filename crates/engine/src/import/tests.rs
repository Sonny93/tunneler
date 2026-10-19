use std::path::PathBuf;

use pretty_assertions::assert_eq;
use ssh_config::{ConfigSources, EnvironmentError, LocalEnvironment};

use super::*;
use crate::host_config::{HostConfigError, HostConfigResolver, SshConfigResolver};

const NOVA_SCRIPT: &str = r#"$env:DISPLAY = "198.51.100.135:0.0"

$ports = @(
    3333 <# adonis #>,
    3334 <# api second port #>,
    24678 <# api's dev server #>,
    6006 <# ui catalog #>,
    8025 <# mail catcher #>
    5173 <# vite dev server #>
    4173 <# vite preview #>
    3030 <# slides preview server #>
)

$forwards = $ports | ForEach-Object { "-L ${_}:127.0.0.1:${_}" }

# Ce script ne fait plus de forward x11 parce que le client ssh via powershell n'embarque xauth.
# xlaunch / VcXsrv est lancé avec l'option "Disable access control" cochée

# Pour x11 forward faut soit lancer le serveur X sur le client avec l'option "Disable access control" cochée
# Soit utiliser un profil x11 forward d'un autre client ssh
ssh -v -N @forwards jules@198.51.100.157
"#;

const NOVA_CONFIG: &str = "Host nova\n  HostName 198.51.100.157\n  User jules\nHost other\n  HostName 198.51.100.90\n  User root\n";

struct TestEnvironment;

impl LocalEnvironment for TestEnvironment {
    fn variable(&self, _name: &str) -> Option<String> {
        None
    }

    fn user_name(&self) -> String {
        "alice".to_owned()
    }

    fn home_directory(&self) -> PathBuf {
        PathBuf::from("/home/alice")
    }

    fn host_name(&self) -> String {
        "laptop".to_owned()
    }

    fn user_id(&self) -> u32 {
        1000
    }

    fn system_config_directory(&self) -> PathBuf {
        PathBuf::from("/etc/ssh")
    }

    fn command_succeeds(&self, _command: &str) -> Result<bool, EnvironmentError> {
        Ok(false)
    }
}

struct FailingFor<Inner, Invalid> {
    inner: Inner,
    invalid: Invalid,
    failing_alias: &'static str,
}

impl<Inner: HostConfigResolver, Invalid: HostConfigResolver> HostConfigResolver
    for FailingFor<Inner, Invalid>
{
    fn resolve(&self, alias: &str) -> Result<ssh_config::ResolvedConfig, HostConfigError> {
        if alias == self.failing_alias {
            return self.invalid.resolve(alias);
        }
        self.inner.resolve(alias)
    }

    fn list_hosts(&self) -> Result<Vec<String>, HostConfigError> {
        self.inner.list_hosts()
    }
}

fn resolver_for(config: &str) -> (tempfile::TempDir, SshConfigResolver<TestEnvironment>) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config");
    std::fs::write(&path, config).unwrap();
    let resolver = SshConfigResolver::new(ConfigSources::user_only(path), TestEnvironment);
    (directory, resolver)
}

fn script_preview(script: &str) -> ImportPreview {
    let (_directory, resolver) = resolver_for(NOVA_CONFIG);
    preview_script(script, &resolver)
}

fn listen_ports(preview: &ImportPreview) -> Vec<u16> {
    preview
        .candidates
        .iter()
        .map(|candidate| candidate.listen_port)
        .collect()
}

#[test]
fn should_find_eight_local_candidates_when_script_is_the_powershell_example() {
    let preview = script_preview(NOVA_SCRIPT);

    assert_eq!(
        listen_ports(&preview),
        vec![3333, 3334, 24678, 6006, 8025, 5173, 4173, 3030]
    );
    assert_eq!(preview.warnings, Vec::new());
}

#[test]
fn should_attribute_candidates_to_alias_when_destination_matches_its_host_name() {
    let preview = script_preview(NOVA_SCRIPT);

    assert!(
        preview
            .candidates
            .iter()
            .all(|candidate| candidate.host.as_deref() == Some("nova"))
    );
}

#[test]
fn should_target_loopback_on_same_port_when_script_template_is_expanded() {
    let preview = script_preview(NOVA_SCRIPT);

    assert!(preview.candidates.iter().all(|candidate| {
        candidate.kind == TunnelKind::Local
            && candidate.target_host.as_deref() == Some("127.0.0.1")
            && candidate.target_port == Some(candidate.listen_port)
    }));
}

#[test]
fn should_keep_the_comment_of_each_candidate_when_ports_have_one() {
    let preview = script_preview(NOVA_SCRIPT);

    let labels: Vec<_> = preview
        .candidates
        .iter()
        .map(|candidate| candidate.comment.clone().unwrap())
        .collect();
    assert_eq!(
        labels,
        vec![
            "adonis",
            "api second port",
            "api's dev server",
            "ui catalog",
            "mail catcher",
            "vite dev server",
            "vite preview",
            "slides preview server"
        ]
    );
}

#[test]
fn should_report_port_lines_when_script_is_the_powershell_example() {
    let preview = script_preview(NOVA_SCRIPT);

    assert_eq!(preview.candidates[0].line, Some(4));
}

#[test]
fn should_read_every_forward_when_bash_script_uses_several_options() {
    let preview = script_preview("#!/bin/bash\nssh -N -L 3333:localhost:3333 -L 8080:db:80 nova\n");

    assert_eq!(listen_ports(&preview), vec![3333, 8080]);
    assert_eq!(preview.candidates[1].target_host.as_deref(), Some("db"));
    assert_eq!(preview.candidates[1].host.as_deref(), Some("nova"));
}

#[test]
fn should_read_every_forward_when_command_continues_with_backslashes() {
    let preview =
        script_preview("ssh -N \\\n  -L 3333:localhost:3333 \\\n  -L 8080:db:80 \\\n  nova\n");

    assert_eq!(listen_ports(&preview), vec![3333, 8080]);
    assert_eq!(preview.candidates[1].line, Some(3));
    assert_eq!(preview.candidates[1].host.as_deref(), Some("nova"));
}

#[test]
fn should_read_every_forward_when_command_continues_with_backticks() {
    let preview = script_preview("ssh -N `\n  -L 3333:localhost:3333 `\n  nova\n");

    assert_eq!(listen_ports(&preview), vec![3333]);
    assert_eq!(preview.candidates[0].host.as_deref(), Some("nova"));
}

#[test]
fn should_read_remote_and_dynamic_forwards_when_script_uses_r_and_d() {
    let preview = script_preview("ssh -R 9000:localhost:80 -D 1080 nova");

    let kinds: Vec<_> = preview.candidates.iter().map(|c| c.kind).collect();
    assert_eq!(kinds, vec![TunnelKind::Remote, TunnelKind::Dynamic]);
    assert_eq!(preview.candidates[1].target_host, None);
}

#[test]
fn should_read_bare_specs_when_text_has_no_ssh_command() {
    let preview = script_preview("-L 3333:localhost:3333\n-L8080:db:80\n-D 1080\n-R 9000:a:90\n");

    assert_eq!(listen_ports(&preview), vec![3333, 8080, 1080, 9000]);
    assert!(preview.candidates.iter().all(|c| c.host.is_none()));
}

#[test]
fn should_read_local_forwards_when_lines_are_bare_triples() {
    let preview = script_preview("3333:127.0.0.1:3333\n8080:db:80\n");

    assert_eq!(listen_ports(&preview), vec![3333, 8080]);
    assert_eq!(preview.candidates[0].kind, TunnelKind::Local);
}

#[test]
fn should_make_local_candidates_without_target_when_text_is_a_port_list() {
    let preview = script_preview("3333 5432\n");

    assert_eq!(listen_ports(&preview), vec![3333, 5432]);
    assert_eq!(preview.candidates[0].target_port, None);
}

#[test]
fn should_read_ports_when_list_is_comma_separated() {
    assert_eq!(listen_ports(&script_preview("3333,5432")), vec![3333, 5432]);
}

#[test]
fn should_read_ports_when_list_has_one_port_per_line_with_comments() {
    let preview = script_preview("3333 # adonis\n5432\n");

    assert_eq!(listen_ports(&preview), vec![3333, 5432]);
    assert_eq!(preview.candidates[0].comment.as_deref(), Some("adonis"));
}

#[test]
fn should_warn_about_destination_when_it_matches_no_alias() {
    let preview = script_preview("ssh -L 3333:localhost:3333 jules@192.0.2.150");

    assert_eq!(
        preview.warnings,
        vec![ImportWarning::UnknownDestination {
            destination: "jules@192.0.2.150".to_owned()
        }]
    );
    assert_eq!(preview.candidates[0].host, None);
}

#[test]
fn should_warn_about_destination_when_script_alias_is_not_configured() {
    let (_directory, resolver) = resolver_for("Host other\n  HostName 198.51.100.90\n");

    let preview = preview_script(NOVA_SCRIPT, &resolver);

    assert_eq!(
        preview.warnings,
        vec![ImportWarning::UnknownDestination {
            destination: "jules@198.51.100.157".to_owned()
        }]
    );
    assert_eq!(preview.candidates.len(), 8);
    assert!(preview.candidates.iter().all(|c| c.host.is_none()));
}

#[test]
fn should_not_match_alias_when_user_differs() {
    let preview = script_preview("ssh -L 3333:localhost:3333 root@198.51.100.157");

    assert_eq!(preview.candidates[0].host, None);
}

#[test]
fn should_match_alias_when_destination_user_comes_from_login_name_option() {
    let preview = script_preview("ssh -l jules -L 3333:localhost:3333 198.51.100.157");

    assert_eq!(preview.candidates[0].host.as_deref(), Some("nova"));
}

#[test]
fn should_prefer_alias_equal_to_host_when_another_alias_has_that_host_name() {
    let (_directory, resolver) = resolver_for(
        "Host first\n  HostName shared.example\nHost shared.example\n  HostName 198.51.100.10\n",
    );

    let preview = preview_script("ssh -L 1:a:1 shared.example", &resolver);

    assert_eq!(
        preview.candidates[0].host.as_deref(),
        Some("shared.example")
    );
}

#[test]
fn should_leave_host_empty_when_script_has_several_destinations_for_loose_specs() {
    let preview = script_preview("ssh nova\nssh other\n-L 3333:localhost:3333\n");

    assert_eq!(preview.candidates[0].host, None);
}

#[test]
fn should_warn_about_malformed_forward_when_specification_cannot_be_parsed() {
    let preview = script_preview("ssh -L nonsense -L 1:a:1 nova");

    assert_eq!(
        preview.warnings,
        vec![ImportWarning::MalformedForward {
            line: 1,
            text: "nonsense".to_owned()
        }]
    );
    assert_eq!(listen_ports(&preview), vec![1]);
}

#[test]
fn should_warn_about_malformed_forward_when_template_has_no_port_list() {
    let preview = script_preview("$f = \"-L ${p}:localhost:${p}\"\n");

    assert_eq!(
        preview.warnings,
        vec![ImportWarning::MalformedForward {
            line: 1,
            text: "${p}:localhost:${p}".to_owned()
        }]
    );
    assert_eq!(preview.candidates, Vec::new());
}

#[test]
fn should_warn_about_unsupported_forward_when_remote_forward_is_socks() {
    let preview = script_preview("ssh -R 9000 -L 1:a:1 nova");

    assert_eq!(
        preview.warnings,
        vec![ImportWarning::UnsupportedForward {
            line: Some(1),
            host: None,
            text: "9000".to_owned()
        }]
    );
}

#[test]
fn should_keep_first_candidate_when_listen_endpoint_is_repeated() {
    let preview = script_preview("ssh -L 3333:a:1 -L 3333:b:2 nova");

    assert_eq!(preview.candidates.len(), 1);
    assert_eq!(preview.candidates[0].target_host.as_deref(), Some("a"));
}

#[test]
fn should_keep_both_candidates_when_only_the_kind_differs() {
    let preview = script_preview("ssh -L 3333:a:1 -R 3333:b:2 nova");

    assert_eq!(preview.candidates.len(), 2);
}

#[test]
fn should_ignore_forwards_in_comments() {
    let preview = script_preview("# ssh -L 3333:a:1 nova\n");

    assert_eq!(preview.candidates, Vec::new());
}

#[test]
fn should_find_nothing_when_text_is_empty() {
    assert_eq!(
        script_preview(""),
        ImportPreview {
            candidates: Vec::new(),
            warnings: Vec::new()
        }
    );
}

const FORWARDING_CONFIG: &str = "\
Host web
  HostName 198.51.100.10
  LocalForward 3333 localhost:3333
  RemoteForward 9000 db:5432
  DynamicForward 1080
Host sock
  LocalForward 8080 /run/app.sock
  RemoteForward 7000
Host broken
  Port 22
Host *
  User me
";

#[test]
fn should_list_every_kind_when_ssh_config_declares_forwards() {
    let (_directory, resolver) = resolver_for(FORWARDING_CONFIG);

    let preview = preview_ssh_config(&resolver).unwrap();

    let found: Vec<_> = preview
        .candidates
        .iter()
        .map(|c| (c.host.clone().unwrap(), c.kind, c.listen_port))
        .collect();
    assert_eq!(
        found,
        vec![
            ("web".to_owned(), TunnelKind::Local, 3333),
            ("web".to_owned(), TunnelKind::Remote, 9000),
            ("web".to_owned(), TunnelKind::Dynamic, 1080),
        ]
    );
}

#[test]
fn should_keep_target_and_no_line_when_ssh_config_forward_is_imported() {
    let (_directory, resolver) = resolver_for(FORWARDING_CONFIG);

    let preview = preview_ssh_config(&resolver).unwrap();

    let local = &preview.candidates[0];
    assert_eq!(
        (
            local.target_host.as_deref(),
            local.target_port,
            local.line,
            local.comment.as_deref()
        ),
        (Some("localhost"), Some(3333), None, None)
    );
}

#[test]
fn should_warn_when_ssh_config_forward_uses_unix_socket_or_remote_socks() {
    let (_directory, resolver) = resolver_for(FORWARDING_CONFIG);

    let preview = preview_ssh_config(&resolver).unwrap();

    assert_eq!(
        preview.warnings[..2],
        [
            ImportWarning::UnsupportedForward {
                line: None,
                host: Some("sock".to_owned()),
                text: "8080 /run/app.sock".to_owned()
            },
            ImportWarning::UnsupportedForward {
                line: None,
                host: Some("sock".to_owned()),
                text: "7000 [socks]:0".to_owned()
            },
        ]
    );
}

#[test]
fn should_warn_when_ssh_config_alias_cannot_be_resolved() {
    let (_directory, inner) = resolver_for(FORWARDING_CONFIG);
    let (_invalid_directory, invalid) = resolver_for("Host broken\n  Port not-a-port\n");
    let resolver = FailingFor {
        inner,
        invalid,
        failing_alias: "broken",
    };

    let preview = preview_ssh_config(&resolver).unwrap();

    assert!(matches!(
        preview.warnings.last(),
        Some(ImportWarning::HostResolutionFailed { host, .. }) if host == "broken"
    ));
}
