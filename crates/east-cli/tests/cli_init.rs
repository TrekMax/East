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
