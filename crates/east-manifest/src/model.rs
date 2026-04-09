#![allow(clippy::doc_markdown)]

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::ManifestError;

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

impl Manifest {
    /// Parse a manifest from a YAML string and validate it.
    ///
    /// Validates:
    /// - Schema version is `1`.
    /// - No duplicate project names.
    /// - All remote references (in projects and defaults) exist.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] if parsing or validation fails.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, ManifestError> {
        let manifest: Self = serde_yaml::from_str(yaml)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest's internal consistency.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] on validation failure.
    pub fn validate(&self) -> Result<(), ManifestError> {
        // Version check
        if self.version != 1 {
            return Err(ManifestError::UnsupportedVersion {
                version: self.version,
            });
        }

        // Duplicate project names
        let mut seen_names = HashSet::new();
        for p in &self.projects {
            if !seen_names.insert(&p.name) {
                return Err(ManifestError::DuplicateProject {
                    name: p.name.clone(),
                });
            }
        }

        // Remote reference validation
        let remote_names: HashSet<&str> = self.remotes.iter().map(|r| r.name.as_str()).collect();

        if let Some(defaults) = &self.defaults {
            if let Some(ref default_remote) = defaults.remote {
                if !remote_names.contains(default_remote.as_str()) {
                    return Err(ManifestError::UnknownRemote {
                        name: default_remote.clone(),
                    });
                }
            }
        }

        for p in &self.projects {
            if let Some(ref remote) = p.remote {
                if !remote_names.contains(remote.as_str()) {
                    return Err(ManifestError::UnknownRemote {
                        name: remote.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Return projects that pass the group filter.
    ///
    /// If `group_filter` is empty, all projects are returned.
    /// Otherwise, a project is included if:
    /// - It has no groups (always included), OR
    /// - It belongs to at least one `+group` AND does not belong to any `-group`.
    #[must_use]
    pub fn filtered_projects(&self) -> Vec<&Project> {
        if self.group_filter.is_empty() {
            return self.projects.iter().collect();
        }

        let include: HashSet<&str> = self
            .group_filter
            .iter()
            .filter_map(|f| f.strip_prefix('+'))
            .collect();

        let exclude: HashSet<&str> = self
            .group_filter
            .iter()
            .filter_map(|f| f.strip_prefix('-'))
            .collect();

        self.projects
            .iter()
            .filter(|p| {
                if p.groups.is_empty() {
                    return true;
                }
                let dominated_by_exclude = p.groups.iter().any(|g| exclude.contains(g.as_str()));
                let matched_by_include = p.groups.iter().any(|g| include.contains(g.as_str()));
                matched_by_include && !dominated_by_exclude
            })
            .collect()
    }

    /// Determine the clone URL for a project.
    ///
    /// Resolves the remote (project-level, then defaults), looks it up,
    /// and constructs `{url_base}/{project_name}`.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::NoRemote`] if no remote can be resolved.
    pub fn project_clone_url(&self, project: &Project) -> Result<String, ManifestError> {
        let remote_name = project
            .remote
            .as_deref()
            .or_else(|| self.defaults.as_ref().and_then(|d| d.remote.as_deref()))
            .ok_or_else(|| ManifestError::NoRemote {
                project: project.name.clone(),
            })?;

        let remote = self
            .remotes
            .iter()
            .find(|r| r.name == remote_name)
            .ok_or_else(|| ManifestError::UnknownRemote {
                name: remote_name.to_string(),
            })?;

        Ok(format!("{}/{}", remote.url_base, project.name))
    }

    /// Determine the effective revision for a project.
    ///
    /// Returns the project's explicit revision if set, otherwise the default
    /// revision. Returns `None` if neither is set.
    #[must_use]
    pub fn project_revision<'a>(&'a self, project: &'a Project) -> Option<&'a str> {
        project
            .revision
            .as_deref()
            .or_else(|| self.defaults.as_ref().and_then(|d| d.revision.as_deref()))
    }
}
