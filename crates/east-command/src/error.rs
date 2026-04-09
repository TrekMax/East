use thiserror::Error;

/// Errors from the template engine.
#[derive(Debug, Error)]
#[allow(clippy::module_name_repetitions)]
pub enum TemplateError {
    /// A referenced key was not found in any namespace.
    #[error("missing key '{key}' in template (source: {source_hint})")]
    MissingKey {
        /// The key that was not found.
        key: String,
        /// Where the template came from (file path or description).
        source_hint: String,
    },

    /// A `${` was found without a matching `}`.
    #[error("unterminated variable in template (source: {source_hint})")]
    UnterminatedVariable {
        /// Where the template came from.
        source_hint: String,
    },
}

/// Errors from command operations.
#[derive(Debug, Error)]
#[allow(clippy::module_name_repetitions)]
pub enum CommandError {
    /// A template rendering error.
    #[error(transparent)]
    Template(#[from] TemplateError),

    /// Failed to spawn a command.
    #[error("failed to spawn command '{name}': {source}")]
    SpawnFailed {
        /// The command name.
        name: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Command not found.
    #[error("unknown command: {name}")]
    NotFound {
        /// The command name.
        name: String,
    },
}
