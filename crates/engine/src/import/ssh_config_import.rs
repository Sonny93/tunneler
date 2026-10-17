//! Import of the forwards declared in the `ssh_config` files.

use ssh_config::RemoteForward;

use super::forward::{dynamic_candidate, fixed_candidate};
use super::{ImportCandidate, ImportPreview, ImportWarning};
use crate::host_config::{HostConfigError, HostConfigResolver};
use crate::store::TunnelKind;

/// Lists the `LocalForward`, `RemoteForward` and `DynamicForward` of every
/// concrete `Host` alias, in the order of [`HostConfigResolver::list_hosts`].
///
/// Unix socket forwards and remote SOCKS proxies are reported as warnings; an
/// alias whose configuration cannot be resolved is reported and skipped.
pub fn preview_ssh_config(
    resolver: &dyn HostConfigResolver,
) -> Result<ImportPreview, HostConfigError> {
    let mut preview = ImportPreview {
        candidates: Vec::new(),
        warnings: Vec::new(),
    };
    for alias in resolver.list_hosts()? {
        match resolver.resolve(&alias) {
            Ok(config) => read_forwards(&alias, &config, &mut preview),
            Err(error) => preview.warnings.push(ImportWarning::HostResolutionFailed {
                host: alias,
                message: error.to_string(),
            }),
        }
    }
    Ok(preview)
}

fn read_forwards(alias: &str, config: &ssh_config::ResolvedConfig, preview: &mut ImportPreview) {
    for spec in &config.local_forwards {
        let candidate = fixed_candidate(TunnelKind::Local, spec);
        record(alias, candidate, spec.ssh_g_form(), preview);
    }
    for forward in &config.remote_forwards {
        let candidate = match forward {
            RemoteForward::Fixed(spec) => fixed_candidate(TunnelKind::Remote, spec),
            RemoteForward::Dynamic(_) => None,
        };
        record(alias, candidate, forward.ssh_g_form(), preview);
    }
    for forward in &config.dynamic_forwards {
        let text = forward.listen.ssh_g_form();
        record(alias, dynamic_candidate(forward), text, preview);
    }
}

fn record(
    alias: &str,
    candidate: Option<ImportCandidate>,
    text: String,
    preview: &mut ImportPreview,
) {
    match candidate {
        Some(candidate) => preview.candidates.push(ImportCandidate {
            host: Some(alias.to_owned()),
            ..candidate
        }),
        None => preview.warnings.push(ImportWarning::UnsupportedForward {
            line: None,
            host: Some(alias.to_owned()),
            text,
        }),
    }
}
