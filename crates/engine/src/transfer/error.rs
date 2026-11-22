use thiserror::Error;

/// Why a file cannot be imported at all; each variant is one `reason` of the
/// IPC error. The text of a refused field is never carried, only where it is.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImportFileError {
    /// The file is not valid TOML, or does not match the layout.
    #[error("{}", describe_syntax(*line, message))]
    Syntax {
        /// The 1-based line of the problem, when known.
        line: Option<usize>,
        /// What is wrong, stripped of control characters.
        message: String,
    },
    /// The file is neither a Tunneler export nor a tunnels file: its `format`
    /// key holds another value.
    #[error("the file is not a Tunneler export or a tunnels file")]
    NotTunnelerFile,
    /// The file was written by a newer version of Tunneler.
    #[error(
        "the file uses format version {found} but this version of Tunneler only supports up to version {supported}"
    )]
    NewerVersion {
        /// The version found in the file.
        found: u32,
        /// The newest version this build understands.
        supported: u32,
    },
    /// A text field holds a control or bidirectional control character.
    #[error(
        "tunnel {} of the file: the field {field} holds a control or bidirectional control character",
        entry + 1
    )]
    UnsafeCharacters {
        /// The 0-based position of the tunnel in the file.
        entry: usize,
        /// The field name as written in the file.
        field: &'static str,
    },
    /// The file is larger than the cap.
    #[error("the file is {found} bytes, over the limit of {limit} bytes")]
    TooLarge {
        /// The size of the file in bytes.
        found: usize,
        /// The cap in bytes.
        limit: usize,
    },
    /// The file holds more tunnels than the cap.
    #[error("the file holds {found} tunnels, over the limit of {limit}")]
    TooManyEntries {
        /// The number of tunnels in the file.
        found: usize,
        /// The cap.
        limit: usize,
    },
}

fn describe_syntax(line: Option<usize>, message: &str) -> String {
    match line {
        Some(line) => format!("the file is not valid, line {line}: {message}"),
        None => format!("the file is not valid: {message}"),
    }
}
