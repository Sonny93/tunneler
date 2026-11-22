//! The wire types of the configuration transfer: `config.export`,
//! `config.import_preview`, `config.import_apply` and `import.classify`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::protocol::dto::{TunnelKindDto, TunnelSpecDto};

/// The largest import file the daemon accepts, in bytes: 256 KiB. Every
/// reader of an import file stops at one byte more.
pub const MAX_CONFIG_IMPORT_BYTES: usize = 262_144;

/// The most tunnel entries an import file may hold.
pub const MAX_CONFIG_IMPORT_TUNNELS: usize = 1000;

/// The most candidates `import.classify` accepts in one call.
pub const MAX_IMPORT_CLASSIFY_CANDIDATES: usize = 1000;

/// The parameters of `config.export`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigExportParams {
    /// Export the tunnels of these hosts; a tunnel matching any filter is
    /// exported, and no filter at all exports every saved tunnel.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<String>,
    /// Export the tunnels carrying one of these tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// The result of `config.export`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigExportResult {
    /// The export file, as TOML text.
    pub contents: String,
    /// How many tunnels it holds.
    pub tunnel_count: u64,
}

/// The parameters of `config.import_preview`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigImportParams {
    /// The import file, at most [`MAX_CONFIG_IMPORT_BYTES`] bytes.
    pub contents: String,
    /// Replaces hosts of the file by local aliases, `{ fileHost: localAlias }`,
    /// before the entries are checked; keys naming no host of the file are
    /// ignored.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub host_map: BTreeMap<String, String>,
}

/// The parameters of `config.import_apply`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigImportApplyParams {
    /// The import file that was previewed.
    pub contents: String,
    /// The host map that was previewed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub host_map: BTreeMap<String, String>,
    /// The 0-based entries to add; risky entries are added when listed here.
    pub indexes: Vec<u32>,
}

/// Where the entries of an import file come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigImportSourceDto {
    /// A Tunneler export file.
    Export,
    /// A raw `tunnels.toml`.
    Store,
}

/// What an import would do with one entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ConfigImportStatusDto {
    /// The entry would be added.
    Importable,
    /// A saved tunnel already has this route; nothing is added.
    AlreadyConfigured {
        /// The saved tunnel.
        existing_id: String,
        /// The tags of the entry that the saved tunnel lacks and that are
        /// not applied to it.
        ignored_tags: Vec<String>,
    },
    /// A saved tunnel holds the same local address and port, or the same
    /// remote address and port on the same host.
    ListenConflict {
        /// The saved tunnel.
        holder_id: String,
    },
    /// An earlier entry of the file has this route.
    DuplicateInFile {
        /// The 0-based index of the earlier entry.
        earlier_index: u32,
    },
    /// The entry breaks a rule of the store.
    Invalid {
        /// What is wrong, in English.
        messages: Vec<String>,
    },
}

/// Why an entry is not selected by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ConfigImportRiskDto {
    /// A local or dynamic tunnel listens on an address that is not loopback.
    NonLoopbackListen {
        /// The effective listen address.
        address: String,
    },
    /// A remote tunnel forwards to a host that is not loopback.
    RemoteNonLoopbackTarget {
        /// The effective target host.
        target_host: String,
    },
    /// A tag of the entry starts tunnels when the daemon launches.
    StartsOnLaunch {
        /// The tags of the entry that are launch tags on this machine.
        tags: Vec<String>,
    },
}

/// Something worth knowing about an entry that does not block it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ConfigImportEntryWarningDto {
    /// No `Host` entry of the `ssh_config` names the host exactly; a
    /// wildcard block may still match it.
    UnknownHost {
        /// The host.
        host: String,
    },
    /// The entry names an opener this machine does not have; it was dropped.
    OpenerDropped {
        /// The opener of the file.
        opener: String,
    },
}

/// One tunnel entry of an import file, planned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigImportEntryDto {
    /// The 0-based position in the tunnel list of the file.
    pub index: u32,
    /// What would be created, after the host map and the dropped opener.
    pub tunnel: TunnelSpecDto,
    /// The host as written in the file, before the host map.
    pub file_host: String,
    /// The address the tunnel would listen on, on this machine.
    pub effective_listen_address: String,
    /// What the import would do with the entry.
    pub status: ConfigImportStatusDto,
    /// Why the entry is not selected by default; empty for a safe entry.
    pub risks: Vec<ConfigImportRiskDto>,
    /// What is worth knowing about the entry.
    pub warnings: Vec<ConfigImportEntryWarningDto>,
    /// Whether the entry is importable and has no risk.
    pub selected_by_default: bool,
}

