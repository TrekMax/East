use std::fs;
use std::path::{Path, PathBuf};

use east_config::ConfigStore;
use east_config::manifest_config::ManifestConfig;

use crate::error::WorkspaceError;

/// The east marker directory name.
const EAST_DIR: &str = ".east";
/// Config file within `.east/`.
const CONFIG_FILE: &str = "config.toml";
/// Legacy manifest file name (Phase 1/2 compatibility).
const LEGACY_MANIFEST_FILE: &str = "east.yml";

/// Represents a discovered or initialized east workspace.
///
/// A workspace is rooted at the directory that contains `.east/`.
/// The manifest location is determined by the `[manifest]` section
/// in `.east/config.toml`.
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
    manifest_repo_path: Option<PathBuf>,
    manifest_file_path: Option<PathBuf>,
}

impl Workspace {
    /// Discover a workspace by walking upward from `start` looking for `.east/`.
    ///
    /// After finding `.east/`, loads `.east/config.toml` to determine
    /// the manifest repository location.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError::NotFound`] if no `.east/` directory is found.
    pub fn discover(start: &Path) -> Result<Self, WorkspaceError> {
        let mut current = fs::canonicalize(start)?;

        loop {
            if current.join(EAST_DIR).is_dir() {
                let root = current;
                let (repo_path, file_path) = Self::load_manifest_paths(&root);
                // If config didn't provide paths, fall back to legacy layout
                let file_path = file_path.unwrap_or_else(|| root.join(LEGACY_MANIFEST_FILE));
                return Ok(Self {
                    root,
                    manifest_repo_path: repo_path,
                    manifest_file_path: Some(file_path),
                });
            }
            if !current.pop() {
                break;
            }
        }

        Err(WorkspaceError::NotFound {
            start: start.to_path_buf(),
        })
    }

    /// Initialize a new workspace at `root` by creating the `.east/` directory.
    ///
    /// Idempotent: succeeds if `.east/` already exists.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError::Io`] if directory creation fails.
    pub fn init(root: &Path) -> Result<Self, WorkspaceError> {
        let east_dir = root.join(EAST_DIR);
        fs::create_dir_all(&east_dir)?;
        let canonical_root = fs::canonicalize(root)?;
        Ok(Self {
            root: canonical_root,
            manifest_repo_path: None,
            manifest_file_path: None,
        })
    }

    /// The workspace root directory (parent of `.east/`).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to the `.east/` directory.
    #[must_use]
    pub fn east_dir(&self) -> PathBuf {
        self.root.join(EAST_DIR)
    }

    /// Path to the manifest repository directory.
    ///
    /// Computed from `config.manifest.path`. Falls back to workspace root
    /// if config is not yet loaded (e.g. during init before config is written).
    #[must_use]
    pub fn manifest_repo_path(&self) -> &Path {
        self.manifest_repo_path.as_deref().unwrap_or(&self.root)
    }

    /// Path to the manifest file.
    ///
    /// Computed from `config.manifest.path` + `config.manifest.file`.
    /// Falls back to `<root>/east.yml` for legacy compatibility.
    #[must_use]
    pub fn manifest_file_path(&self) -> &Path {
        self.manifest_file_path.as_deref().unwrap_or(&self.root)
    }

    /// Legacy compatibility: path to `<root>/east.yml`.
    ///
    /// Deprecated in Phase 2.6. Use `manifest_file_path()` instead.
    #[must_use]
    pub fn manifest_path(&self) -> PathBuf {
        self.manifest_file_path
            .as_ref()
            .map_or_else(|| self.root.join(LEGACY_MANIFEST_FILE), Clone::clone)
    }

    /// Load manifest paths from `.east/config.toml`.
    ///
    /// Returns `(manifest_repo_path, manifest_file_path)` or `(None, None)`
    /// if config doesn't exist or lacks a `[manifest]` section.
    fn load_manifest_paths(root: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
        let config_path = root.join(EAST_DIR).join(CONFIG_FILE);
        let Ok(store) = ConfigStore::load_from_file(&config_path) else {
            return (None, None);
        };

        ManifestConfig::from_store(&store).map_or((None, None), |mc| {
            let repo = root.join(mc.path());
            let file = repo.join(mc.file());
            (Some(repo), Some(file))
        })
    }
}
