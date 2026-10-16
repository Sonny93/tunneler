use std::path::Path;
use std::sync::Arc;

use crate::error::WarningKind;
use crate::key::HostKey;
use crate::pattern::HostPatterns;

const MARKER_PREFIX: char = '@';
const CERT_AUTHORITY_MARKER: &str = "@cert-authority";
const REVOKED_MARKER: &str = "@revoked";
const COMMENT_PREFIX: char = '#';
const FIELD_SEPARATORS: [char; 2] = [' ', '\t'];

/// What a line says about its key, given by its optional marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Trusted,
    CertificateAuthority,
    Revoked,
}

/// A valid `known_hosts` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) kind: EntryKind,
    pub(crate) patterns: HostPatterns,
    pub(crate) key: HostKey,
    pub(crate) path: Arc<Path>,
    pub(crate) line: usize,
}

/// A line without its position in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedLine {
    pub(crate) kind: EntryKind,
    pub(crate) patterns: HostPatterns,
    pub(crate) key: HostKey,
}

/// Parses one line: `Ok(None)` for blank lines and comments.
pub(crate) fn parse_line(line: &str) -> Result<Option<ParsedLine>, WarningKind> {
    let line = line.trim_start_matches(FIELD_SEPARATORS);
    if line.is_empty() || line.starts_with(COMMENT_PREFIX) {
        return Ok(None);
    }
    let (kind, remainder) = split_marker(line)?;
    let (hosts, remainder) = split_field(remainder, "host names")?;
    let (algorithm, remainder) = split_field(remainder, "key type")?;
    let (encoded_key, _comment) = split_field(remainder, "key")?;
    let key = HostKey::from_base64(algorithm, encoded_key)
        .map_err(|source| WarningKind::InvalidKey { source })?;
    Ok(Some(ParsedLine {
        kind,
        patterns: HostPatterns::parse(hosts)?,
        key,
    }))
}

fn split_marker(line: &str) -> Result<(EntryKind, &str), WarningKind> {
    if !line.starts_with(MARKER_PREFIX) {
        return Ok((EntryKind::Trusted, line));
    }
    let (marker, remainder) = line
        .split_once(FIELD_SEPARATORS)
        .ok_or_else(|| unknown_marker(line))?;
    let remainder = remainder.trim_start_matches(FIELD_SEPARATORS);
    let kind = match marker {
        CERT_AUTHORITY_MARKER => EntryKind::CertificateAuthority,
        REVOKED_MARKER => EntryKind::Revoked,
        _ => return Err(unknown_marker(marker)),
    };
    if remainder.starts_with(MARKER_PREFIX) {
        return Err(WarningKind::DuplicateMarker);
    }
    Ok((kind, remainder))
}

fn unknown_marker(marker: &str) -> WarningKind {
    WarningKind::UnknownMarker {
        marker: marker.to_owned(),
    }
}

