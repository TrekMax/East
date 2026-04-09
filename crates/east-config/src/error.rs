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
}
