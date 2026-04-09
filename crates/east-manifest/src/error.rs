use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur when parsing or validating a manifest.
#[derive(Debug, Error)]
#[allow(clippy::module_name_repetitions)]
pub enum ManifestError {
    /// YAML deserialization failed.
    #[error("failed to parse manifest YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Filesystem I/O error while reading a manifest file.
    #[error("failed to read manifest file {path}: {source}")]
    Io {
        /// The file path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// The manifest declares an unsupported schema version.
    #[error("unsupported manifest version {version} (expected 1)")]
    UnsupportedVersion {
        /// The version found in the manifest.
        version: u32,
    },

    /// Two projects share the same name.
    #[error("duplicate project name: {name}")]
    DuplicateProject {
        /// The duplicated project name.
        name: String,
    },

    /// A project or defaults block references a remote that is not declared.
    #[error("unknown remote: {name}")]
    UnknownRemote {
        /// The remote name that was not found.
        name: String,
    },

    /// No remote could be determined for a project (neither explicit nor default).
    #[error("no remote configured for project: {project}")]
    NoRemote {
        /// The project name missing a remote.
        project: String,
    },

    /// An import cycle was detected during manifest resolution.
    #[error("import cycle detected: {path}")]
    ImportCycle {
        /// The path that was already visited.
        path: PathBuf,
    },

    /// A command name is invalid (must match `[a-z][a-z0-9-]*`).
    #[error("invalid command name: {name} (must match [a-z][a-z0-9-]*)")]
    InvalidCommandName {
        /// The invalid name.
        name: String,
    },

    /// A command must have exactly one of `exec`, `executable`, or `script`.
    #[error("command '{name}': exactly one of exec, executable, or script must be set")]
    CommandMutuallyExclusive {
        /// The command name.
        name: String,
    },

    /// A command uses a reserved or builtin name.
    #[error("command '{name}' uses a reserved name")]
    ReservedCommandName {
        /// The reserved name.
        name: String,
    },
}
