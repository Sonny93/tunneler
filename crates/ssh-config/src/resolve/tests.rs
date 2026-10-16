use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use tempfile::TempDir;

use super::*;
use crate::directive::{
    AddKeysMode, DynamicForward, Endpoint, ForwardSpec, JumpHost, RemoteForward,
    StrictHostKeyChecking,
};
use crate::environment::fake::FakeEnvironment;
use crate::error::{Error, TokenError, ValueError, WarningKind};
use crate::resolved::{AgentSocket, ForwardAgent, IdentityAgent, ResolvedConfig};

struct Fixture {
    directory: TempDir,
    environment: FakeEnvironment,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let environment = FakeEnvironment {
            home_directory: directory.path().join("home"),
            system_directory: directory.path().join("etc"),
            ..FakeEnvironment::default()
        };
        Self {
            directory,
            environment,
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.directory.path().join(relative)
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn resolution(&self, config: &str, alias: &str) -> Result<Resolution, Error> {
        let user = self.write("user_config", config);
        resolve(&ConfigSources::user_only(user), alias, &self.environment)
    }

    fn resolved(&self, config: &str, alias: &str) -> ResolvedConfig {
        self.resolution(config, alias).unwrap().config
    }

    fn resolution_with_both(&self, user: &str, system: &str, alias: &str) -> Resolution {
        let sources = ConfigSources {
            user: Some(self.write("user_config", user)),
            system: Some(self.write("etc/ssh_config", system)),
        };
        resolve(&sources, alias, &self.environment).unwrap()
    }
}

fn resolved(config: &str, alias: &str) -> ResolvedConfig {
    Fixture::new().resolved(config, alias)
}

fn tcp(address: Option<&str>, port: u16) -> Endpoint {
    Endpoint::Tcp {
        address: address.map(str::to_owned),
        port,
    }
}

fn paths(values: &[&str]) -> Vec<PathBuf> {
    values.iter().map(PathBuf::from).collect()
}

fn fixture_home(fixture: &Fixture, relative: &str) -> PathBuf {
    fixture.environment.home_directory.join(relative)
}

mod defaults {
    use super::*;

    #[test]
    fn should_default_port_to_22_when_nothing_sets_it() {
        assert_eq!(resolved("", "web").port, 22);
    }

    #[test]
    fn should_default_user_to_local_user_when_nothing_sets_it() {
        assert_eq!(resolved("", "web").user, "alice");
    }

    #[test]
    fn should_default_hostname_to_alias_when_nothing_sets_it() {
        assert_eq!(resolved("", "web").hostname, "web");
    }

    #[test]
    fn should_lowercase_hostname_when_alias_has_capitals() {
        assert_eq!(resolved("", "Web.Example.COM").hostname, "web.example.com");
    }

    #[test]
    fn should_keep_alias_case_when_reporting_the_original_host() {
        assert_eq!(resolved("", "Web").alias, "Web");
    }

    #[test]
    fn should_keep_ip_address_case_when_hostname_is_an_ipv6_literal() {
        let config = resolved("Host x\n  HostName 2001:DB8::ABCD\n", "x");
        assert_eq!(config.hostname, "2001:DB8::ABCD");
    }

    #[test]
    fn should_default_server_alive_count_max_to_3() {
        assert_eq!(resolved("", "web").server_alive_count_max, 3);
    }

    #[test]
    fn should_default_strict_host_key_checking_to_ask() {
        assert_eq!(
            resolved("", "web").strict_host_key_checking,
            StrictHostKeyChecking::Ask
        );
    }

    #[test]
    fn should_default_known_hosts_files_under_home_when_nothing_sets_them() {
        let fixture = Fixture::new();
        let config = fixture.resolved("", "web");
        assert_eq!(
            config.user_known_hosts_files,
            vec![
                fixture_home(&fixture, ".ssh/known_hosts"),
                fixture_home(&fixture, ".ssh/known_hosts2")
            ]
        );
    }

    #[test]
    fn should_default_global_known_hosts_files_to_the_system_directory() {
        let fixture = Fixture::new();
        let config = fixture.resolved("", "web");
        assert_eq!(
            config.global_known_hosts_files,
            vec![
                fixture.path("etc/ssh_known_hosts"),
                fixture.path("etc/ssh_known_hosts2")
            ]
        );
    }

