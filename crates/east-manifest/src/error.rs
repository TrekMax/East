use thiserror::Error;

/// Errors that can occur when parsing or validating a manifest.
#[derive(Debug, Error)]
#[allow(clippy::module_name_repetitions)]
pub enum ManifestError {
    /// YAML deserialization failed.
    #[error("failed to parse manifest YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

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
}
