#![allow(clippy::doc_markdown)]

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use glob_match::glob_match;

use crate::error::ManifestError;
use crate::model::Manifest;

/// Resolve a manifest file, recursively processing imports.
///
/// Returns a single flattened `Manifest` with all transitive projects merged.
/// The first definition of a project (by name) wins; later duplicates from
/// imports are silently skipped.
///
/// # Errors
///
/// Returns [`ManifestError`] if:
/// - Any manifest file cannot be read or parsed.
/// - An import cycle is detected.
pub fn resolve(path: impl AsRef<Path>) -> Result<Manifest, ManifestError> {
    let mut visited = HashSet::new();
    let mut seen_project_names: HashSet<String> = HashSet::new();
    let mut all_projects = Vec::new();

    // BFS-like queue: (manifest_file_path,)
    let mut queue: VecDeque<(PathBuf, Vec<String>)> = VecDeque::new();
    let canonical = canonicalize_path(path.as_ref())?;
    queue.push_back((canonical, Vec::new()));

    // We'll keep the top-level manifest to use as base for the result
    let mut top_manifest: Option<Manifest> = None;

    while let Some((manifest_path, parent_allowlist)) = queue.pop_front() {
        // Cycle detection
        if !visited.insert(manifest_path.clone()) {
            return Err(ManifestError::ImportCycle {
                path: manifest_path,
            });
        }

        let content = fs::read_to_string(&manifest_path).map_err(|e| ManifestError::Io {
            path: manifest_path.clone(),
            source: e,
        })?;
        let manifest = Manifest::from_yaml_str(&content)?;

        let manifest_dir = manifest_path
            .parent()
            .expect("manifest path should have a parent directory");

        // Filter projects by parent's allowlist (empty = accept all)
        for project in &manifest.projects {
            let passes_allowlist = parent_allowlist.is_empty()
                || parent_allowlist
                    .iter()
                    .any(|pattern| glob_match(pattern, &project.name));

            if passes_allowlist && seen_project_names.insert(project.name.clone()) {
                all_projects.push(project.clone());
            }
        }

        // Queue child imports
        for import in &manifest.imports {
            let import_path = manifest_dir.join(&import.file);
            let import_canonical = canonicalize_path(&import_path)?;
            queue.push_back((import_canonical, import.allowlist.clone()));
        }

        if top_manifest.is_none() {
            top_manifest = Some(manifest);
        }
    }

    let mut result = top_manifest.unwrap_or_else(|| Manifest {
        version: 1,
        remotes: Vec::new(),
        defaults: None,
        projects: Vec::new(),
        imports: Vec::new(),
        group_filter: Vec::new(),
    });
    result.projects = all_projects;

    Ok(result)
}

/// Canonicalize a path, producing a helpful error on failure.
fn canonicalize_path(path: &Path) -> Result<PathBuf, ManifestError> {
    fs::canonicalize(path).map_err(|e| ManifestError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}
