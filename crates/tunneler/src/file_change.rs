//! What an install did to a file, for the reports of the commands.

use std::path::PathBuf;

/// What an install did to one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "only the platforms with a menu entry construct the variants"
)]
pub(crate) enum FileChange {
    /// The file was missing or different, and now has the wanted content.
    Written,
    /// The file already had the wanted content and was left alone.
    Unchanged,
}

/// A file and what was done to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChangedFile {
    pub(crate) path: PathBuf,
    pub(crate) change: FileChange,
}
