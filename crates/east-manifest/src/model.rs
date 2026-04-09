#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};

/// A named remote repository base URL.
///
/// Combined with a project name to form the full clone URL:
/// `{url_base}/{project_name}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remote {
    /// Unique identifier for this remote (e.g. `"origin"`).
    pub name: String,
    /// Base URL for repositories under this remote (e.g. `"https://github.com/org"`).
    #[serde(rename = "url-base")]
    pub url_base: String,
}

/// Default values applied to projects that do not specify their own.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Defaults {
    /// Default remote name. Projects without an explicit `remote` inherit this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// Default revision. Projects without an explicit `revision` inherit this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

/// A project tracked by the manifest.
///
/// Each project maps to one git repository that will be cloned into
/// the workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Unique project name (also used to construct the clone URL).
    pub name: String,
    /// Filesystem path relative to the workspace root. Defaults to `name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Remote name for this project. Falls back to `defaults.remote`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// Git revision (branch, tag, or SHA). Falls back to `defaults.revision`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Group memberships for filtering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
}

impl Project {
    /// Returns the effective filesystem path: the explicit `path` if set,
    /// otherwise the project `name`.
    #[must_use]
    pub fn effective_path(&self) -> &str {
        self.path.as_deref().unwrap_or(&self.name)
    }
}

/// An import directive referencing another manifest file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Import {
    /// Path to the imported manifest, relative to the directory of the
    /// manifest that declares this import.
    pub file: String,
    /// Optional glob patterns to filter which projects are imported.
    /// An empty list means import all projects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowlist: Vec<String>,
}

/// Top-level east manifest (`east.yml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Schema version. Must be `1` for the current format.
    pub version: u32,
    /// Named remotes providing base URLs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remotes: Vec<Remote>,
    /// Default values for project fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Defaults>,
    /// Projects to manage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<Project>,
    /// Manifest files to import transitively.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<Import>,
    /// Group filter expressions (e.g. `["+required", "-optional"]`).
    #[serde(
        default,
        rename = "group-filter",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub group_filter: Vec<String>,
}
