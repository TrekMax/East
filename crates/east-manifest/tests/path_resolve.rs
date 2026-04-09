//! Tests for `ManifestRelativePath` resolve logic.

use std::fs;

use east_manifest::path_resolve::ManifestRelativePath;
use tempfile::TempDir;

#[test]
fn resolve_relative_path() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("sub/east.yml");
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    fs::write(&manifest_path, "").unwrap();

    // Create the target file
    let target = dir.path().join("sub/scripts/run.sh");
    fs::create_dir_all(dir.path().join("sub/scripts")).unwrap();
    fs::write(&target, "#!/bin/sh\n").unwrap();

    let mrp = ManifestRelativePath::new(&manifest_path, "scripts/run.sh");
    let resolved = mrp.resolve().unwrap();
    assert_eq!(resolved, fs::canonicalize(&target).unwrap());
}

#[test]
fn resolve_absolute_path_used_as_is() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("east.yml");
    fs::write(&manifest_path, "").unwrap();

    // Create an absolute target
    let target = dir.path().join("abs-script.sh");
    fs::write(&target, "#!/bin/sh\n").unwrap();
    let abs_path = fs::canonicalize(&target).unwrap();

    let mrp = ManifestRelativePath::new(&manifest_path, abs_path.to_str().unwrap());
    let resolved = mrp.resolve().unwrap();
    assert_eq!(resolved, abs_path);
}

#[test]
fn resolve_parent_escape() {
    let dir = TempDir::new().unwrap();
    // Manifest is in sub/east.yml, script is ../outside.sh (i.e. at dir root)
    let manifest_path = dir.path().join("sub/east.yml");
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    fs::write(&manifest_path, "").unwrap();

    let target = dir.path().join("outside.sh");
    fs::write(&target, "#!/bin/sh\n").unwrap();

    let mrp = ManifestRelativePath::new(&manifest_path, "../outside.sh");
    let resolved = mrp.resolve().unwrap();
    assert_eq!(resolved, fs::canonicalize(&target).unwrap());
}

#[test]
fn resolve_missing_file_errors() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join("east.yml");
    fs::write(&manifest_path, "").unwrap();

    let mrp = ManifestRelativePath::new(&manifest_path, "nonexistent.sh");
    let err = mrp.resolve().unwrap_err();
    let err_msg = err.to_string();
    // Error must mention the manifest path and the attempted path
    assert!(
        err_msg.contains("nonexistent.sh"),
        "error should mention attempted path: {err_msg}"
    );
    assert!(
        err_msg.contains("east.yml"),
        "error should mention declaring manifest: {err_msg}"
    );
}

#[test]
fn resolve_deeply_nested_manifest() {
    let dir = TempDir::new().unwrap();
    // Manifest at a/b/c/east.yml, script at a/b/c/helpers/tool.sh
    let manifest_dir = dir.path().join("a/b/c");
    fs::create_dir_all(&manifest_dir).unwrap();
    let manifest_path = manifest_dir.join("east.yml");
    fs::write(&manifest_path, "").unwrap();

    let helpers_dir = manifest_dir.join("helpers");
    fs::create_dir_all(&helpers_dir).unwrap();
    let target = helpers_dir.join("tool.sh");
    fs::write(&target, "#!/bin/sh\n").unwrap();

    let mrp = ManifestRelativePath::new(&manifest_path, "helpers/tool.sh");
    let resolved = mrp.resolve().unwrap();
    assert_eq!(resolved, fs::canonicalize(&target).unwrap());
}
