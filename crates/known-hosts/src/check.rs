use crate::entry::{Entry, EntryKind};
use crate::key::HostKey;
use crate::lookup::PresentedKey;
use crate::verdict::Verdict;

/// Judges `presented` against the entries that apply to one name.
pub(crate) fn judge(applicable: &[&Entry], presented: &PresentedKey) -> Verdict {
    if let Some(revoking) = find_revocation(applicable, presented) {
        return Verdict::Revoked {
            path: revoking.path.to_path_buf(),
            line: revoking.line,
        };
    }
    match presented {
        PresentedKey::Plain(key) => judge_plain(applicable, key),
        PresentedKey::Certificate { signing_authority } => {
            judge_authority(applicable, signing_authority)
        }
    }
}

fn find_revocation<'entry>(
    applicable: &[&'entry Entry],
    presented: &PresentedKey,
) -> Option<&'entry Entry> {
    let revoked_key = match presented {
        PresentedKey::Plain(key) => key,
        PresentedKey::Certificate { signing_authority } => signing_authority,
    };
    applicable
        .iter()
        .find(|entry| entry.kind == EntryKind::Revoked && entry.key == *revoked_key)
        .copied()
}

fn judge_plain(applicable: &[&Entry], key: &HostKey) -> Verdict {
    let mut last_conflict: Option<&Entry> = None;
    for entry in entries_of_kind(applicable, EntryKind::Trusted) {
        if entry.key == *key {
            return trusted(entry);
        }
        last_conflict = Some(entry);
    }
    match last_conflict {
        Some(conflicting) => Verdict::Changed {
            path: conflicting.path.to_path_buf(),
            line: conflicting.line,
            known: conflicting.key.clone(),
        },
        None => Verdict::Unknown,
    }
}

fn judge_authority(applicable: &[&Entry], signing_authority: &HostKey) -> Verdict {
    entries_of_kind(applicable, EntryKind::CertificateAuthority)
        .find(|entry| entry.key == *signing_authority)
        .map_or(Verdict::Unknown, trusted)
}

/// Algorithms of the plain entries, in first-seen order, without duplicates.
pub(crate) fn known_algorithms(applicable: &[&Entry]) -> Vec<String> {
    let mut algorithms: Vec<String> = Vec::new();
    for entry in entries_of_kind(applicable, EntryKind::Trusted) {
        if !algorithms.contains(&entry.key.algorithm) {
            algorithms.push(entry.key.algorithm.clone());
        }
    }
    algorithms
}

fn entries_of_kind<'entry>(
    applicable: &'entry [&'entry Entry],
    kind: EntryKind,
) -> impl Iterator<Item = &'entry Entry> {
    applicable
        .iter()
        .copied()
        .filter(move |entry| entry.kind == kind)
}

fn trusted(entry: &Entry) -> Verdict {
    Verdict::Trusted {
        path: entry.path.to_path_buf(),
        line: entry.line,
    }
}
