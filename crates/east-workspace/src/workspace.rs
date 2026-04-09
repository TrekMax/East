use std::fs;
use std::path::{Path, PathBuf};

use crate::error::WorkspaceError;

/// The east marker directory name.
const EAST_DIR: &str = ".east";
/// The default manifest file name.
const MANIFEST_FILE: &str = "east.yml";

/// Represents a discovered or initialized east workspace.
///
/// A workspace is rooted at the directory that contains `.east/`.
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Discover a workspace by walking upward from `start` looking for `.east/`.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError::NotFound`] if no `.east/` directory is found
    /// before reaching the filesystem root.
    pub fn discover(start: &Path) -> Result<Self, WorkspaceError> {
        let mut current = fs::canonicalize(start)?;

        loop {
            if current.join(EAST_DIR).is_dir() {
                return Ok(Self { root: current });
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

    /// Path to the top-level manifest file (`east.yml`).
    #[must_use]
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }
}
