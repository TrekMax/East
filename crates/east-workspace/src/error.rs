use std::path::PathBuf;

use thiserror::Error;

/// Errors from workspace operations.
#[derive(Debug, Error)]
#[allow(clippy::module_name_repetitions)]
pub enum WorkspaceError {
    /// No `.east/` directory found when walking up from the starting path.
    #[error("no east workspace found (searched upward from {start})")]
    NotFound {
        /// The directory where the search started.
        start: PathBuf,
    },

    /// Filesystem I/O error.
    #[error("workspace I/O error: {0}")]
    Io(#[from] std::io::Error),
}
