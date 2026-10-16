use std::path::Path;

use crate::atomic_file::{read_or_empty, replace_contents};
use crate::error::Error;
use crate::hashing::HashedHost;
use crate::key::HostKey;
use crate::lookup::HostLookup;

const LINE_FEED: u8 = b'\n';
const FORBIDDEN_NAME_CHARACTERS: &str = "@?*#[]|'\"\\!,";

/// How the host name is written in a new line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hashing {
    /// The name in clear text.
    Plain,
    /// A `|1|salt|hash` entry with a random salt (`HashKnownHosts yes`).
    Hashed,
}

/// Appends `key` for `lookup` to the file at `path`.
///
/// The existing lines are never rewritten or reordered. The file is replaced
/// atomically (temporary file next to it, fsync, rename), so a concurrent
/// writer can lose an update but a reader never sees a partial line. A missing
/// trailing newline in the existing content is added. See
/// [`HostLookup::lookup_name`] for the recorded name.
pub fn append_entry(
    path: &Path,
    lookup: &HostLookup,
    key: &HostKey,
    hashing: Hashing,
) -> Result<(), Error> {
    let name = recordable_name(lookup)?;
    let host_field = host_field(&name, hashing)?;
    let mut content = read_or_empty(path)?;
    ensure_trailing_newline(&mut content);
    content.extend_from_slice(format_line(&host_field, key).as_bytes());
    replace_contents(path, &content)
}

fn recordable_name(lookup: &HostLookup) -> Result<String, Error> {
    let name = lookup.lookup_name();
    let source = lookup.host_key_alias.as_deref().unwrap_or(&lookup.hostname);
    let is_invalid = source.is_empty()
        || source.chars().any(|character| {
            character.is_whitespace() || FORBIDDEN_NAME_CHARACTERS.contains(character)
        });
    if is_invalid {
        return Err(Error::InvalidHostName {
            name: source.to_owned(),
        });
    }
    Ok(name)
}

fn host_field(name: &str, hashing: Hashing) -> Result<String, Error> {
    match hashing {
        Hashing::Plain => Ok(name.to_owned()),
        Hashing::Hashed => HashedHost::with_random_salt(name).map(|hashed| hashed.to_string()),
    }
}

fn ensure_trailing_newline(content: &mut Vec<u8>) {
    if content.last().is_some_and(|byte| *byte != LINE_FEED) {
        content.push(LINE_FEED);
    }
}

