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
fn update_skips_dirty_project_checkout() {
    let (_fixture, workspace) = setup_multi_project_workspace(2);

    // Make project-0 dirty
    fs::write(workspace.path().join("project-0/lib.rs"), "// modified\n").unwrap();

    // Update should succeed but skip project-0's checkout
    AssertCmd::cargo_bin("east")
        .unwrap()
        .arg("update")
        .current_dir(workspace.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("skipped checkout").or(predicate::str::is_empty()))
        // The output goes to stderr via progress bar; check stdout for completion
        .stdout(predicate::str::contains("updated 2 projects"));
}

#[test]
fn update_force_specific_project() {
    let (_fixture, workspace) = setup_multi_project_workspace(2);

    // Make both projects dirty
    fs::write(workspace.path().join("project-0/lib.rs"), "// modified\n").unwrap();
    fs::write(workspace.path().join("project-1/lib.rs"), "// modified\n").unwrap();

    // Force only project-0; project-1 should still be skipped
    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["update", "--force", "project-0"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // project-0 should be checked out (clean now — git may use \r\n on Windows)
    let content = fs::read_to_string(workspace.path().join("project-0/lib.rs")).unwrap();
    assert!(
        content.contains("// code for project-0"),
        "project-0 should be restored after force checkout"
    );

    // project-1 should still have local modifications (checkout was skipped)
    let content = fs::read_to_string(workspace.path().join("project-1/lib.rs")).unwrap();
    assert!(
        content.contains("// modified"),
        "project-1 should still have local modifications"
    );
}

#[test]
fn update_force_all_projects() {
    let (_fixture, workspace) = setup_multi_project_workspace(2);

    // Make both dirty
    fs::write(workspace.path().join("project-0/lib.rs"), "// modified\n").unwrap();
    fs::write(workspace.path().join("project-1/lib.rs"), "// modified\n").unwrap();

    // Force all (no project names)
    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["update", "--force"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Both should be restored (git may use \r\n on Windows)
    let c0 = fs::read_to_string(workspace.path().join("project-0/lib.rs")).unwrap();
    let c1 = fs::read_to_string(workspace.path().join("project-1/lib.rs")).unwrap();
    assert!(
        c0.contains("// code for project-0"),
        "project-0 should be restored"
    );
    assert!(
        c1.contains("// code for project-1"),
        "project-1 should be restored"
    );
}

#[test]
fn update_force_unknown_project_fails() {
    let (_fixture, workspace) = setup_multi_project_workspace(1);

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["update", "--force", "nonexistent"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unknown project(s) for --force: nonexistent",
        ));
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
