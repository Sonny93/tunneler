//! Turns pasted text or the `ssh_config` files into tunnel candidates.
//!
//! [`preview_script`] reads a script (PowerShell, shell, a list of ports or
//! bare `-L`/`-R`/`-D` specs) and [`preview_ssh_config`] reads the forwards
//! declared by the `Host` aliases. Both only produce a preview: nothing is
//! stored, the caller decides which candidates become tunnels.
//!
//! # Known limits
//!
//! - Script parsing is textual, not a shell or PowerShell interpreter:
//!   variables other than the placeholders of a port template are not
//!   evaluated, and `-o LocalForward=...` options are not read.
//! - A template (`-L ${port}:localhost:${port}`) is expanded with every port
//!   of the script's port list.

mod comments;
mod destination;
mod forward;
mod line_scan;
mod port_list;
mod script;
mod ssh_config_import;
mod template;
#[cfg(test)]
mod tests;
mod tokens;

pub use script::preview_script;
pub use ssh_config_import::preview_ssh_config;

use crate::store::TunnelKind;

/// The tunnels found in an import source, and what could not be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPreview {
    /// The forwards found, in source order, without duplicates.
    pub candidates: Vec<ImportCandidate>,
    /// What was skipped or could not be resolved.
    pub warnings: Vec<ImportWarning>,
}

/// One forward found in an import source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportCandidate {
    /// The `ssh_config` alias the forward belongs to; `None` when unknown.
    pub host: Option<String>,
    /// The direction of the forward.
    pub kind: TunnelKind,
    /// The bind address; `None` when none was written.
    pub listen_address: Option<String>,
    /// The port to listen on.
    pub listen_port: u16,
    /// The host the traffic is forwarded to; `None` for a dynamic forward or a bare port.
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to; `None` for a dynamic forward or a bare port.
    pub target_port: Option<u16>,
    /// A name taken from the comment next to the port.
    pub label: Option<String>,
    /// The 1-based line of the pasted text; `None` for an `ssh_config` import.
    pub line: Option<u32>,
}

/// Something that was skipped while reading an import source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportWarning {
    /// A forward specification that cannot be parsed, or a template without a port list.
    MalformedForward {
        /// The 1-based line of the specification.
        line: u32,
        /// The specification as written.
        text: String,
    },
    /// A forward that tunnels cannot represent: a Unix socket or a remote SOCKS proxy.
    UnsupportedForward {
        /// The 1-based line of the specification; `None` for an `ssh_config` import.
        line: Option<u32>,
        /// The `ssh_config` alias declaring the forward, when known.
        host: Option<String>,
        /// The specification as written.
        text: String,
    },
    /// An `ssh` destination that matches no `ssh_config` alias.
    UnknownDestination {
        /// The destination as written, for example `jules@198.51.100.157`.
        destination: String,
    },
    /// An alias whose configuration could not be resolved.
    HostResolutionFailed {
        /// The alias.
        host: String,
        /// Why the resolution failed.
        message: String,
    },
}
