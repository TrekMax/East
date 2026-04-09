use std::path::PathBuf;

use thiserror::Error;

/// Errors from git shell-out operations.
#[derive(Debug, Error)]
#[allow(clippy::module_name_repetitions)]
pub enum VcsError {
    /// A git command failed with a non-zero exit code.
    #[error("git command failed in {path}: {stderr}")]
    GitFailed {
        /// Working directory or target path.
        path: PathBuf,
        /// The stderr output from git.
        stderr: String,
    },

    /// Failed to spawn the git process.
    #[error("failed to execute git: {0}")]
    Io(#[from] std::io::Error),
}
