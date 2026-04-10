use std::path::Path;

use crate::error::ConfigError;
use crate::store::ConfigStore;
use crate::value::ConfigValue;

/// Configuration for the manifest repository location within a workspace.
///
/// Corresponds to the `[manifest]` section in `.east/config.toml`.
#[derive(Debug, Clone)]
pub struct ManifestConfig {
    /// Workspace-relative path to the manifest repository directory.
    path: String,
    /// Manifest file name within the manifest repo (default: `east.yml`).
    file: String,
}

impl ManifestConfig {
    /// Create a new `ManifestConfig` with explicit values.
    #[must_use]
    pub fn new(path: &str, file: &str) -> Self {
        Self {
            path: path.to_string(),
            file: file.to_string(),
        }
    }

    /// Extract `ManifestConfig` from a `ConfigStore`.
    ///
    /// Reads `manifest.path` (required) and `manifest.file` (defaults to `east.yml`).
    ///
    /// # Errors
    ///
    /// - [`ConfigError::ManifestSectionMissing`] if `manifest.path` is absent.
    /// - [`ConfigError::InvalidManifestPath`] if the path is absolute, empty, or contains `..`.
    pub fn from_store(store: &ConfigStore) -> Result<Self, ConfigError> {
        let path = store
            .get("manifest.path")
            .and_then(ConfigValue::as_str)
            .ok_or(ConfigError::ManifestSectionMissing)?
            .to_string();

        validate_manifest_path(&path)?;

        let file = store
            .get("manifest.file")
            .and_then(ConfigValue::as_str)
            .unwrap_or("east.yml")
            .to_string();

        Ok(Self { path, file })
    }

    /// Write this config to a `ConfigStore`.
    pub fn write_to_store(&self, store: &mut ConfigStore) {
        store.set("manifest.path", ConfigValue::String(self.path.clone()));
        store.set("manifest.file", ConfigValue::String(self.file.clone()));
    }

    /// The workspace-relative path to the manifest repository.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The manifest file name.
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }
}

/// Validate that a manifest path is relative, non-empty, and contains no `..`.
fn validate_manifest_path(path: &str) -> Result<(), ConfigError> {
    if path.is_empty() {
        return Err(ConfigError::InvalidManifestPath {
            path: path.to_string(),
            reason: "path must not be empty".to_string(),
        });
    }

    let p = Path::new(path);
    if p.is_absolute() {
        return Err(ConfigError::InvalidManifestPath {
            path: path.to_string(),
            reason: "path must be relative, not absolute".to_string(),
        });
    }

    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ConfigError::InvalidManifestPath {
            path: path.to_string(),
            reason: "path must not contain '..' components".to_string(),
        });
    }

    Ok(())
}