fn format_line(host_field: &str, key: &HostKey) -> String {
    format!("{host_field} {} {}\n", key.algorithm, key.to_base64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::tests::key_for;
    use crate::known_hosts::KnownHosts;
    use crate::lookup::PresentedKey;
    use crate::verdict::Verdict;
    use pretty_assertions::assert_eq;
    use std::fs;

    fn key() -> HostKey {
        key_for("ssh-ed25519", b"payload")
    }

    fn read_lines(path: &Path) -> Vec<String> {
        fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn should_write_plain_line_when_hashing_is_plain() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");

        append_entry(&path, &HostLookup::new("Host", 22), &key(), Hashing::Plain).unwrap();

        assert_eq!(
            read_lines(&path),
            vec![format!("host ssh-ed25519 {}", key().to_base64())]
        );
    }

    #[test]
    fn should_write_bracketed_name_when_port_is_not_default() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");

        append_entry(
            &path,
            &HostLookup::new("host", 2222),
            &key(),
            Hashing::Plain,
        )
        .unwrap();

        assert!(read_lines(&path)[0].starts_with("[host]:2222 "));
    }

    #[test]
    fn should_write_alias_without_port_when_alias_is_set() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");
        let lookup = HostLookup {
            host_key_alias: Some("jump".to_owned()),
            ..HostLookup::new("host", 2222)
        };

        append_entry(&path, &lookup, &key(), Hashing::Plain).unwrap();

        assert!(read_lines(&path)[0].starts_with("jump "));
    }

    #[test]
    fn should_write_hashed_field_when_hashing_is_hashed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");

        append_entry(&path, &HostLookup::new("host", 22), &key(), Hashing::Hashed).unwrap();

        assert!(read_lines(&path)[0].starts_with("|1|"));
    }

    #[test]
    fn should_hash_bracketed_name_when_port_is_not_default() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");
        let lookup = HostLookup::new("host", 2222);

        append_entry(&path, &lookup, &key(), Hashing::Hashed).unwrap();

        let known = KnownHosts::load(&[path], &[]).unwrap().known_hosts;
        assert!(matches!(
            known.check(&lookup, &PresentedKey::Plain(key())),
            Verdict::Trusted { line: 1, .. }
        ));
    }

    #[test]
    fn should_not_match_plain_name_when_only_bracketed_name_was_hashed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");

        append_entry(
            &path,
            &HostLookup::new("host", 2222),
            &key(),
            Hashing::Hashed,
        )
        .unwrap();

        let known = KnownHosts::load(&[path], &[]).unwrap().known_hosts;
        assert!(matches!(
            known.check(&HostLookup::new("host", 22), &PresentedKey::Plain(key())),
            Verdict::Unknown
        ));
    }

    #[test]
    fn should_add_newline_when_existing_content_has_none() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");
        fs::write(&path, "# no trailing newline").unwrap();

        append_entry(&path, &HostLookup::new("host", 22), &key(), Hashing::Plain).unwrap();

        assert_eq!(read_lines(&path)[0], "# no trailing newline");
        assert_eq!(read_lines(&path).len(), 2);
    }

    #[test]
    fn should_not_add_blank_line_when_existing_content_ends_with_newline() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");
        fs::write(&path, "# header\n").unwrap();

        append_entry(&path, &HostLookup::new("host", 22), &key(), Hashing::Plain).unwrap();

        assert_eq!(read_lines(&path).len(), 2);
    }

    #[test]
    fn should_end_with_newline_when_line_is_appended() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");

        append_entry(&path, &HostLookup::new("host", 22), &key(), Hashing::Plain).unwrap();

        assert!(fs::read(&path).unwrap().ends_with(b"\n"));
    }

    #[test]
    fn should_keep_existing_lines_unchanged_when_appending() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");
        let original = "zeta ssh-rsa AAAA\r\n@revoked b junk\n# keep\n\n";
        fs::write(&path, original).unwrap();

        append_entry(&path, &HostLookup::new("host", 22), &key(), Hashing::Plain).unwrap();

        assert!(fs::read_to_string(&path).unwrap().starts_with(original));
    }

    #[test]
    fn should_create_missing_parent_directory_when_file_is_new() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".ssh").join("known_hosts");

        append_entry(&path, &HostLookup::new("host", 22), &key(), Hashing::Plain).unwrap();

        assert!(path.is_file());
    }

    #[test]
    fn should_reject_hostname_when_it_contains_a_wildcard() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");

        let outcome = append_entry(
            &path,
            &HostLookup::new("*.example.com", 22),
            &key(),
            Hashing::Plain,
        );

        assert!(matches!(outcome, Err(Error::InvalidHostName { .. })));
        assert!(!path.exists());
    }

    #[test]
    fn should_reject_hostname_when_it_contains_whitespace() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");

        let outcome = append_entry(&path, &HostLookup::new("a b", 22), &key(), Hashing::Plain);

        assert!(matches!(outcome, Err(Error::InvalidHostName { .. })));
    }

    #[test]
    fn should_report_error_with_path_when_target_is_a_directory() {
        let directory = tempfile::tempdir().unwrap();

        let outcome = append_entry(
            directory.path(),
            &HostLookup::new("host", 22),
            &key(),
            Hashing::Plain,
        );

        assert!(matches!(outcome, Err(Error::Io { path, .. }) if path == directory.path()));
    }

    #[cfg(unix)]
    #[test]
    fn should_leave_no_temporary_file_when_appending_fails() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known_hosts");
        fs::write(&path, "# header\n").unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o500)).unwrap();

        let outcome = append_entry(&path, &HostLookup::new("host", 22), &key(), Hashing::Plain);

        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        assert!(outcome.is_err());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
