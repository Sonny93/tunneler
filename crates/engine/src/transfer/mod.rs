//! Moving the tunnels between machines: the export of the saved tunnels to a
//! file, and the import of such a file, or of a `tunnels.toml`, as new
//! tunnels.
//!
//! The import is strictly additive and treats every file as foreign: the size
//! and the number of tunnels are capped, a control or bidirectional control
//! character in any text field refuses the whole file, and every entry goes
//! through the host alias rule and the validation of the store. Entries are
//! planned first ([`ConfigImportPlan`]): each one is importable or says why it
//! is not, and the risky ones are not selected by default. Applying re-plans
//! the same file against the store as it is at that moment.
//!
//! The same classification of routes against existing tunnels serves every
//! import source, see [`classify_routes`].

mod classify;
mod error;
mod export;
mod parse;
mod plan;
mod plan_types;
mod prepare;
mod risk;
mod selection;

pub use classify::{ImportRoute, RouteConflict, classify_routes};
pub use error::ImportFileError;
pub use export::{ConfigExport, ConfigExportFilter};
pub use plan_types::{
    ConfigImportAdded, ConfigImportEntry, ConfigImportEntryWarning, ConfigImportPlan,
    ConfigImportReport, ConfigImportRisk, ConfigImportSkipped, ConfigImportSource,
    ConfigImportStatus,
};

pub(crate) use export::export_tunnels;
pub(crate) use parse::{ParsedImportFile, parse_import_file};
pub(crate) use plan::{PlanContext, plan_parsed};
pub(crate) use selection::select_importable;

/// The most bytes of a file the import reads; a larger one is refused before
/// it is parsed. Every reader of an import file applies the same cap.
pub const MAX_IMPORT_FILE_BYTES: usize = 262_144;

/// The most tunnels an import file may hold.
pub const MAX_IMPORT_TUNNELS: usize = 1000;

pub(crate) const EXPORT_FORMAT: &str = "tunneler-export";
pub(crate) const EXPORT_VERSION: u32 = 1;
