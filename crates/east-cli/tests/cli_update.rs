//! Integration tests for `east update` with concurrent projects.

use std::fs;
use std::process::Command;

use assert_cmd::Command as AssertCmd;
use predicates::prelude::*;
use tempfile::TempDir;

/// Create N local project repos and a manifest repo referencing them.
fn setup_multi_project_workspace(n: usize) -> (TempDir, TempDir) {
    let fixture = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    // Create N project repos
    let mut project_entries = String::new();
    for i in 0..n {
        let name = format!("project-{i}");
        let repo_path = fixture.path().join(&name);
        create_repo(&repo_path, "main", &format!("// code for {name}\n"));
        project_entries.push_str(&format!("  - name: {name}\n"));
    }

    // Create manifest repo
    let manifest_repo = fixture.path().join("manifest-repo");
    Command::new("git")
        .args(["init", "-b", "main"])
        .arg(&manifest_repo)
        .output()
        .unwrap();
    git_config(&manifest_repo);

    let manifest = format!(
        r"version: 1

remotes:
  - name: local
    url-base: {base}

defaults:
  remote: local
  revision: main

projects:
{projects}",
        base = fixture.path().display(),
        projects = project_entries,
    );
    fs::write(manifest_repo.join("east.yml"), manifest).unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&manifest_repo)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&manifest_repo)
        .args(["commit", "-m", "init"])
        .output()
        .unwrap();

    // Initialize workspace
    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["init", manifest_repo.to_str().unwrap()])
        .current_dir(workspace.path())
        .assert()
        .success();

    (fixture, workspace)
}

fn create_repo(dir: &std::path::Path, branch: &str, file_content: &str) {
    Command::new("git")
        .args(["init", "-b", branch])
        .arg(dir)
        .output()
        .unwrap();
    git_config(dir);
    fs::write(dir.join("lib.rs"), file_content).unwrap();
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["commit", "-m", "init"])
        .output()
        .unwrap();
}

fn git_config(dir: &std::path::Path) {
    for (key, val) in [
        ("user.email", "test@test.com"),
        ("user.name", "Test"),
        ("commit.gpgsign", "false"),
    ] {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["config", key, val])
            .output()
            .unwrap();
    }
}

#[test]
fn update_with_3_concurrent_projects() {
    let (_fixture, workspace) = setup_multi_project_workspace(3);

    // All 3 projects should be cloned
    for i in 0..3 {
        let project_dir = workspace.path().join(format!("project-{i}"));
        assert!(project_dir.exists(), "project-{i} should exist");
        assert!(
            project_dir.join("lib.rs").exists(),
            "project-{i}/lib.rs should exist"
        );
    }

    // Running update again should succeed (fetch + checkout)
    AssertCmd::cargo_bin("east")
        .unwrap()
        .arg("update")
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("updated 3 projects"));
}

#[test]
fn list_shows_all_projects() {
    let (_fixture, workspace) = setup_multi_project_workspace(3);

    AssertCmd::cargo_bin("east")
        .unwrap()
        .arg("list")
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("project-0"))
        .stdout(predicate::str::contains("project-1"))
        .stdout(predicate::str::contains("project-2"))
        .stdout(predicate::str::contains("yes")); // cloned
}

#[test]
fn status_shows_clean_projects() {
    let (_fixture, workspace) = setup_multi_project_workspace(2);

    AssertCmd::cargo_bin("east")
        .unwrap()
        .arg("status")
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("project-0"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("main"));
}

#[test]
fn status_detects_dirty_project() {
    let (_fixture, workspace) = setup_multi_project_workspace(1);

    // Make project-0 dirty
    fs::write(workspace.path().join("project-0/lib.rs"), "// modified\n").unwrap();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .arg("status")
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("dirty"));
}

#[test]
fn manifest_resolve_outputs_yaml() {
    let (_fixture, workspace) = setup_multi_project_workspace(2);

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["manifest", "--resolve"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("version: 1"))
        .stdout(predicate::str::contains("project-0"))
        .stdout(predicate::str::contains("project-1"));
}

#[test]
fn update_outside_workspace_fails() {
    let dir = TempDir::new().unwrap();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .arg("update")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("workspace"));
}
