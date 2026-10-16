use std::path::PathBuf;

use crate::check::{judge, known_algorithms};
use crate::entry::Entry;
use crate::error::{Error, Warning};
use crate::lookup::{HostLookup, PresentedKey};
use crate::parser::{ParsedFile, read_file};
use crate::verdict::Verdict;

/// The entries of the user and global `known_hosts` files.
#[derive(Debug, Default)]
pub struct KnownHosts {
    entries: Vec<Entry>,
}

/// A loaded set of files and the warnings for the lines that were skipped.
#[derive(Debug)]
pub struct Loaded {
    /// The valid entries.
    pub known_hosts: KnownHosts,
    /// One warning per malformed line.
    pub warnings: Vec<Warning>,
}

impl KnownHosts {
    /// Reads the user files, then the global files, in the order given.
    ///
    /// A missing file reads as empty and a malformed line is skipped with a
    /// warning. Any other I/O failure is an error carrying the path.
    pub fn load(user_files: &[PathBuf], global_files: &[PathBuf]) -> Result<Loaded, Error> {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();
        for path in user_files.iter().chain(global_files) {
            let ParsedFile {
                entries: file_entries,
                warnings: file_warnings,
            } = read_file(path)?;
            entries.extend(file_entries);
            warnings.extend(file_warnings);
        }
        Ok(Loaded {
            known_hosts: Self { entries },
            warnings,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_content(path: &std::path::Path, content: &str) -> Self {
        let parsed = crate::parser::parse_content(path, content.as_bytes());
        Self {
            entries: parsed.entries,
        }
    }

    /// Judges the key a server presented, following OpenSSH 10.0
    /// (`check_key_in_hostkeys` and `check_host_key`).
    ///
    /// Only the lines whose host patterns match the lookup name apply, for
    /// markers as well. Lines are consulted user files first, then global
    /// files, each in file order. Precedence:
    ///
    /// 1. [`Verdict::Revoked`]: a `@revoked` line equal to the key (for a
    ///    certificate: to its signing authority) wins over everything.
    /// 2. Plain key: the first plain line equal to the key gives
    ///    [`Verdict::Trusted`], in any file, even when another line has a
    ///    different key. `@cert-authority` lines are ignored.
    /// 3. Otherwise, if any plain line applies, whatever its algorithm, the
    ///    result is [`Verdict::Changed`], reporting the last such line as
    ///    OpenSSH does. This is a hard failure on purpose: a server must not
    ///    be able to get a "new host" prompt by presenting an algorithm not
    ///    yet recorded. Use [`KnownHosts::known_algorithms`] to negotiate an
    ///    algorithm the host has a key for.
    /// 4. Certificate: the first `@cert-authority` line equal to the signing
    ///    authority gives [`Verdict::Trusted`]. Plain lines are ignored and
    ///    there is no `Changed` for certificates.
    /// 5. Otherwise [`Verdict::Unknown`]: no applicable entry.
    /// 6. When the result for `[host]:port` is `Unknown` (port other than 22,
    ///    no alias), the plain host name is consulted as OpenSSH does
    ///    ("checking without port identifier"): `Trusted` carries over and
    ///    anything else stays `Unknown`. `Revoked` also carries over, which is
    ///    stricter than OpenSSH.
    pub fn check(&self, lookup: &HostLookup, presented: &PresentedKey) -> Verdict {
        let verdict = self.judge_name(&lookup.lookup_name(), presented);
        if !lookup.has_port_fallback() || verdict != Verdict::Unknown {
            return verdict;
        }
        match self.judge_name(&lookup.plain_name(), presented) {
            carried @ (Verdict::Trusted { .. } | Verdict::Revoked { .. }) => carried,
            _ => verdict,
        }
    }

    /// The algorithms of the plain entries that apply to `lookup`, in
    /// first-seen order without duplicates.
    ///
    /// The engine uses it to prefer host key algorithms the host already has a
    /// key for (OpenSSH `order_hostkeyalgs`), which avoids a false
    /// [`Verdict::Changed`]. The plain-name port fallback of
    /// [`KnownHosts::check`] applies when `[host]:port` has no entry.
    pub fn known_algorithms(&self, lookup: &HostLookup) -> Vec<String> {
        let algorithms = known_algorithms(&self.applicable_entries(&lookup.lookup_name()));
        if !algorithms.is_empty() || !lookup.has_port_fallback() {
            return algorithms;
        }
        known_algorithms(&self.applicable_entries(&lookup.plain_name()))
    }

    fn judge_name(&self, name: &str, presented: &PresentedKey) -> Verdict {
        judge(&self.applicable_entries(name), presented)
    }

    fn applicable_entries(&self, name: &str) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.patterns.matches(name))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::HashedHost;
    use crate::key::HostKey;
    use crate::key::tests::key_for;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    const FILE: &str = "known_hosts";

    fn ed25519(payload: &str) -> HostKey {
        key_for("ssh-ed25519", payload.as_bytes())
    }

    fn rsa(payload: &str) -> HostKey {
        key_for("ssh-rsa", payload.as_bytes())
    }

    fn line(hosts: &str, key: &HostKey) -> String {
        format!("{hosts} {} {}\n", key.algorithm, key.to_base64())
    }

    fn known_hosts(content: &str) -> KnownHosts {
        KnownHosts::from_content(Path::new(FILE), content)
    }

    fn check(known: &KnownHosts, host: &str, key: &HostKey) -> Verdict {
        known.check(
            &HostLookup::new(host, 22),
            &PresentedKey::Plain(key.clone()),
        )
    }

    fn trusted_at(line: usize) -> Verdict {
        Verdict::Trusted {
            path: FILE.into(),
            line,
        }
    }

    #[test]
    fn should_trust_key_when_host_entry_has_same_key() {
        let known = known_hosts(&line("host", &ed25519("a")));

        assert_eq!(check(&known, "host", &ed25519("a")), trusted_at(1));
    }

    #[test]
    fn should_trust_key_when_lookup_hostname_has_other_case() {
        let known = known_hosts(&line("host", &ed25519("a")));

        assert_eq!(check(&known, "HOST", &ed25519("a")), trusted_at(1));
    }

    #[test]
    fn should_trust_key_when_entry_is_hashed() {
        let hashed = HashedHost::with_salt("host", [3; 20]).to_string();
        let known = known_hosts(&line(&hashed, &ed25519("a")));

        assert_eq!(check(&known, "host", &ed25519("a")), trusted_at(1));
    }

    #[test]
    fn should_report_unknown_when_host_has_no_entry() {
        let known = known_hosts(&line("other", &ed25519("a")));

        assert_eq!(check(&known, "host", &ed25519("a")), Verdict::Unknown);
    }

    #[test]
    fn should_report_changed_when_only_a_different_algorithm_is_known() {
        let known = known_hosts(&format!(
            "{}{}",
            line("host", &rsa("a")),
            line("host", &rsa("b"))
        ));

        assert_eq!(
            check(&known, "host", &ed25519("a")),
            Verdict::Changed {
                path: FILE.into(),
                line: 2,
                known: rsa("b"),
            }
        );
    }

    #[test]
    fn should_report_changed_when_host_has_same_algorithm_with_other_key() {
        let known = known_hosts(&line("host", &ed25519("a")));

        assert_eq!(
            check(&known, "host", &ed25519("b")),
            Verdict::Changed {
                path: FILE.into(),
                line: 1,
                known: ed25519("a"),
            }
        );
    }

    #[test]
    fn should_report_last_conflicting_line_when_several_lines_conflict() {
        let known = known_hosts(&format!(
            "{}{}",
            line("host", &ed25519("a")),
            line("host", &ed25519("b"))
        ));

        assert!(matches!(
            check(&known, "host", &ed25519("c")),
            Verdict::Changed { line: 2, .. }
        ));
    }

    #[test]
    fn should_trust_key_when_a_later_line_matches_after_a_conflicting_one() {
        let known = known_hosts(&format!(
            "{}{}",
            line("host", &ed25519("a")),
            line("host", &ed25519("b"))
        ));

        assert_eq!(check(&known, "host", &ed25519("b")), trusted_at(2));
    }

    #[test]
    fn should_trust_key_when_it_matches_a_pattern_line_after_a_conflicting_plain_line() {
        let known = known_hosts(&format!(
            "{}{}",
            line("host", &ed25519("a")),
            line("*", &ed25519("b"))
        ));

        assert_eq!(check(&known, "host", &ed25519("b")), trusted_at(2));
    }

    #[test]
    fn should_not_apply_line_when_negated_pattern_matches() {
        let known = known_hosts(&line("*,!host", &ed25519("a")));

        assert_eq!(check(&known, "host", &ed25519("a")), Verdict::Unknown);
    }

    #[test]
    fn should_report_revoked_when_revoked_line_matches_host_and_key() {
        let known = known_hosts(&line("@revoked host", &ed25519("a")));

        assert_eq!(
            check(&known, "host", &ed25519("a")),
            Verdict::Revoked {
                path: FILE.into(),
                line: 1
            }
        );
    }

    #[test]
    fn should_report_revoked_when_key_is_also_trusted_on_another_line() {
        let known = known_hosts(&format!(
            "{}{}",
            line("host", &ed25519("a")),
            line("@revoked *", &ed25519("a"))
        ));

        assert!(matches!(
            check(&known, "host", &ed25519("a")),
            Verdict::Revoked { line: 2, .. }
        ));
    }

    #[test]
    fn should_not_report_revoked_when_revoked_line_does_not_match_host() {
        let known = known_hosts(&format!(
            "{}{}",
            line("host", &ed25519("a")),
            line("@revoked other", &ed25519("a"))
        ));

        assert_eq!(check(&known, "host", &ed25519("a")), trusted_at(1));
    }

    #[test]
    fn should_not_report_revoked_when_revoked_line_holds_another_key() {
        let known = known_hosts(&format!(
            "{}{}",
            line("host", &ed25519("a")),
            line("@revoked host", &ed25519("b"))
        ));

        assert_eq!(check(&known, "host", &ed25519("a")), trusted_at(1));
    }

    #[test]
    fn should_not_trust_plain_key_when_it_only_matches_a_cert_authority_line() {
        let known = known_hosts(&line("@cert-authority host", &ed25519("a")));

        assert_eq!(check(&known, "host", &ed25519("a")), Verdict::Unknown);
    }

    fn certificate(authority: &HostKey) -> PresentedKey {
        PresentedKey::Certificate {
            signing_authority: authority.clone(),
        }
    }

    #[test]
    fn should_trust_certificate_when_cert_authority_line_matches_host_and_authority() {
        let known = known_hosts(&line("@cert-authority *.example.com", &ed25519("ca")));

        assert_eq!(
            known.check(
                &HostLookup::new("web.example.com", 22),
                &certificate(&ed25519("ca"))
            ),
            trusted_at(1)
        );
    }

    #[test]
    fn should_not_trust_certificate_when_authority_line_does_not_match_host() {
        let known = known_hosts(&line("@cert-authority *.example.com", &ed25519("ca")));

        assert_eq!(
            known.check(
                &HostLookup::new("web.example.org", 22),
                &certificate(&ed25519("ca"))
            ),
            Verdict::Unknown
        );
    }

    #[test]
    fn should_not_trust_certificate_when_only_a_plain_line_holds_the_authority() {
        let known = known_hosts(&line("host", &ed25519("ca")));

        assert_eq!(
            known.check(&HostLookup::new("host", 22), &certificate(&ed25519("ca"))),
            Verdict::Unknown
        );
    }

    #[test]
    fn should_report_unknown_when_certificate_authority_differs() {
        let known = known_hosts(&line("@cert-authority host", &ed25519("ca")));

        assert_eq!(
            known.check(
                &HostLookup::new("host", 22),
                &certificate(&ed25519("other"))
            ),
            Verdict::Unknown
        );
    }

    #[test]
    fn should_report_revoked_when_certificate_authority_is_revoked() {
        let known = known_hosts(&format!(
            "{}{}",
            line("@cert-authority host", &ed25519("ca")),
            line("@revoked host", &ed25519("ca"))
        ));

        assert!(matches!(
            known.check(&HostLookup::new("host", 22), &certificate(&ed25519("ca"))),
            Verdict::Revoked { line: 2, .. }
        ));
    }

    #[test]
    fn should_look_up_bracketed_entry_when_port_is_not_default() {
        let known = known_hosts(&line("[host]:2222", &ed25519("a")));

        assert_eq!(
            known.check(
                &HostLookup::new("host", 2222),
                &PresentedKey::Plain(ed25519("a"))
            ),
            trusted_at(1)
        );
    }

    #[test]
    fn should_not_use_bracketed_entry_when_port_is_default() {
        let known = known_hosts(&line("[host]:2222", &ed25519("a")));

        assert_eq!(check(&known, "host", &ed25519("a")), Verdict::Unknown);
    }

    #[test]
    fn should_fall_back_to_plain_entry_when_port_is_not_default_and_nothing_is_known() {
        let known = known_hosts(&line("host", &ed25519("a")));

        assert_eq!(
            known.check(
                &HostLookup::new("host", 2222),
                &PresentedKey::Plain(ed25519("a"))
            ),
            trusted_at(1)
        );
    }

    #[test]
    fn should_not_fall_back_to_plain_entry_when_bracketed_entry_has_other_key() {
        let known = known_hosts(&format!(
            "{}{}",
            line("[host]:2222", &ed25519("b")),
            line("host", &ed25519("a"))
        ));

        assert!(matches!(
            known.check(
                &HostLookup::new("host", 2222),
                &PresentedKey::Plain(ed25519("a"))
            ),
            Verdict::Changed { line: 1, .. }
        ));
    }

    #[test]
    fn should_stay_unknown_when_plain_entry_has_other_key_on_fallback() {
        let known = known_hosts(&line("host", &ed25519("b")));

        assert_eq!(
            known.check(
                &HostLookup::new("host", 2222),
                &PresentedKey::Plain(ed25519("a"))
            ),
            Verdict::Unknown
        );
    }

    #[test]
    fn should_report_revoked_when_plain_name_revokes_key_on_fallback() {
        let known = known_hosts(&line("@revoked host", &ed25519("a")));

        assert!(matches!(
            known.check(
                &HostLookup::new("host", 2222),
                &PresentedKey::Plain(ed25519("a"))
            ),
            Verdict::Revoked { .. }
        ));
    }

    #[test]
    fn should_look_up_alias_without_port_when_alias_is_set() {
        let known = known_hosts(&line("jump", &ed25519("a")));
        let lookup = HostLookup {
            host_key_alias: Some("jump".to_owned()),
            ..HostLookup::new("real.example.com", 2222)
        };

        assert_eq!(
            known.check(&lookup, &PresentedKey::Plain(ed25519("a"))),
            trusted_at(1)
        );
    }

    #[test]
    fn should_not_fall_back_to_hostname_when_alias_is_set() {
        let known = known_hosts(&line("real.example.com", &ed25519("a")));
        let lookup = HostLookup {
            host_key_alias: Some("jump".to_owned()),
            ..HostLookup::new("real.example.com", 22)
        };

        assert_eq!(
            known.check(&lookup, &PresentedKey::Plain(ed25519("a"))),
            Verdict::Unknown
        );
    }

    fn algorithms(known: &KnownHosts, lookup: &HostLookup) -> Vec<String> {
        known.known_algorithms(lookup)
    }

    #[test]
    fn should_list_algorithms_in_first_seen_order_without_duplicates() {
        let known = known_hosts(&format!(
            "{}{}{}",
            line("host", &rsa("a")),
            line("host", &ed25519("a")),
            line("host", &rsa("b"))
        ));

        assert_eq!(
            algorithms(&known, &HostLookup::new("host", 22)),
            vec!["ssh-rsa", "ssh-ed25519"]
        );
    }

    #[test]
    fn should_ignore_marked_and_unrelated_lines_when_listing_algorithms() {
        let known = known_hosts(&format!(
            "{}{}{}",
            line("@cert-authority host", &rsa("a")),
            line("@revoked host", &rsa("b")),
            line("other", &rsa("c"))
        ));

        assert_eq!(
            algorithms(&known, &HostLookup::new("host", 22)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn should_list_plain_name_algorithms_when_port_is_not_default_and_bracketed_name_is_unknown() {
        let known = known_hosts(&line("host", &rsa("a")));

        assert_eq!(
            algorithms(&known, &HostLookup::new("host", 2222)),
            vec!["ssh-rsa"]
        );
    }

    #[test]
    fn should_list_bracketed_name_algorithms_only_when_it_has_entries() {
        let known = known_hosts(&format!(
            "{}{}",
            line("[host]:2222", &ed25519("a")),
            line("host", &rsa("a"))
        ));

        assert_eq!(
            algorithms(&known, &HostLookup::new("host", 2222)),
            vec!["ssh-ed25519"]
        );
    }

    #[test]
    fn should_not_fall_back_when_listing_algorithms_for_an_alias() {
        let known = known_hosts(&line("host", &rsa("a")));
        let lookup = HostLookup {
            host_key_alias: Some("jump".to_owned()),
            ..HostLookup::new("host", 2222)
        };

        assert_eq!(algorithms(&known, &lookup), Vec::<String>::new());
    }

    #[test]
    fn should_report_global_line_when_user_and_global_files_both_conflict() {
        let directory = tempfile::tempdir().unwrap();
        let user = directory.path().join("user");
        let global = directory.path().join("global");
        std::fs::write(&user, line("host", &ed25519("a"))).unwrap();
        std::fs::write(&global, line("host", &ed25519("b"))).unwrap();

        let loaded = KnownHosts::load(&[user], &[global]).unwrap();

        assert_eq!(
            check(&loaded.known_hosts, "host", &ed25519("c")),
            Verdict::Changed {
                path: directory.path().join("global"),
                line: 1,
                known: ed25519("b"),
            }
        );
    }

    #[test]
    fn should_trust_key_when_only_the_global_file_holds_it() {
        let directory = tempfile::tempdir().unwrap();
        let user = directory.path().join("user");
        let global = directory.path().join("global");
        std::fs::write(&user, line("host", &ed25519("a"))).unwrap();
        std::fs::write(&global, line("host", &ed25519("b"))).unwrap();

        let loaded = KnownHosts::load(&[user], std::slice::from_ref(&global)).unwrap();

        assert_eq!(
            check(&loaded.known_hosts, "host", &ed25519("b")),
            Verdict::Trusted {
                path: global,
                line: 1
            }
        );
    }

    #[test]
    fn should_collect_warnings_from_every_file_when_loading() {
        let directory = tempfile::tempdir().unwrap();
        let user = directory.path().join("user");
        let global = directory.path().join("global");
        std::fs::write(&user, "garbage\n").unwrap();
        std::fs::write(&global, "host ssh-ed25519\n").unwrap();

        let loaded = KnownHosts::load(&[user], &[global]).unwrap();

        assert_eq!(loaded.warnings.len(), 2);
    }

    #[test]
    fn should_load_empty_when_files_are_missing() {
        let directory = tempfile::tempdir().unwrap();

        let loaded = KnownHosts::load(&[directory.path().join("absent")], &[]).unwrap();

        assert_eq!(
            check(&loaded.known_hosts, "host", &ed25519("a")),
            Verdict::Unknown
        );
    }
}
