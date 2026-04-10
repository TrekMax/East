use miette::Diagnostic;
use thiserror::Error;

/// Errors from configuration operations.
#[derive(Debug, Error, Diagnostic)]
#[allow(clippy::module_name_repetitions)]
pub enum ConfigError {
    /// Filesystem I/O error.
    #[error("config I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parsing error.
    #[error("failed to parse TOML: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization error.
    #[error("failed to serialize TOML: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// Workspace config exists but has no `[manifest]` section.
    /// This indicates a workspace created by an older east version.
    #[error(
        "This workspace was created by an older east version and is not compatible. \
         Please re-initialize: remove `.east/`, then run `east init -l <path>` or `east init -m <url>`."
    )]
    ManifestSectionMissing,

    /// `manifest.path` value is invalid.
    #[error("invalid manifest.path '{path}': {reason}")]
    InvalidManifestPath {
        /// The invalid path value.
        path: String,
        /// Why it's invalid.
        reason: String,
    },
}
