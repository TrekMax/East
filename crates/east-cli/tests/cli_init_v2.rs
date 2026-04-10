//! Integration tests for `east init` Phase 2.6 — three init modes.

use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::Command as AssertCmd;
use predicates::prelude::*;
use tempfile::TempDir;

/// Helper: create a manifest repo (directory with east.yml + git init).
fn create_manifest_repo(parent: &Path, dir_name: &str, east_yml: &str) {
    let repo = parent.join(dir_name);
    fs::create_dir_all(&repo).unwrap();
    fs::write(repo.join("east.yml"), east_yml).unwrap();

    Command::new("git")
        .args(["init"])
        .arg(&repo)
        .output()
        .expect("git init failed");

    for (key, val) in [
        ("user.email", "test@test.com"),
        ("user.name", "Test"),
        ("commit.gpgsign", "false"),
    ] {
        Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["config", key, val])
            .output()
            .unwrap();
    }

    Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["commit", "-m", "init manifest"])
        .output()
        .unwrap();
}

fn east_cmd(config_home: &Path) -> AssertCmd {
    let mut cmd = AssertCmd::cargo_bin("east").unwrap();
    cmd.env("XDG_CONFIG_HOME", config_home);
    cmd.env("APPDATA", config_home);
    cmd
}

// ── Mode L: local existing repo ─────────────────────────────────────

#[test]
fn init_local_creates_workspace() {
    let dir = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();

    create_manifest_repo(dir.path(), "my-app", "version: 1\n");

    east_cmd(config_home.path())
        .args(["init", "-l", "my-app"])
        .current_dir(dir.path())
        .assert()
        .success();

    // .east/ should exist at workspace root (parent of my-app)
    assert!(dir.path().join(".east").is_dir());
    // config.toml should have [manifest] section
    let config = fs::read_to_string(dir.path().join(".east/config.toml")).unwrap();
    assert!(config.contains("manifest"), "config should have manifest section");
    assert!(config.contains("my-app"), "config should reference my-app");
}

#[test]
fn init_local_east_already_exists_fails() {
    let dir = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();

    create_manifest_repo(dir.path(), "my-app", "version: 1\n");
    fs::create_dir_all(dir.path().join(".east")).unwrap();

    east_cmd(config_home.path())
        .args(["init", "-l", "my-app"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already"));
}

#[test]
fn init_local_missing_manifest_fails() {
    let dir = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();

    // Directory exists but no east.yml
    fs::create_dir_all(dir.path().join("empty-dir")).unwrap();

    east_cmd(config_home.path())
        .args(["init", "-l", "empty-dir"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn init_local_nonexistent_dir_fails() {
    let dir = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();

    east_cmd(config_home.path())
        .args(["init", "-l", "no-such-dir"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

// ── Mode T: template ────────────────────────────────────────────────

#[test]
fn init_template_default_dir() {
    let dir = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();

    east_cmd(config_home.path())
        .args(["init"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Default dir is "manifest"
    assert!(dir.path().join("manifest").is_dir());
    assert!(dir.path().join("manifest/east.yml").exists());
    assert!(dir.path().join("manifest/.git").is_dir());
    assert!(dir.path().join("manifest/.gitignore").exists());
    assert!(dir.path().join(".east").is_dir());
    assert!(dir.path().join(".east/config.toml").exists());
}

#[test]
fn init_template_custom_dir() {
    let dir = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();

    east_cmd(config_home.path())
        .args(["init", "my-sdk"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join("my-sdk/east.yml").exists());
    assert!(dir.path().join("my-sdk/.git").is_dir());
    assert!(dir.path().join(".east").is_dir());

    let config = fs::read_to_string(dir.path().join(".east/config.toml")).unwrap();
    assert!(config.contains("my-sdk"), "config should reference my-sdk");
}

#[test]
fn init_template_no_initial_commit() {
    let dir = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();

    east_cmd(config_home.path())
        .args(["init"])
        .current_dir(dir.path())
        .assert()
        .success();

    // git log should fail (no commits yet)
    let output = Command::new("git")
        .arg("-C")
        .arg(dir.path().join("manifest"))
        .args(["log", "--oneline"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "should have no commits yet");
}
