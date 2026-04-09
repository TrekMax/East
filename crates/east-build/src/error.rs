use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

/// Errors from build operations.
#[derive(Debug, Error, Diagnostic)]
#[allow(clippy::module_name_repetitions)]
pub enum BuildError {
    /// `CMake` executable not found on PATH.
    #[error("cmake not found. Install CMake >= 3.21 and ensure it is on PATH.")]
    CmakeNotFound,

    /// `CMake` version is too low.
    #[error("cmake version {found} is too low (requires >= {required})")]
    CmakeVersionTooLow {
        /// The version found.
        found: String,
        /// The minimum required version.
        required: String,
    },

    /// Source directory does not exist.
    #[error("source directory not found: {path}")]
    SourceDirNotFound {
        /// The path that was expected.
        path: PathBuf,
    },

    /// `CMake` configure step failed.
    #[error("cmake configure failed:\n{stderr_tail}")]
    ConfigureFailed {
        /// Last lines of stderr.
        stderr_tail: String,
    },

    /// `CMake` build step failed.
    #[error("cmake build failed:\n{stderr_tail}")]
    BuildFailed {
        /// Last lines of stderr.
        stderr_tail: String,
    },

    /// No build artifacts found (warning-level, but can be surfaced as error).
    #[error("no build artifacts (*.elf, *.bin, *.hex) found in {build_dir}")]
    NoArtifactsFound {
        /// The build directory searched.
        build_dir: PathBuf,
    },

    /// I/O error.
    #[error("build I/O error: {0}")]
    Io(#[from] std::io::Error),
}
