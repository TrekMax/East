use std::path::{Path, PathBuf};

use thiserror::Error;

/// Error resolving a manifest-relative path.
#[derive(Debug, Error)]
#[allow(clippy::module_name_repetitions)]
pub enum PathResolveError {
    /// The resolved path does not exist or cannot be canonicalized.
    #[error("failed to resolve path '{raw}' declared in {manifest}: {source}")]
    NotFound {
        /// The raw path string from the manifest.
        raw: String,
        /// The manifest file that declared this path.
        manifest: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
}

/// A path declared in a manifest file, resolved relative to that manifest's location.
///
/// If the raw path is absolute, it is used as-is. If relative, it is joined
/// onto the directory containing the declaring manifest file. The result is
/// always canonicalized before returning.
pub struct ManifestRelativePath {
    manifest_path: PathBuf,
    raw: String,
}

impl ManifestRelativePath {
    /// Create a new manifest-relative path.
    ///
    /// - `manifest_path`: the filesystem path to the manifest file that declared this path.
    /// - `raw`: the path string as written in the manifest.
    #[must_use]
    pub fn new(manifest_path: &Path, raw: &str) -> Self {
        Self {
            manifest_path: manifest_path.to_path_buf(),
            raw: raw.to_string(),
        }
    }

    /// Resolve the path to an absolute, canonicalized filesystem path.
    ///
    /// # Errors
    ///
    /// Returns [`PathResolveError::NotFound`] if the resolved path does not
    /// exist or cannot be canonicalized.
    pub fn resolve(&self) -> Result<PathBuf, PathResolveError> {
        let raw_path = Path::new(&self.raw);
        let joined = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            let manifest_dir = self
                .manifest_path
                .parent()
                .unwrap_or_else(|| Path::new("."));
            manifest_dir.join(raw_path)
        };

        std::fs::canonicalize(&joined).map_err(|source| PathResolveError::NotFound {
            raw: self.raw.clone(),
            manifest: self.manifest_path.clone(),
            source,
        })
    }
}