    #[test]
    fn should_default_identity_files_when_none_is_configured() {
        let fixture = Fixture::new();
        let config = fixture.resolved("", "web");
        let names: Vec<String> = config
            .identity_files
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "id_rsa",
                "id_ecdsa",
                "id_ecdsa_sk",
                "id_ed25519",
                "id_ed25519_sk",
                "id_xmss"
            ]
        );
    }

    #[test]
    fn should_not_default_identity_files_when_one_is_configured() {
        let config = resolved("IdentityFile /keys/one\n", "web");
        assert_eq!(config.identity_files, paths(&["/keys/one"]));
    }

    #[test]
    fn should_not_default_identity_files_when_identity_file_is_none() {
        assert_eq!(
            resolved("IdentityFile none\n", "web").identity_files,
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn should_default_booleans_like_openssh() {
        let config = resolved("", "web");
        assert_eq!(
            (
                config.password_authentication,
                config.kbd_interactive_authentication,
                config.tcp_keep_alive,
                config.compression,
                config.batch_mode
            ),
            (true, true, true, false, false)
        );
    }

    #[test]
    fn should_default_other_to_empty() {
        assert!(resolved("", "web").other.is_empty());
    }

    #[test]
    fn should_have_no_connect_timeout_when_unset() {
        assert_eq!(resolved("", "web").connect_timeout, None);
    }

    #[test]
    fn should_default_add_keys_to_agent_to_no() {
        assert_eq!(resolved("", "web").add_keys_to_agent.mode, AddKeysMode::No);
    }
}

mod evaluation {
    use super::*;

    #[test]
    fn should_keep_first_value_when_directive_is_repeated() {
        let config = resolved("Host *\n  User first\n  User second\n", "web");
        assert_eq!(config.user, "first");
    }

    #[test]
    fn should_keep_value_of_earlier_block_when_later_block_also_matches() {
        let config = resolved("Host web\n  Port 1\nHost *\n  Port 2\n", "web");
        assert_eq!(config.port, 1);
    }

    #[test]
    fn should_ignore_block_when_host_pattern_does_not_match() {
        let config = resolved("Host other\n  Port 1\n", "web");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_apply_directives_before_any_block_to_every_host() {
        let config = resolved("Port 99\nHost other\n  Port 1\n", "web");
        assert_eq!(config.port, 99);
    }

    #[test]
    fn should_skip_block_when_negated_pattern_matches() {
        let config = resolved("Host * !web\n  Port 1\n", "web");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_match_host_pattern_case_sensitively() {
        let config = resolved("Host Web\n  Port 1\n", "web");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_accumulate_identity_files_in_order_when_several_blocks_match() {
        let config = resolved(
            "Host web\n  IdentityFile /k/a\nHost *\n  IdentityFile /k/b\n",
            "web",
        );
        assert_eq!(config.identity_files, paths(&["/k/a", "/k/b"]));
    }

    #[test]
    fn should_drop_duplicate_identity_file_when_the_same_path_is_added_twice() {
        let config = resolved(
            "IdentityFile /k/a\nIdentityFile /k/b\nIdentityFile /k/a\n",
            "web",
        );
        assert_eq!(config.identity_files, paths(&["/k/a", "/k/b"]));
    }

    #[test]
    fn should_accumulate_certificate_files_when_several_are_given() {
        let config = resolved("CertificateFile /c/a\nCertificateFile /c/b\n", "web");
        assert_eq!(config.certificate_files, paths(&["/c/a", "/c/b"]));
    }

    #[test]
    fn should_accumulate_local_forwards_and_drop_duplicates() {
        let config = resolved(
            "LocalForward 80 h:80\nLocalForward 81 h:81\nLocalForward 80 h:80\n",
            "web",
        );
        assert_eq!(
            config.local_forwards,
            vec![
                ForwardSpec {
                    listen: tcp(None, 80),
                    target: tcp(Some("h"), 80)
                },
                ForwardSpec {
                    listen: tcp(None, 81),
                    target: tcp(Some("h"), 81)
                }
            ]
        );
    }

    #[test]
    fn should_accumulate_remote_and_dynamic_forwards() {
        let config = resolved(
            "RemoteForward 90 h:90\nRemoteForward 91\nDynamicForward 1080\n",
            "web",
        );
        assert_eq!(
            (config.remote_forwards.len(), config.dynamic_forwards),
            (
                2,
                vec![DynamicForward {
                    listen: tcp(None, 1080)
                }]
            )
        );
    }

    #[test]
    fn should_parse_socks_remote_forward_when_it_has_no_target() {
        let config = resolved("RemoteForward 91\n", "web");
        assert_eq!(
            config.remote_forwards,
            vec![RemoteForward::Dynamic(DynamicForward {
                listen: tcp(None, 91)
            })]
        );
    }

    #[test]
    fn should_drop_all_forwards_when_clear_all_forwardings_is_set() {
        let config = resolved(
            "LocalForward 80 h:80\nRemoteForward 90 h:90\nDynamicForward 1\nClearAllForwardings yes\n",
            "web",
        );
        assert_eq!(
            (
                config.local_forwards.len(),
                config.remote_forwards.len(),
                config.dynamic_forwards.len()
            ),
            (0, 0, 0)
        );
    }

    #[test]
    fn should_append_send_env_without_deduplicating() {
        let config = resolved("SendEnv A B\nSendEnv A\n", "web");
        assert_eq!(config.send_env, vec!["A", "B", "A"]);
    }

    #[test]
    fn should_remove_send_env_entries_when_pattern_is_prefixed_with_a_dash() {
        let config = resolved("SendEnv LANG LC_ALL LC_CTYPE\nSendEnv -LC_*\n", "web");
        assert_eq!(config.send_env, vec!["LANG"]);
    }

    #[test]
    fn should_keep_first_set_env_when_directive_is_repeated() {
        let config = resolved("SetEnv A=1\nSetEnv B=2\n", "web");
        assert_eq!(
            config
                .set_env
                .iter()
                .map(|variable| variable.name.as_str())
                .collect::<Vec<_>>(),
            vec!["A"]
        );
    }

    #[test]
    fn should_apply_user_file_before_system_file() {
        let fixture = Fixture::new();
        let resolution =
            fixture.resolution_with_both("User fromuser\n", "User fromsystem\nPort 2\n", "web");
        assert_eq!(
            (resolution.config.user, resolution.config.port),
            ("fromuser".to_owned(), 2)
        );
    }

    #[test]
    fn should_treat_missing_files_as_empty() {
        let sources = ConfigSources {
            user: Some(PathBuf::from("/nonexistent/config")),
            system: Some(PathBuf::from("/nonexistent/ssh_config")),
        };
        let resolution = resolve(&sources, "web", &FakeEnvironment::default()).unwrap();
        assert_eq!(resolution.config.port, 22);
    }

    #[test]
    fn should_leave_connect_timeout_open_to_later_lines_when_value_is_none() {
        let config = resolved("ConnectTimeout none\nConnectTimeout 5\n", "web");
        assert_eq!(config.connect_timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn should_parse_time_values_when_they_have_units() {
        let config = resolved("ServerAliveInterval 1m30s\n", "web");
        assert_eq!(config.server_alive_interval, Duration::from_secs(90));
    }

    #[test]
    fn should_report_known_hosts_files_as_empty_when_none() {
        let config = resolved(
            "UserKnownHostsFile none\nGlobalKnownHostsFile none\n",
            "web",
        );
        assert_eq!(
            (
                config.user_known_hosts_files,
                config.global_known_hosts_files
            ),
            (Vec::<PathBuf>::new(), Vec::<PathBuf>::new())
        );
    }

    #[test]
    fn should_keep_first_known_hosts_files_when_directive_is_repeated() {
        let config = resolved("UserKnownHostsFile /a /b\nUserKnownHostsFile /c\n", "web");
        assert_eq!(config.user_known_hosts_files, paths(&["/a", "/b"]));
    }

    #[test]
    fn should_split_preferred_authentications_on_commas() {
        let config = resolved("PreferredAuthentications publickey,password\n", "web");
        assert_eq!(
            config.preferred_authentications,
            Some(vec!["publickey".to_owned(), "password".to_owned()])
        );
    }

    #[test]
    fn should_lowercase_host_key_alias() {
        let config = resolved("HostKeyAlias MixedCase\n", "web");
        assert_eq!(config.host_key_alias.as_deref(), Some("mixedcase"));
    }

    #[test]
    fn should_keep_unmodeled_known_directive_in_other_with_first_value() {
        let config = resolved("LogLevel DEBUG\nLogLevel ERROR\n", "web");
        assert_eq!(
            config.other.get("loglevel"),
            Some(&vec!["DEBUG".to_owned()])
        );
    }

    #[test]
    fn should_not_keep_unmodeled_directive_when_its_block_is_inactive() {
        let config = resolved("Host other\n  LogLevel DEBUG\n", "web");
        assert!(!config.other.contains_key("loglevel"));
    }

    #[test]
    fn should_parse_add_keys_to_agent_lifetime() {
        let config = resolved("AddKeysToAgent confirm 5m\n", "web");
        assert_eq!(
            (
                config.add_keys_to_agent.mode,
                config.add_keys_to_agent.lifetime
            ),
            (AddKeysMode::Confirm, Some(Duration::from_secs(300)))
        );
    }

    #[test]
    fn should_keep_tag_and_ignore_unknown_values() {
        let config = resolved("IgnoreUnknown Foo\nTag blue\n", "web");
        assert_eq!(
            (config.tag.as_deref(), config.ignore_unknown.as_deref()),
            (Some("blue"), Some("Foo"))
        );
    }
}

mod matching {
    use super::*;

    #[test]
    fn should_match_host_against_hostname_set_above() {
        let config = resolved(
            "Host alias\n  HostName real.example.com\nMatch host real.example.com\n  Port 2022\n",
            "alias",
        );
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_not_match_host_against_hostname_set_below() {
        let config = resolved(
            "Match host real.example.com\n  Port 2022\nHost alias\n  HostName real.example.com\n",
            "alias",
        );
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_expand_percent_h_when_matching_host_against_hostname() {
        let config = resolved(
            "Host alias\n  HostName %h.example.com\nMatch host alias.example.com\n  Port 2022\n",
            "alias",
        );
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_match_host_case_insensitively_against_a_pattern_list() {
        let config = resolved(
            "Match host WEB.example.com,other\n  Port 2022\n",
            "web.example.com",
        );
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_match_original_host_against_the_alias_even_when_hostname_differs() {
        let config = resolved(
            "Host alias\n  HostName real\nMatch originalhost alias\n  Port 2022\n",
            "alias",
        );
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_not_match_original_host_against_the_hostname() {
        let config = resolved(
            "Host alias\n  HostName real\nMatch originalhost real\n  Port 2022\n",
            "alias",
        );
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_match_user_against_local_user_when_user_is_not_set() {
        let config = resolved("Match user alice\n  Port 2022\n", "web");
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_match_user_against_configured_user() {
        let config = resolved("User bob\nMatch user bob\n  Port 2022\n", "web");
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_match_localuser_against_the_local_account() {
        let config = resolved("User bob\nMatch localuser alice\n  Port 2022\n", "web");
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_negate_criterion_when_it_has_a_bang() {
        let config = resolved("Match !localuser alice\n  Port 2022\n", "web");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_require_every_criterion_when_several_are_given() {
        let config = resolved("Match localuser alice host other\n  Port 2022\n", "web");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_match_everything_when_criterion_is_all() {
        let config = resolved("Host other\n  Port 1\nMatch all\n  Port 2022\n", "web");
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_match_tagged_against_the_tag_set_above() {
        let config = resolved("Tag blue\nMatch tagged blue\n  Port 2022\n", "web");
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_match_empty_tagged_when_no_tag_is_set() {
        let config = resolved("Match tagged \"\"\n  Port 2022\n", "web");
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_run_exec_command_with_tokens_expanded() {
        let mut fixture = Fixture::new();
        fixture.environment.succeeding_commands = vec!["test web = web".to_owned()];
        let config = fixture.resolved("Match exec \"test %h = web\"\n  Port 2022\n", "web");
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_not_match_when_exec_command_fails() {
        let config = resolved("Match exec \"false\"\n  Port 2022\n", "web");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_not_run_exec_command_when_an_earlier_criterion_failed() {
        let fixture = Fixture::new();
        fixture.resolved("Match host other exec \"touch x\"\n  Port 2022\n", "web");
        assert!(fixture.environment.executed_commands.borrow().is_empty());
    }

    #[test]
    fn should_not_match_unsupported_criterion_even_when_negated() {
        let config = resolved("Match !version 1.*\n  Port 2022\n", "web");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_warn_when_criterion_is_unsupported() {
        let warnings = Fixture::new()
            .resolution("Match version 1.*\n  Port 2022\n", "web")
            .unwrap()
            .warnings;
        assert!(matches!(
            warnings[0].kind,
            WarningKind::UnsupportedCriterion { ref criterion } if criterion == "version"
        ));
    }

    #[test]
    fn should_reset_block_state_when_match_follows_a_host_block() {
        let config = resolved(
            "Host web\n  Port 1\nMatch host other\n  User nobody\n",
            "web",
        );
        assert_eq!(config.user, "alice");
    }
}

mod final_pass {
    use super::*;

    #[test]
    fn should_match_final_only_in_the_second_pass() {
        let config = resolved("Match final\n  Port 2022\n", "web");
        assert_eq!(config.port, 2022);
    }

    #[test]
    fn should_not_match_canonical_in_the_first_pass() {
        let config = resolved("Match canonical\n  Port 1\nMatch final\n  User x\n", "web");
        assert_eq!(config.port, 1);
    }

    #[test]
    fn should_match_negated_final_in_the_first_pass() {
        let config = resolved("Match !final\n  Port 1\nMatch final\n  Port 2\n", "web");
        assert_eq!(config.port, 1);
    }

    #[test]
    fn should_match_host_lines_against_the_resolved_host_in_the_final_pass() {
        let config = resolved(
            "Host alias\n  HostName real.example.com\nHost real.example.com\n  User fromreal\nMatch final all\n  Tag x\n",
            "alias",
        );
        assert_eq!(config.user, "fromreal");
    }

    #[test]
    fn should_not_rematch_alias_host_lines_in_the_final_pass() {
        let config = resolved(
            "Host alias\n  HostName real.example.com\n  SendEnv A\nMatch final all\n  Tag x\n",
            "alias",
        );
        assert_eq!(config.send_env, vec!["A"]);
    }

    #[test]
    fn should_keep_first_values_in_the_final_pass() {
        let config = resolved("Port 1\nMatch final\n  Port 2\n", "web");
        assert_eq!(config.port, 1);
    }

    #[test]
    fn should_not_apply_final_pass_when_no_match_requests_it() {
        let config = resolved(
            "Host alias\n  HostName real.example.com\nHost real.example.com\n  User fromreal\n",
            "alias",
        );
        assert_eq!(config.user, "alice");
    }

    #[test]
    fn should_request_final_pass_when_match_final_is_in_an_inactive_include() {
        let fixture = Fixture::new();
        fixture.write("final_inc.conf", "Match final\n  Port 7\n");
        let config = fixture.resolved(
            &format!(
                "Host alias\n  HostName real.example.com\nHost never\n  Include {}\nHost real.example.com\n  Tag late\n",
                fixture.path("final_inc.conf").display()
            ),
            "alias",
        );
        assert_eq!(config.tag.as_deref(), Some("late"));
    }

    #[test]
    fn should_expand_include_with_final_host_in_the_final_pass() {
        let fixture = Fixture::new();
        fixture.write("by_host/web.real.org.conf", "Port 2200\n");
        let config = fixture.resolved(
            &format!(
                "Match final\n  Include {}/by_host/%h.conf\nHost web\n  HostName WEB.real.ORG\n",
                fixture.directory.path().display()
            ),
            "web",
        );
        assert_eq!(config.port, 2200);
    }
}

mod warnings {
    use super::*;

    fn warning_kinds(config: &str) -> Vec<WarningKind> {
        Fixture::new()
            .resolution(config, "web")
            .unwrap()
            .warnings
            .into_iter()
            .map(|warning| warning.kind)
            .collect()
    }

    #[test]
    fn should_warn_when_keyword_is_unknown() {
        assert_eq!(
            warning_kinds("Frobnicate yes\n"),
            vec![WarningKind::UnknownKeyword {
                keyword: "frobnicate".to_owned()
            }]
        );
    }

    #[test]
    fn should_carry_path_and_line_when_warning_about_an_unknown_keyword() {
        let warnings = Fixture::new()
            .resolution("Port 22\nFrobnicate yes\n", "web")
            .unwrap()
            .warnings;
        assert_eq!(warnings[0].line, 2);
        assert!(warnings[0].path.ends_with("user_config"));
    }

    #[test]
    fn should_not_warn_when_unknown_keyword_is_ignored() {
        assert_eq!(
            warning_kinds("IgnoreUnknown Frob*\nFrobnicate yes\n"),
            Vec::<WarningKind>::new()
        );
    }

    #[test]
    fn should_warn_when_unknown_keyword_precedes_ignore_unknown() {
        assert_eq!(
            warning_kinds("Frobnicate yes\nIgnoreUnknown Frob*\n").len(),
            1
        );
    }

    #[test]
    fn should_warn_when_unknown_keyword_does_not_match_ignore_unknown() {
        assert_eq!(
            warning_kinds("IgnoreUnknown Other\nFrobnicate yes\n").len(),
            1
        );
    }

    #[test]
    fn should_not_warn_when_keyword_is_known_but_unmodeled() {
        assert_eq!(
            warning_kinds("LogLevel DEBUG\nControlMaster auto\n"),
            Vec::new()
        );
    }

    #[test]
    fn should_warn_once_when_the_final_pass_walks_the_same_line_again() {
        assert_eq!(
            warning_kinds("Frobnicate yes\nMatch final\n  Port 1\n").len(),
            1
        );
    }

    #[test]
    fn should_warn_when_canonicalize_hostname_is_enabled() {
        assert_eq!(
            warning_kinds("CanonicalizeHostname yes\n"),
            vec![WarningKind::UnsupportedCanonicalization {
                value: "yes".to_owned()
            }]
        );
    }

    #[test]
    fn should_not_warn_when_canonicalize_hostname_is_no() {
        assert_eq!(warning_kinds("CanonicalizeHostname no\n"), Vec::new());
    }

    #[test]
    fn should_not_request_final_pass_when_canonicalize_hostname_is_enabled() {
        let config = resolved(
            "CanonicalizeHostname always\nHost alias\n  HostName real\nHost real\n  User x\n",
            "alias",
        );
        assert_eq!(config.user, "alice");
    }

    #[test]
    fn should_not_warn_when_canonicalize_hostname_is_in_an_inactive_block() {
        assert_eq!(
            warning_kinds("Host other\n  CanonicalizeHostname yes\n"),
            Vec::new()
        );
    }
}

mod proxies {
    use super::*;

    fn jump(host: &str) -> JumpHost {
        JumpHost {
            user: None,
            host: host.to_owned(),
            port: None,
        }
    }

    #[test]
    fn should_resolve_proxy_jump_chain_in_order() {
        let config = resolved("ProxyJump a,u@b:2222\n", "web");
        assert_eq!(
            config.proxy_jump,
            vec![
                jump("a"),
                JumpHost {
                    user: Some("u".to_owned()),
                    host: "b".to_owned(),
                    port: Some(2222)
                }
            ]
        );
    }

    #[test]
    fn should_have_no_proxy_command_when_unset() {
        assert_eq!(resolved("", "web").proxy_command, None);
    }

    #[test]
    fn should_ignore_proxy_command_when_proxy_jump_came_first() {
        let config = resolved("ProxyJump a\nProxyCommand nc %h %p\n", "web");
        assert_eq!(config.proxy_command, None);
    }

    #[test]
    fn should_ignore_proxy_jump_when_proxy_command_came_first() {
        let config = resolved("ProxyCommand nc %h %p\nProxyJump a\n", "web");
        assert_eq!(config.proxy_jump, Vec::<JumpHost>::new());
    }

    #[test]
    fn should_report_no_proxy_when_proxy_command_is_none() {
        let config = resolved("ProxyCommand none\nProxyJump a\n", "web");
        assert_eq!(
            (config.proxy_command, config.proxy_jump),
            (None, Vec::<JumpHost>::new())
        );
    }

    #[test]
    fn should_allow_proxy_command_after_proxy_jump_none() {
        let config = resolved("ProxyJump none\nProxyCommand nc %h %p\n", "web");
        assert_eq!(config.proxy_command.as_deref(), Some("nc web 22"));
    }

    #[test]
    fn should_ignore_later_proxy_jump_after_proxy_jump_none() {
        let config = resolved("ProxyJump none\nProxyJump a\n", "web");
        assert_eq!(config.proxy_jump, Vec::<JumpHost>::new());
    }

    #[test]
    fn should_expand_tokens_when_resolving_a_proxy_command() {
        let config = resolved(
            "User bob\nPort 2222\nProxyCommand ssh -W %h:%p -l %r %n %%\n",
            "web",
        );
        assert_eq!(
            config.proxy_command.as_deref(),
            Some("ssh -W web:2222 -l bob web %")
        );
    }

    #[test]
    fn should_keep_trailing_comment_in_proxy_command() {
        let config = resolved("ProxyCommand nc %h %p # why\n", "web");
        assert_eq!(config.proxy_command.as_deref(), Some("nc web 22 # why"));
    }
}

mod expansion {
    use super::*;
    use crate::tokens::connection_hash;

    #[test]
    fn should_expand_hostname_token_with_the_alias() {
        let config = resolved("Host web\n  HostName %h.example.com\n", "web");
        assert_eq!(config.hostname, "web.example.com");
    }

    #[test]
    fn should_fail_when_hostname_uses_a_disallowed_token() {
        let result = Fixture::new().resolution("HostName %r.example.com\n", "web");
        assert!(matches!(
            result,
            Err(Error::Token {
                source: TokenError::UnknownToken { token: 'r' },
                ..
            })
        ));
    }

    #[test]
    fn should_expand_user_tokens_when_user_has_a_local_user_token() {
        let config = resolved("User %u-remote\n", "web");
        assert_eq!(config.user, "alice-remote");
    }

    #[test]
    fn should_fail_when_user_refers_to_itself() {
        let result = Fixture::new().resolution("User %r\n", "web");
        assert!(matches!(result, Err(Error::Token { .. })));
    }

    #[test]
    fn should_expand_identity_file_tokens_and_tilde() {
        let fixture = Fixture::new();
        let config = fixture.resolved("User bob\nIdentityFile ~/.ssh/k_%r_%h\n", "web");
        assert_eq!(
            config.identity_files,
            vec![fixture_home(&fixture, ".ssh/k_bob_web")]
        );
    }

    #[test]
    fn should_expand_environment_variables_in_identity_files() {
        let mut fixture = Fixture::new();
        fixture
            .environment
            .variables
            .insert("KEYS".to_owned(), "/keys".to_owned());
        let config = fixture.resolved("IdentityFile ${KEYS}/id\n", "web");
        assert_eq!(config.identity_files, paths(&["/keys/id"]));
    }

    #[test]
    fn should_expand_home_token_in_certificate_files() {
        let config = resolved("CertificateFile %d/cert\n", "web");
        assert_eq!(config.certificate_files.len(), 1);
        assert!(config.certificate_files[0].ends_with("home/cert"));
    }

    #[test]
    fn should_report_origin_when_identity_file_has_an_unknown_token() {
        let error = Fixture::new()
            .resolution("Port 22\nIdentityFile ~/%z\n", "web")
            .unwrap_err();
        let Error::Token { origin, source } = error else {
            panic!("expected a token error");
        };
        assert_eq!(origin.line, 2);
        assert_eq!(source, TokenError::UnknownToken { token: 'z' });
    }

    #[test]
    fn should_fail_when_environment_variable_of_an_identity_file_is_undefined() {
        let result = Fixture::new().resolution("IdentityFile ${NOPE}\n", "web");
        assert!(matches!(
            result,
            Err(Error::Token {
                source: TokenError::UndefinedEnvironmentVariable { .. },
                ..
            })
        ));
    }

    #[test]
    fn should_expand_connection_hash_in_known_hosts_files() {
        let config = resolved("User bob\nPort 2222\nUserKnownHostsFile /kh/%C\n", "web");
        let expected = connection_hash("laptop.example.org", "web", "2222", "bob", "");
        assert_eq!(
            config.user_known_hosts_files,
            vec![PathBuf::from(format!("/kh/{expected}"))]
        );
    }

    #[test]
    fn should_include_the_jump_host_in_the_connection_hash() {
        let config = resolved("ProxyJump jumper\nUserKnownHostsFile /kh/%C\n", "web");
        let expected = connection_hash("laptop.example.org", "web", "22", "alice", "jumper");
        assert_eq!(
            config.user_known_hosts_files,
            vec![PathBuf::from(format!("/kh/{expected}"))]
        );
    }

    #[test]
    fn should_expand_all_simple_tokens_in_known_hosts_files() {
        let config = resolved(
            "UserKnownHostsFile /kh/%l_%L_%i_%u_%n_%k_%j_%%\nHostKeyAlias KeyAlias\n",
            "Web",
        );
        assert_eq!(
            config.user_known_hosts_files,
            paths(&["/kh/laptop.example.org_laptop_1000_alice_Web_keyalias__%"])
        );
    }

    #[test]
    fn should_use_the_original_host_for_percent_k_when_there_is_no_host_key_alias() {
        let config = resolved("UserKnownHostsFile /kh/%k\n", "Web");
        assert_eq!(config.user_known_hosts_files, paths(&["/kh/Web"]));
    }

    #[test]
    fn should_only_expand_tilde_in_global_known_hosts_files() {
        let fixture = Fixture::new();
        let config = fixture.resolved("GlobalKnownHostsFile ~/g_%h\n", "web");
        assert_eq!(
            config.global_known_hosts_files,
            vec![fixture_home(&fixture, "g_%h")]
        );
    }

    #[test]
    fn should_expand_tokens_in_unix_socket_forward_paths() {
        let config = resolved("User bob\nLocalForward /s/%r.sock h:80\n", "web");
        assert_eq!(
            config.local_forwards[0].listen,
            Endpoint::Unix(PathBuf::from("/s/bob.sock"))
        );
    }

    #[test]
    fn should_fail_when_a_unix_socket_forward_path_has_an_unknown_token() {
        let result = Fixture::new().resolution("LocalForward /s/%z.sock h:80\n", "web");
        assert!(matches!(result, Err(Error::Token { .. })));
    }

    #[test]
    fn should_expand_set_env_values_but_not_names() {
        let config = resolved("User bob\nSetEnv WHO=%r\n", "web");
        assert_eq!(config.set_env[0].value, "bob");
    }

    #[test]
    fn should_classify_identity_agent_none_as_disabled() {
        let config = resolved("IdentityAgent none\n", "web");
        assert_eq!(config.identity_agent, Some(IdentityAgent::Disabled));
    }

    #[test]
    fn should_classify_ssh_auth_sock_as_environment_socket() {
        let config = resolved("IdentityAgent SSH_AUTH_SOCK\n", "web");
        assert_eq!(
            config.identity_agent,
            Some(IdentityAgent::Socket(AgentSocket::Environment(
                "SSH_AUTH_SOCK".to_owned()
            )))
        );
    }

    #[test]
    fn should_classify_dollar_name_as_environment_socket() {
        let config = resolved("IdentityAgent $MY_AGENT\n", "web");
        assert_eq!(
            config.identity_agent,
            Some(IdentityAgent::Socket(AgentSocket::Environment(
                "MY_AGENT".to_owned()
            )))
        );
    }

    #[test]
    fn should_classify_other_identity_agent_values_as_paths_with_tokens_expanded() {
        let config = resolved("User bob\nIdentityAgent /run/agent_%r.sock\n", "web");
        assert_eq!(
            config.identity_agent,
            Some(IdentityAgent::Socket(AgentSocket::Path(PathBuf::from(
                "/run/agent_bob.sock"
            ))))
        );
    }

    #[test]
    fn should_have_no_identity_agent_when_unset() {
        assert_eq!(resolved("", "web").identity_agent, None);
    }

    #[test]
    fn should_resolve_forward_agent_flags_and_sockets() {
        let enabled = resolved("ForwardAgent yes\n", "web").forward_agent;
        let disabled = resolved("ForwardAgent no\n", "web").forward_agent;
        let socket = resolved("ForwardAgent /run/agent.sock\n", "web").forward_agent;
        assert_eq!(
            (enabled, disabled, socket),
            (
                ForwardAgent::Enabled,
                ForwardAgent::Disabled,
                ForwardAgent::Socket(AgentSocket::Path(PathBuf::from("/run/agent.sock")))
            )
        );
    }

    #[test]
    fn should_report_origin_when_hostname_expansion_fails() {
        let error = Fixture::new()
            .resolution("Port 1\nHostName %n\n", "web")
            .unwrap_err();
        let Error::Token { origin, .. } = error else {
            panic!("expected a token error");
        };
        assert_eq!(origin.line, 2);
    }
}

mod errors {
    use super::*;

    #[test]
    fn should_report_path_and_line_when_a_value_is_invalid() {
        let error = Fixture::new()
            .resolution("Host web\n  Port 70000\n", "web")
            .unwrap_err();
        assert!(
            error
                .to_string()
                .ends_with("user_config line 2: invalid value for port: bad port \"70000\"")
        );
    }

    #[test]
    fn should_report_value_error_kind_when_a_value_is_invalid() {
        let error = Fixture::new()
            .resolution("ConnectTimeout soon\n", "web")
            .unwrap_err();
        assert!(matches!(
            error,
            Error::Value {
                kind: ValueError::InvalidTime { .. },
                ..
            }
        ));
    }

    #[test]
    fn should_validate_values_of_blocks_that_do_not_apply() {
        let result = Fixture::new().resolution("Host other\n  Port nope\n", "web");
        assert!(matches!(result, Err(Error::Value { .. })));
    }

    #[test]
    fn should_report_syntax_errors_with_their_line() {
        let error = Fixture::new()
            .resolution("Port 22\nHost \"unclosed\n", "web")
            .unwrap_err();
        assert!(matches!(error, Error::Syntax { origin, .. } if origin.line == 2));
    }

    #[test]
    fn should_report_match_errors_with_their_line() {
        let error = Fixture::new()
            .resolution("\n\nMatch host\n", "web")
            .unwrap_err();
        assert!(matches!(error, Error::Criteria { origin, .. } if origin.line == 3));
    }

    #[test]
    fn should_fail_when_a_match_exec_command_cannot_run() {
        struct FailingEnvironment(FakeEnvironment);
        impl LocalEnvironment for FailingEnvironment {
            fn variable(&self, name: &str) -> Option<String> {
                self.0.variable(name)
            }
            fn user_name(&self) -> String {
                self.0.user_name()
            }
            fn home_directory(&self) -> PathBuf {
                self.0.home_directory()
            }
            fn host_name(&self) -> String {
                self.0.host_name()
            }
            fn user_id(&self) -> u32 {
                self.0.user_id()
            }
            fn system_config_directory(&self) -> PathBuf {
                self.0.system_config_directory()
            }
            fn command_succeeds(
                &self,
                _command: &str,
            ) -> Result<bool, crate::environment::EnvironmentError> {
                Err(crate::environment::EnvironmentError::CommandTerminatedAbnormally)
            }
        }
        let fixture = Fixture::new();
        let user = fixture.write("user_config", "Match exec \"x\"\n  Port 1\n");
        let result = resolve(
            &ConfigSources::user_only(user),
            "web",
            &FailingEnvironment(FakeEnvironment::default()),
        );
        assert!(matches!(result, Err(Error::MatchExec { .. })));
    }
}

#[cfg(unix)]
mod includes {
    use super::*;

    #[test]
    fn should_include_relative_path_from_the_user_ssh_directory() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/extra.conf", "Port 2200\n");
        let config = fixture.resolved("Include extra.conf\n", "web");
        assert_eq!(config.port, 2200);
    }

    #[test]
    fn should_include_relative_path_from_the_system_directory_for_the_system_file() {
        let fixture = Fixture::new();
        fixture.write("etc/extra.conf", "Port 2200\n");
        let resolution = fixture.resolution_with_both("", "Include extra.conf\n", "web");
        assert_eq!(resolution.config.port, 2200);
    }

    #[test]
    fn should_include_absolute_path_as_is() {
        let fixture = Fixture::new();
        let included = fixture.write("elsewhere/x.conf", "Port 2200\n");
        let config = fixture.resolved(&format!("Include {}\n", included.display()), "web");
        assert_eq!(config.port, 2200);
    }

    #[test]
    fn should_include_files_matching_a_glob_in_sorted_order() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/conf.d/b.conf", "User fromb\n");
        fixture.write("home/.ssh/conf.d/a.conf", "User froma\n");
        let config = fixture.resolved("Include conf.d/*.conf\n", "web");
        assert_eq!(config.user, "froma");
    }

    #[test]
    fn should_ignore_include_when_nothing_matches() {
        let fixture = Fixture::new();
        let config = fixture.resolved("Include missing/*.conf\nPort 5\n", "web");
        assert_eq!(config.port, 5);
    }

    #[test]
    fn should_accept_several_patterns_on_one_include_line() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/a.conf", "User froma\n");
        fixture.write("home/.ssh/b.conf", "Port 2\n");
        let config = fixture.resolved("Include a.conf b.conf\n", "web");
        assert_eq!((config.user, config.port), ("froma".to_owned(), 2));
    }

    #[test]
    fn should_apply_included_file_inside_an_active_block() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/inc.conf", "Port 2200\n");
        let config = fixture.resolved("Host web\n  Include inc.conf\n", "web");
        assert_eq!(config.port, 2200);
    }

    #[test]
    fn should_never_apply_included_file_inside_an_inactive_block() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/inc.conf", "Port 2200\n");
        let config = fixture.resolved("Host other\n  Include inc.conf\n", "web");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_never_apply_host_lines_of_a_file_included_from_an_inactive_block() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/inc.conf", "Host web\n  Port 2200\n");
        let config = fixture.resolved("Host other\n  Include inc.conf\n", "web");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn should_apply_host_lines_of_a_file_included_from_an_active_block() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/inc.conf", "Host web\n  Port 2200\n");
        let config = fixture.resolved("Host web\n  Include inc.conf\n", "web");
        assert_eq!(config.port, 2200);
    }

    #[test]
    fn should_restore_the_including_block_state_after_the_include() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/inc.conf", "Match host nomatch\n  Port 9999\n");
        let config = fixture.resolved("Host web\n  Include inc.conf\n  Compression yes\n", "web");
        assert!(config.compression);
    }

    #[test]
    fn should_restore_an_inactive_state_after_the_include() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/inc.conf", "Match all\n  Port 9999\n");
        let config = fixture.resolved("Host other\n  Include inc.conf\n  Compression yes\n", "web");
        assert_eq!((config.port, config.compression), (22, false));
    }

    #[test]
    fn should_follow_nested_includes() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/a.conf", "Include b.conf\n");
        fixture.write("home/.ssh/b.conf", "Port 2200\n");
        let config = fixture.resolved("Include a.conf\n", "web");
        assert_eq!(config.port, 2200);
    }

    #[test]
    fn should_expand_tokens_in_include_paths() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/web.conf", "Port 2200\n");
        let config = fixture.resolved("Include %h.conf\n", "web");
        assert_eq!(config.port, 2200);
    }

    #[test]
    fn should_expand_environment_variables_in_include_paths() {
        let mut fixture = Fixture::new();
        fixture.write("home/.ssh/dev.conf", "Port 2200\n");
        fixture
            .environment
            .variables
            .insert("PROFILE".to_owned(), "dev".to_owned());
        let config = fixture.resolved("Include ${PROFILE}.conf\n", "web");
        assert_eq!(config.port, 2200);
    }

    #[test]
    fn should_skip_include_with_a_warning_when_its_environment_variable_is_undefined() {
        let fixture = Fixture::new();
        let resolution = fixture
            .resolution("Include ${UNDEFINED}.conf\nPort 5\n", "web")
            .unwrap();
        assert!(matches!(
            resolution.warnings[0].kind,
            WarningKind::IncludeSkipped { .. }
        ));
        assert_eq!(resolution.config.port, 5);
    }

    #[test]
    fn should_report_the_include_line_when_a_nested_file_has_an_error() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/bad.conf", "Port nope\n");
        let error = fixture.resolution("Include bad.conf\n", "web").unwrap_err();
        assert!(
            matches!(error, Error::Value { origin, .. } if origin.path.ends_with("bad.conf") && origin.line == 1)
        );
    }

    #[test]
    fn should_fail_with_a_cycle_error_when_a_file_includes_itself() {
        let fixture = Fixture::new();
        let looping = fixture.path("loop.conf");
        fixture.write("loop.conf", &format!("Include {}\n", looping.display()));
        let error = fixture
            .resolution(&format!("Include {}\n", looping.display()), "web")
            .unwrap_err();
        assert!(matches!(error, Error::IncludeCycle { chain, .. } if chain.len() == 3));
    }

    #[test]
    fn should_fail_with_a_cycle_error_when_two_files_include_each_other() {
        let fixture = Fixture::new();
        let first = fixture.path("a.conf");
        let second = fixture.path("b.conf");
        fixture.write("a.conf", &format!("Include {}\n", second.display()));
        fixture.write("b.conf", &format!("Include {}\n", first.display()));
        let error = fixture
            .resolution(&format!("Include {}\n", first.display()), "web")
            .unwrap_err();
        assert!(matches!(error, Error::IncludeCycle { .. }));
    }

    fn chain_of_includes(fixture: &Fixture, length: usize) -> String {
        for level in 1..=length {
            let content = if level < length {
                format!(
                    "Include {}\n",
                    fixture.path(&format!("d/{}.conf", level + 1)).display()
                )
            } else {
                "Tag deepest\n".to_owned()
            };
            fixture.write(&format!("d/{level}.conf"), &content);
        }
        format!("Include {}\n", fixture.path("d/1.conf").display())
    }

    #[test]
    fn should_follow_sixteen_nested_includes() {
        let fixture = Fixture::new();
        let main = chain_of_includes(&fixture, 16);
        let config = fixture.resolved(&main, "web");
        assert_eq!(config.tag.as_deref(), Some("deepest"));
    }

    #[test]
    fn should_fail_when_includes_are_nested_seventeen_levels_deep() {
        let fixture = Fixture::new();
        let main = chain_of_includes(&fixture, 17);
        let error = fixture.resolution(&main, "web").unwrap_err();
        assert!(
            matches!(error, Error::IncludeDepthExceeded { limit: 16, chain, .. } if chain.len() == 18)
        );
    }

    #[test]
    fn should_fail_when_the_system_file_includes_a_home_relative_path() {
        let fixture = Fixture::new();
        let sources = ConfigSources {
            user: None,
            system: Some(fixture.write("etc/ssh_config", "Include ~/x\n")),
        };
        let result = resolve(&sources, "web", &fixture.environment);
        assert!(matches!(result, Err(Error::InvalidIncludePath { .. })));
    }

    #[test]
    fn should_fail_when_an_include_argument_is_empty() {
        let result = Fixture::new().resolution("Include \"\"\n", "web");
        assert!(matches!(result, Err(Error::Value { .. })));
    }
}

#[cfg(unix)]
mod hosts {
    use super::*;

    fn listed(fixture: &Fixture, user_config: &str) -> Vec<String> {
        let sources = ConfigSources::user_only(fixture.write("user_config", user_config));
        list_hosts(&sources, &fixture.environment).unwrap()
    }

    #[test]
    fn should_list_concrete_aliases_in_file_order() {
        let fixture = Fixture::new();
        assert_eq!(
            listed(&fixture, "Host web db\nHost cache\n"),
            vec!["web", "db", "cache"]
        );
    }

    #[test]
    fn should_skip_wildcards_and_negations() {
        let fixture = Fixture::new();
        assert_eq!(
            listed(&fixture, "Host web* ?db !skip real *\n"),
            vec!["real"]
        );
    }

    #[test]
    fn should_deduplicate_aliases_keeping_the_first_position() {
        let fixture = Fixture::new();
        assert_eq!(
            listed(&fixture, "Host web db\nHost db web cache\n"),
            vec!["web", "db", "cache"]
        );
    }

    #[test]
    fn should_follow_includes_even_inside_a_block_that_would_not_apply() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/inc.conf", "Host included\n");
        assert_eq!(
            listed(
                &fixture,
                "Host first\nMatch host nothing\n  Include inc.conf\nHost last\n"
            ),
            vec!["first", "included", "last"]
        );
    }

    #[test]
    fn should_follow_nested_includes_and_globs() {
        let fixture = Fixture::new();
        fixture.write("home/.ssh/conf.d/a.conf", "Include b.conf\nHost a\n");
        fixture.write("home/.ssh/b.conf", "Host b\n");
        assert_eq!(listed(&fixture, "Include conf.d/*.conf\n"), vec!["b", "a"]);
    }

    #[test]
    fn should_list_user_aliases_before_system_aliases() {
        let fixture = Fixture::new();
        let sources = ConfigSources {
            user: Some(fixture.write("user_config", "Host fromuser\n")),
            system: Some(fixture.write("etc/ssh_config", "Host fromsystem fromuser\n")),
        };
        assert_eq!(
            list_hosts(&sources, &fixture.environment).unwrap(),
            vec!["fromuser", "fromsystem"]
        );
    }

    #[test]
    fn should_return_nothing_when_files_are_missing() {
        let sources = ConfigSources::user_only("/nonexistent/config");
        assert_eq!(
            list_hosts(&sources, &FakeEnvironment::default()).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn should_fail_when_a_file_has_a_syntax_error() {
        let fixture = Fixture::new();
        let sources = ConfigSources::user_only(fixture.write("user_config", "Host \"a\n"));
        assert!(matches!(
            list_hosts(&sources, &fixture.environment),
            Err(Error::Syntax { .. })
        ));
    }

    #[test]
    fn should_fail_when_includes_cycle() {
        let fixture = Fixture::new();
        let looping = fixture.path("loop.conf");
        fixture.write("loop.conf", &format!("Include {}\n", looping.display()));
        let sources = ConfigSources::user_only(
            fixture.write("user_config", &format!("Include {}\n", looping.display())),
        );
        assert!(matches!(
            list_hosts(&sources, &fixture.environment),
            Err(Error::IncludeCycle { .. })
        ));
    }

    #[test]
    fn should_skip_includes_whose_environment_variable_is_undefined() {
        let fixture = Fixture::new();
        assert_eq!(
            listed(&fixture, "Include ${UNDEFINED}\nHost web\n"),
            vec!["web"]
        );
    }
}
