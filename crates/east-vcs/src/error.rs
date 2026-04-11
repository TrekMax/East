use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

/// Errors from git shell-out operations.
#[derive(Debug, Error, Diagnostic)]
#[allow(clippy::module_name_repetitions)]
pub enum VcsError {
    /// A git command failed with a non-zero exit code.
    #[error("git command failed in {path}")]
    #[diagnostic(help("check that the repository exists and the revision is valid"))]
    GitFailed {
        /// Working directory or target path.
        path: PathBuf,
        /// The stderr output from git.
        #[source_code]
        stderr: String,
    },

    /// Failed to spawn the git process.
    #[error("failed to execute git: {0}")]
    #[diagnostic(help("ensure git is installed and available on PATH"))]
    Io(#[from] std::io::Error),
}