fn split_field<'line>(
    text: &'line str,
    field: &'static str,
) -> Result<(&'line str, &'line str), WarningKind> {
    let is_missing = text.is_empty() || text.starts_with(COMMENT_PREFIX);
    if is_missing {
        return Err(WarningKind::MissingField { field });
    }
    let (value, remainder) = text.split_once(FIELD_SEPARATORS).unwrap_or((text, ""));
    Ok((value, remainder.trim_start_matches(FIELD_SEPARATORS)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::WarningKind::{
        DuplicateMarker, InvalidHostHash, InvalidKey, MissingField, UnknownMarker,
    };
    use crate::key::HostKeyError;
    use crate::key::tests::key_for;
    use pretty_assertions::assert_eq;

    fn line_for(prefix: &str, algorithm: &str, suffix: &str) -> String {
        let key = key_for(algorithm, b"payload");
        format!("{prefix}{algorithm} {}{suffix}", key.to_base64())
    }

    fn parse(line: &str) -> Result<Option<ParsedLine>, WarningKind> {
        parse_line(line)
    }

    #[test]
    fn should_skip_line_when_it_is_blank() {
        assert_eq!(parse("   \t"), Ok(None));
    }

    #[test]
    fn should_skip_line_when_it_is_a_comment() {
        assert_eq!(parse("  # a comment"), Ok(None));
    }

    #[test]
    fn should_parse_trusted_entry_when_line_has_no_marker() {
        let line = line_for("host ", "ssh-ed25519", "");

        assert_eq!(parse(&line).unwrap().unwrap().kind, EntryKind::Trusted);
    }

    #[test]
    fn should_accept_tabs_and_leading_whitespace_when_separating_fields() {
        let key = key_for("ssh-ed25519", b"payload");
        let line = format!("  host\tssh-ed25519 \t{}", key.to_base64());

        assert_eq!(parse(&line).unwrap().unwrap().key, key);
    }

    #[test]
    fn should_ignore_trailing_comment_when_line_has_one() {
        let line = line_for("host ", "ssh-ed25519", " user@laptop");

        assert!(parse(&line).unwrap().is_some());
    }

    #[test]
    fn should_tolerate_carriage_return_after_comment_when_line_is_crlf() {
        let line = line_for("host ", "ssh-ed25519", " comment\r");

        assert!(parse(&line).unwrap().is_some());
    }

    #[test]
    fn should_parse_certificate_authority_when_marker_is_cert_authority() {
        let line = line_for("@cert-authority *.example.com ", "ssh-ed25519", "");

        assert_eq!(
            parse(&line).unwrap().unwrap().kind,
            EntryKind::CertificateAuthority
        );
    }

    #[test]
    fn should_parse_revoked_entry_when_marker_is_revoked() {
        let line = line_for("@revoked host ", "ssh-ed25519", "");

        assert_eq!(parse(&line).unwrap().unwrap().kind, EntryKind::Revoked);
    }

    #[test]
    fn should_reject_line_when_marker_is_unknown() {
        let line = line_for("@trusted host ", "ssh-ed25519", "");

        assert_eq!(
            parse(&line),
            Err(UnknownMarker {
                marker: "@trusted".to_owned()
            })
        );
    }

    #[test]
    fn should_reject_line_when_marker_is_alone() {
        assert_eq!(
            parse("@revoked"),
            Err(UnknownMarker {
                marker: "@revoked".to_owned()
            })
        );
    }

    #[test]
    fn should_reject_line_when_two_markers_are_present() {
        let line = line_for("@revoked @cert-authority host ", "ssh-ed25519", "");

        assert_eq!(parse(&line), Err(DuplicateMarker));
    }

    #[test]
    fn should_reject_line_when_key_type_is_missing() {
        assert_eq!(parse("host"), Err(MissingField { field: "key type" }));
    }

    #[test]
    fn should_reject_line_when_key_is_missing() {
        assert_eq!(
            parse("host ssh-ed25519"),
            Err(MissingField { field: "key" })
        );
    }

    #[test]
    fn should_reject_line_when_key_is_replaced_by_a_comment() {
        assert_eq!(
            parse("host ssh-ed25519 #comment"),
            Err(MissingField { field: "key" })
        );
    }

    #[test]
    fn should_reject_line_when_key_is_not_base64() {
        assert_eq!(
            parse("host ssh-ed25519 !!!"),
            Err(InvalidKey {
                source: HostKeyError::InvalidBase64
            })
        );
    }

    #[test]
    fn should_reject_line_when_key_type_does_not_match_blob() {
        let key = key_for("ssh-rsa", b"payload");
        let line = format!("host ssh-ed25519 {}", key.to_base64());

        assert_eq!(
            parse(&line),
            Err(InvalidKey {
                source: HostKeyError::AlgorithmMismatch {
                    declared: "ssh-ed25519".to_owned(),
                    embedded: "ssh-rsa".to_owned(),
                }
            })
        );
    }

    #[test]
    fn should_reject_line_when_hashed_host_is_malformed() {
        let line = line_for("|1|bad ", "ssh-ed25519", "");

        assert_eq!(parse(&line), Err(InvalidHostHash));
    }
}
