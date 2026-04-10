//! Integration tests for `east init`.

use std::fs;
use std::process::Command;

use assert_cmd::Command as AssertCmd;
use predicates::prelude::*;
use tempfile::TempDir;

/// Create a local git repo containing an `east.yml` manifest with a project
/// that points to another local repo.
fn setup_manifest_repo(dir: &tempfile::TempDir) -> (String, String) {
    let manifest_repo = dir.path().join("manifest-repo");
    let project_repo = dir.path().join("project-repo");

    // Create the project repo with a commit
    Command::new("git")
        .args(["init", "-b", "main"])
        .arg(&project_repo)
        .output()
        .expect("git init project failed");
    for (key, val) in [
        ("user.email", "test@test.com"),
        ("user.name", "Test"),
        ("commit.gpgsign", "false"),
    ] {
        Command::new("git")
            .arg("-C")
            .arg(&project_repo)
            .args(["config", key, val])
            .output()
            .unwrap();
    }
    fs::write(project_repo.join("lib.rs"), "// project code\n").unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&project_repo)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&project_repo)
        .args(["commit", "-m", "init"])
        .output()
        .unwrap();

    // Create the manifest repo with east.yml
    Command::new("git")
        .args(["init", "-b", "main"])
        .arg(&manifest_repo)
        .output()
        .expect("git init manifest failed");
    for (key, val) in [
        ("user.email", "test@test.com"),
        ("user.name", "Test"),
        ("commit.gpgsign", "false"),
    ] {
        Command::new("git")
            .arg("-C")
            .arg(&manifest_repo)
            .args(["config", key, val])
            .output()
            .unwrap();
    }

    let manifest_content = format!(
        r"version: 1

remotes:
  - name: local
    url-base: {project_parent}

defaults:
  remote: local
  revision: main

projects:
  - name: {project_name}
",
        project_parent = project_repo.parent().unwrap().display(),
        project_name = project_repo.file_name().unwrap().to_str().unwrap(),
    );
    fs::write(manifest_repo.join("east.yml"), manifest_content).unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&manifest_repo)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&manifest_repo)
        .args(["commit", "-m", "add manifest"])
        .output()
        .unwrap();

    (
        manifest_repo.to_str().unwrap().to_string(),
        project_repo
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
    )
}

#[test]
fn init_creates_workspace_from_local_manifest_repo() {
    let fixture = TempDir::new().unwrap();
    let (manifest_url, project_name) = setup_manifest_repo(&fixture);

    let workspace = TempDir::new().unwrap();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["init", &manifest_url])
        .current_dir(workspace.path())
        .assert()
        .success();

    // .east/ directory should exist
    assert!(workspace.path().join(".east").is_dir());
    // east.yml should exist
    assert!(workspace.path().join("east.yml").exists());
    // The project should be cloned
    assert!(
        workspace.path().join(&project_name).exists(),
        "project {project_name} should be cloned"
    );
    assert!(workspace.path().join(&project_name).join("lib.rs").exists());
}

#[test]
fn init_from_branch_with_revision_flag() {
    let fixture = TempDir::new().unwrap();
    let (_manifest_path, project_name) = setup_manifest_repo(&fixture);
    let manifest_repo = fixture.path().join("manifest-repo");

    // Create a new branch "dev" with a different east.yml
    Command::new("git")
        .arg("-C")
        .arg(&manifest_repo)
        .args(["checkout", "-b", "dev"])
        .output()
        .unwrap();
    let dev_manifest = format!(
        r"version: 1

remotes:
  - name: local
    url-base: {project_parent}

defaults:
  remote: local
  revision: main

projects:
  - name: {project_name}
    path: dev/{project_name}
",
        project_parent = fixture
            .path()
            .join("project-repo")
            .parent()
            .unwrap()
            .display(),
        project_name = project_name,
    );
    fs::write(manifest_repo.join("east.yml"), dev_manifest).unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&manifest_repo)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&manifest_repo)
        .args(["commit", "-m", "dev branch manifest"])
        .output()
        .unwrap();

    // Switch back to main so we can verify -r fetches dev
    Command::new("git")
        .arg("-C")
        .arg(&manifest_repo)
        .args(["checkout", "main"])
        .output()
        .unwrap();

    let workspace = TempDir::new().unwrap();

    // Use file:// URL to trigger git clone path (local dir path skips revision)
    let file_url = format!("file://{}", manifest_repo.display());

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["init", &file_url, "-r", "dev"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // The project should be cloned under dev/ path (from dev branch manifest)
    assert!(
        workspace.path().join("dev").join(&project_name).exists(),
        "project should be cloned at dev/{project_name} per dev branch manifest"
    );
}

#[test]
fn init_from_tag_with_revision_flag() {
    let fixture = TempDir::new().unwrap();
    let (_manifest_path, _project_name) = setup_manifest_repo(&fixture);
    let manifest_repo = fixture.path().join("manifest-repo");

    // Tag the current commit
    Command::new("git")
        .arg("-C")
        .arg(&manifest_repo)
        .args(["tag", "v1.0"])
        .output()
        .unwrap();

    let workspace = TempDir::new().unwrap();

    // Use file:// URL to trigger git clone path
    let file_url = format!("file://{}", manifest_repo.display());

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["init", &file_url, "-r", "v1.0"])
        .current_dir(workspace.path())
        .assert()
        .success();

    assert!(workspace.path().join(".east").is_dir());
    assert!(workspace.path().join("east.yml").exists());
}

#[test]
fn init_help_shows_revision_option() {
    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--revision"));
}

#[test]
fn init_fails_with_invalid_manifest() {
    let workspace = TempDir::new().unwrap();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["init", "/nonexistent/path"])
        .current_dir(workspace.path())
        .assert()
        .failure();
}

#[test]
fn init_shows_help() {
    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("manifest"));
}
