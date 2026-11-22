use crate::runtime::NewTunnel;
use crate::store::TunnelId;

/// Which of the accepted layouts a file was read as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigImportSource {
    /// A configuration export (`format = "tunneler-export"`).
    Export,
    /// A raw `tunnels.toml`, of which only the `[[tunnels]]` are read.
    Store,
}

/// What an import of a file would do, entry by entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigImportPlan {
    /// The layout the file was read as.
    pub source: ConfigImportSource,
    /// One entry per tunnel of the file, in file order.
    pub entries: Vec<ConfigImportEntry>,
    /// The distinct hosts of the file, as written, that no `Host` entry names
    /// exactly and that the host map does not rename, sorted.
    pub unknown_hosts: Vec<String>,
}

/// One tunnel of an import file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigImportEntry {
    /// The 0-based position of the tunnel in the file.
    pub index: usize,
    /// What would be created, after the host map and the removal of an
    /// unknown opener. For an invalid entry it still carries the raw values.
    pub tunnel: NewTunnel,
    /// The host as written in the file, before the host map.
    pub file_host: String,
    /// The address the tunnel would listen on, the default of this machine
    /// when the file gives none.
    pub effective_listen_address: String,
    /// Whether the entry can be imported, and if not why.
    pub status: ConfigImportStatus,
    /// What makes the entry worth a second look; empty for an entry that is
    /// not importable.
    pub risks: Vec<ConfigImportRisk>,
    /// Things the import changed or noticed.
    pub warnings: Vec<ConfigImportEntryWarning>,
    /// Whether the entry is importable and has no risk.
    pub selected_by_default: bool,
}

/// What an import would do with an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigImportStatus {
    /// The entry would be added.
    Importable,
    /// An existing tunnel has the same host, kind, listen address and port
    /// and target.
    AlreadyConfigured {
        /// The existing tunnel.
        existing_id: TunnelId,
        /// The tags of the file the existing tunnel lacks; they are not
        /// applied.
        ignored_tags: Vec<String>,
    },
    /// An existing tunnel holds the same listen address and port.
    ListenConflict {
        /// The tunnel that holds it.
        holder_id: TunnelId,
    },
    /// An earlier importable entry of the file holds the same listen address
    /// and port.
    DuplicateInFile {
        /// The 0-based position of the earlier entry.
        earlier_index: usize,
    },
    /// The entry breaks a rule.
    Invalid {
        /// What is wrong, in English.
        messages: Vec<String>,
    },
}

/// Why an importable entry is not selected by default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigImportRisk {
    /// The tunnel would listen on an address that is not loopback, so other
    /// machines could reach it. For a local or dynamic tunnel the address is
    /// on this machine; for a remote tunnel it is on the server, where
    /// `GatewayPorts` decides whether it is honored.
    NonLoopbackListen {
        /// The effective listen address.
        address: String,
    },
    /// A remote tunnel would forward to a target that is not loopback.
    RemoteNonLoopbackTarget {
        /// The effective target host.
        target_host: String,
    },
    /// A tag of the entry starts tunnels when the daemon launches.
    StartsOnLaunch {
        /// The tags of the entry found in `start_on_launch_tags`.
        tags: Vec<String>,
    },
}

/// Something the import changed or noticed about an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigImportEntryWarning {
    /// No `Host` entry of the `ssh_config` names the host exactly; a wildcard
    /// block may still match it.
    UnknownHost {
        /// The host, after the host map.
        host: String,
    },
    /// The opener the entry names exists neither among the openers of the
    /// user nor among the detected ones, so `open.with` was dropped.
    OpenerDropped {
        /// The opener the file names.
        opener: String,
    },
}

/// What applying an import did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigImportReport {
    /// The tunnels created, in file order.
    pub added: Vec<ConfigImportAdded>,
    /// The requested entries that were not added, with their status now.
    pub skipped: Vec<ConfigImportSkipped>,
}

/// A tunnel created by an import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigImportAdded {
    /// The 0-based position of the tunnel in the file.
    pub index: usize,
    /// The id of the new tunnel.
    pub id: TunnelId,
}

/// A requested entry that an import did not add.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigImportSkipped {
    /// The 0-based position of the tunnel in the file.
    pub index: usize,
    /// Why it was not added.
    pub status: ConfigImportStatus,
}
