//! Import of pasted scripts, port lists and bare forward specifications.

use std::collections::HashSet;

use super::comments::{logical_lines, strip_comments};
use super::destination::{AliasMatcher, describe};
use super::forward::{ParsedForward, parse_forward};
use super::line_scan::{LineScan, RawForward, scan_line};
use super::port_list::{PortEntry, extract_port_list};
use super::template::{has_placeholder, substitute};
use super::tokens::tokenize;
use super::{ImportCandidate, ImportPreview, ImportWarning};
use crate::external::SshDestination;
use crate::host_config::HostConfigResolver;
use crate::store::TunnelKind;

/// A candidate waiting for the alias of its destination.
struct Draft {
    candidate: ImportCandidate,
    destination: Option<SshDestination>,
}

/// What one forward specification gave.
struct Expansion {
    candidates: Vec<ImportCandidate>,
    warning: Option<ImportWarning>,
}

/// Reads the forwards of a pasted script.
///
/// Understands PowerShell and shell scripts calling `ssh`, lists of ports and
/// bare `-L`/`-R`/`-D` specifications. A forward template
/// (`-L ${port}:localhost:${port}`) is expanded with the ports of the
/// script's port list. Forwards are attributed to the `ssh_config` alias their
/// `ssh` destination stands for; when the script has exactly one destination,
/// forwards written outside an `ssh` command use it too.
pub fn preview_script(text: &str, resolver: &dyn HostConfigResolver) -> ImportPreview {
    let lines = strip_comments(text);
    let ports = extract_port_list(&lines);
    let scans: Vec<LineScan> = logical_lines(&lines)
        .iter()
        .map(|segments| scan_line(&tokenize(segments)))
        .collect();
    let fallback = single_destination(&scans);
    let forwards: Vec<&RawForward> = scans.iter().flat_map(|scan| &scan.forwards).collect();
    let has_templates = forwards
        .iter()
        .any(|forward| has_placeholder(&forward.text));
    let mut warnings = Vec::new();
    let mut drafts = forward_drafts(&forwards, &ports, fallback.as_ref(), &mut warnings);
    if !has_templates {
        drafts.extend(port_drafts(&ports, fallback.as_ref()));
        drafts.sort_by_key(|draft| draft.candidate.line);
    }
    let drafts = without_duplicates(drafts);
    attribute_hosts(drafts, resolver, warnings)
}

fn single_destination(scans: &[LineScan]) -> Option<SshDestination> {
    let mut distinct: Vec<&SshDestination> = Vec::new();
    for destination in scans.iter().flat_map(|scan| &scan.destinations) {
        if !distinct.contains(&destination) {
            distinct.push(destination);
        }
    }
    match distinct.as_slice() {
        [only] => Some((*only).clone()),
        _ => None,
    }
}

fn forward_drafts(
    forwards: &[&RawForward],
    ports: &[PortEntry],
    fallback: Option<&SshDestination>,
    warnings: &mut Vec<ImportWarning>,
) -> Vec<Draft> {
    let mut drafts = Vec::new();
    for forward in forwards {
        let expansion = expand(forward, ports);
        warnings.extend(expansion.warning);
        let destination = forward.destination.clone().or_else(|| fallback.cloned());
        drafts.extend(expansion.candidates.into_iter().map(|candidate| Draft {
            candidate,
            destination: destination.clone(),
        }));
    }
    drafts
}

fn expand(forward: &RawForward, ports: &[PortEntry]) -> Expansion {
    if !has_placeholder(&forward.text) {
        return expand_literal(forward);
    }
    if ports.is_empty() {
        return Expansion {
            candidates: Vec::new(),
            warning: Some(malformed(forward)),
        };
    }
    expand_template(forward, ports)
}

fn expand_literal(forward: &RawForward) -> Expansion {
    match parse_forward(forward.kind, &forward.text) {
        ParsedForward::Supported(candidate) => Expansion {
            candidates: vec![ImportCandidate {
                line: Some(forward.line),
                ..candidate
            }],
            warning: None,
        },
        ParsedForward::Unsupported => rejected(unsupported(forward)),
        ParsedForward::Malformed => rejected(malformed(forward)),
    }
}

fn expand_template(forward: &RawForward, ports: &[PortEntry]) -> Expansion {
    let mut candidates = Vec::new();
    for entry in ports {
        let text = substitute(&forward.text, entry.port);
        match parse_forward(forward.kind, &text) {
            ParsedForward::Supported(candidate) => candidates.push(ImportCandidate {
                comment: entry.comment.clone(),
                line: Some(entry.line),
                ..candidate
            }),
            ParsedForward::Unsupported => return rejected(unsupported(forward)),
            ParsedForward::Malformed => return rejected(malformed(forward)),
        }
    }
    Expansion {
        candidates,
        warning: None,
    }
}

fn rejected(warning: ImportWarning) -> Expansion {
    Expansion {
        candidates: Vec::new(),
        warning: Some(warning),
    }
}

fn malformed(forward: &RawForward) -> ImportWarning {
    ImportWarning::MalformedForward {
        line: forward.line,
        text: forward.text.clone(),
    }
}

fn unsupported(forward: &RawForward) -> ImportWarning {
    ImportWarning::UnsupportedForward {
        line: Some(forward.line),
        host: None,
        text: forward.text.clone(),
    }
}

fn port_drafts(ports: &[PortEntry], fallback: Option<&SshDestination>) -> Vec<Draft> {
    ports
        .iter()
        .map(|entry| Draft {
            candidate: ImportCandidate {
                host: None,
                kind: TunnelKind::Local,
                listen_address: None,
                listen_port: entry.port,
                target_host: None,
                target_port: None,
                comment: entry.comment.clone(),
                line: Some(entry.line),
            },
            destination: fallback.cloned(),
        })
        .collect()
}

fn without_duplicates(drafts: Vec<Draft>) -> Vec<Draft> {
    let mut seen = HashSet::new();
    drafts
        .into_iter()
        .filter(|draft| {
            let candidate = &draft.candidate;
            seen.insert((
                candidate.kind,
                candidate.listen_address.clone(),
                candidate.listen_port,
            ))
        })
        .collect()
}

fn note_unknown(
    unknown: &mut Vec<SshDestination>,
    host: Option<&String>,
    destination: Option<SshDestination>,
) {
    let Some(destination) = destination else {
        return;
    };
    if host.is_none() && !unknown.contains(&destination) {
        unknown.push(destination);
    }
}

fn attribute_hosts(
    drafts: Vec<Draft>,
    resolver: &dyn HostConfigResolver,
    mut warnings: Vec<ImportWarning>,
) -> ImportPreview {
    let mut matcher = AliasMatcher::new(resolver);
    let mut unknown: Vec<SshDestination> = Vec::new();
    let mut candidates = Vec::new();
    for draft in drafts {
        let host = draft
            .destination
            .as_ref()
            .and_then(|destination| matcher.alias_for(destination));
        note_unknown(&mut unknown, host.as_ref(), draft.destination);
        candidates.push(ImportCandidate {
            host,
            ..draft.candidate
        });
    }
    warnings.extend(
        unknown
            .iter()
            .map(|destination| ImportWarning::UnknownDestination {
                destination: describe(destination),
            }),
    );
    ImportPreview {
        candidates,
        warnings,
    }
}