/// The result of `config.import_preview`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigImportPlanDto {
    /// What kind of file it is.
    pub source: ConfigImportSourceDto,
    /// The entries, in file order.
    pub entries: Vec<ConfigImportEntryDto>,
    /// The distinct hosts of the file, before the map, that no `Host` entry
    /// names exactly and that the map does not replace; sorted.
    pub unknown_hosts: Vec<String>,
}

/// A tunnel added by `config.import_apply`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigImportAddedDto {
    /// The 0-based index of the entry.
    pub index: u32,
    /// The identifier of the new tunnel.
    pub id: String,
}

/// A requested entry that `config.import_apply` did not add.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigImportSkippedDto {
    /// The 0-based index of the entry.
    pub index: u32,
    /// Why, as of the moment of the apply.
    pub status: ConfigImportStatusDto,
}

/// The result of `config.import_apply`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigImportReportDto {
    /// The tunnels added.
    pub added: Vec<ConfigImportAddedDto>,
    /// The requested entries that were not added.
    pub skipped: Vec<ConfigImportSkippedDto>,
}

/// One route of an import source, host included, to check against the saved
/// tunnels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRouteDto {
    /// The `ssh_config` alias the tunnel would go through.
    pub host: String,
    /// The direction of the forward.
    pub kind: TunnelKindDto,
    /// The port to listen on.
    pub listen_port: u16,
    /// The bind address, when one was written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<String>,
    /// The host the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_host: Option<String>,
    /// The port the traffic is forwarded to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_port: Option<u16>,
}

/// The parameters of `import.classify`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportClassifyParams {
    /// The routes to check, at most [`MAX_IMPORT_CLASSIFY_CANDIDATES`].
    pub candidates: Vec<ImportRouteDto>,
}

/// Why an import route cannot be added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ImportConflictDto {
    /// A saved tunnel already has this route.
    AlreadyConfigured,
    /// A saved tunnel holds the same local address and port, or the same
    /// remote address and port on the same host.
    ListenConflict {
        /// The saved tunnel.
        holder_id: String,
    },
    /// An earlier route of the same call has this route.
    DuplicateInImport,
}

/// The result of `import.classify`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportClassifyResult {
    /// One per candidate, in the same order; `None` when the route can be
    /// added.
    pub conflicts: Vec<Option<ImportConflictDto>>,
}

/// Why an import file was refused as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InvalidImportReasonDto {
    /// The text is not valid TOML, or has the wrong shape.
    Syntax,
    /// The `format` key names something other than a Tunneler export.
    NotTunnelerFile,
    /// The file was written by a newer version of Tunneler.
    NewerVersion,
    /// A value holds a control or bidirectional formatting character.
    UnsafeCharacters,
    /// The contents exceed [`MAX_CONFIG_IMPORT_BYTES`].
    TooLarge,
    /// The file holds more than [`MAX_CONFIG_IMPORT_TUNNELS`] tunnels.
    TooManyEntries,
}

/// The `data` of an `invalid_import_file` error; only the keys that apply to
/// the reason are present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidImportFileDto {
    /// Why the file was refused.
    pub reason: InvalidImportReasonDto,
    /// The 1-based line of a syntax error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// What the parser said about a syntax error; never a value of the file
    /// that holds unsafe characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The 0-based index of the entry that holds unsafe characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<u32>,
    /// The name of the field that holds unsafe characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// The version found in a file that is too new.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub found: Option<u32>,
    /// The newest version this Tunneler reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supported: Option<u32>,
    /// The cap that was exceeded: bytes or entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

impl InvalidImportFileDto {
    /// Details for a reason that needs none of the optional keys.
    pub const fn new(reason: InvalidImportReasonDto) -> Self {
        Self {
            reason,
            line: None,
            message: None,
            entry: None,
            field: None,
            found: None,
            supported: None,
            limit: None,
        }
    }
}
