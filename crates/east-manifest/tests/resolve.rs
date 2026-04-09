//! Integration tests for manifest import resolution and cycle detection.

use std::fs;

use east_manifest::Manifest;
use tempfile::TempDir;

/// Helper: write a YAML string to a file inside the temp dir.
fn write_manifest(dir: &TempDir, relative_path: &str, content: &str) {
    let path = dir.path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, content).unwrap();
}

#[test]
fn resolve_single_manifest_no_imports() {
    let dir = TempDir::new().unwrap();
    write_manifest(
        &dir,
        "east.yml",
        r"
version: 1
projects:
  - name: core
  - name: drivers
",
    );

    let resolved = Manifest::resolve(dir.path().join("east.yml")).unwrap();
    assert_eq!(resolved.projects.len(), 2);
    assert_eq!(resolved.projects[0].name, "core");
    assert_eq!(resolved.projects[1].name, "drivers");
}

#[test]
fn resolve_one_level_import() {
    let dir = TempDir::new().unwrap();
    write_manifest(
        &dir,
        "east.yml",
        r"
version: 1
projects:
  - name: top-proj
imports:
  - file: sub/east.yml
",
    );
    write_manifest(
        &dir,
        "sub/east.yml",
        r"
version: 1
projects:
  - name: sub-proj-a
  - name: sub-proj-b
",
    );

    let resolved = Manifest::resolve(dir.path().join("east.yml")).unwrap();
    let names: Vec<&str> = resolved.projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["top-proj", "sub-proj-a", "sub-proj-b"]);
}

#[test]
fn resolve_two_level_import() {
    let dir = TempDir::new().unwrap();
    write_manifest(
        &dir,
        "east.yml",
        r"
version: 1
projects:
  - name: root
imports:
  - file: level1/east.yml
",
    );
    write_manifest(
        &dir,
        "level1/east.yml",
        r"
version: 1
projects:
  - name: mid
imports:
  - file: level2/east.yml
",
    );
    // Note: level2 path is relative to level1/ directory
    write_manifest(
        &dir,
        "level1/level2/east.yml",
        r"
version: 1
projects:
  - name: leaf
",
    );

    let resolved = Manifest::resolve(dir.path().join("east.yml")).unwrap();
    let names: Vec<&str> = resolved.projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["root", "mid", "leaf"]);
}

#[test]
fn resolve_import_allowlist_filters_projects() {
    let dir = TempDir::new().unwrap();
    write_manifest(
        &dir,
        "east.yml",
        r"
version: 1
projects:
  - name: top
imports:
  - file: sub/east.yml
    allowlist: ['hal-*']
",
    );
    write_manifest(
        &dir,
        "sub/east.yml",
        r"
version: 1
projects:
  - name: hal-gpio
  - name: hal-uart
  - name: driver-spi
  - name: unrelated
",
    );

    let resolved = Manifest::resolve(dir.path().join("east.yml")).unwrap();
    let names: Vec<&str> = resolved.projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["top", "hal-gpio", "hal-uart"]);
}

#[test]
fn resolve_first_definition_wins_no_override() {
    let dir = TempDir::new().unwrap();
    write_manifest(
        &dir,
        "east.yml",
        r"
version: 1
projects:
  - name: shared
    revision: v1.0
imports:
  - file: sub/east.yml
",
    );
    write_manifest(
        &dir,
        "sub/east.yml",
        r"
version: 1
projects:
  - name: shared
    revision: v2.0
",
    );

    let resolved = Manifest::resolve(dir.path().join("east.yml")).unwrap();
    // First definition (from top-level) wins
    assert_eq!(resolved.projects.len(), 1);
    assert_eq!(resolved.projects[0].revision.as_deref(), Some("v1.0"));
}

#[test]
fn resolve_detects_direct_cycle() {
    let dir = TempDir::new().unwrap();
    write_manifest(
        &dir,
        "east.yml",
        r"
version: 1
imports:
  - file: east.yml
",
    );

    let err = Manifest::resolve(dir.path().join("east.yml")).unwrap_err();
    assert!(
        err.to_string().contains("cycle"),
        "error should mention cycle: {err}"
    );
}

#[test]
fn resolve_detects_indirect_cycle() {
    let dir = TempDir::new().unwrap();
    write_manifest(
        &dir,
        "a.yml",
        r"
version: 1
imports:
  - file: b.yml
",
    );
    write_manifest(
        &dir,
        "b.yml",
        r"
version: 1
imports:
  - file: a.yml
",
    );

    let err = Manifest::resolve(dir.path().join("a.yml")).unwrap_err();
    assert!(
        err.to_string().contains("cycle"),
        "error should mention cycle: {err}"
    );
}

#[test]
fn resolve_missing_import_file_errors() {
    let dir = TempDir::new().unwrap();
    write_manifest(
        &dir,
        "east.yml",
        r"
version: 1
imports:
  - file: nonexistent.yml
",
    );

    let err = Manifest::resolve(dir.path().join("east.yml")).unwrap_err();
    assert!(
        err.to_string().contains("nonexistent"),
        "error should mention missing file: {err}"
    );
}
