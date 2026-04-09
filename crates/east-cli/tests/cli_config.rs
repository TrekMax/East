//! Integration tests for `east config` subcommand.

use std::fs;
use std::path::Path;

use assert_cmd::Command as AssertCmd;
use predicates::prelude::*;
use tempfile::TempDir;

/// Set up a workspace with .east/ and a custom global config dir.
fn setup_workspace() -> (TempDir, TempDir) {
    let workspace = TempDir::new().unwrap();
    fs::create_dir_all(workspace.path().join(".east")).unwrap();
    fs::write(workspace.path().join("east.yml"), "version: 1\n").unwrap();

    let config_home = TempDir::new().unwrap();
    (workspace, config_home)
}

/// Build an east command with config dir isolation for all platforms.
/// Sets both `XDG_CONFIG_HOME` (Unix) and `APPDATA` (Windows).
fn east_cmd(config_home: &Path) -> AssertCmd {
    let mut cmd = AssertCmd::cargo_bin("east").unwrap();
    cmd.env("XDG_CONFIG_HOME", config_home);
    cmd.env("APPDATA", config_home);
    cmd
}

#[test]
fn config_set_and_get_string() {
    let (workspace, config_home) = setup_workspace();

    east_cmd(config_home.path())
        .args(["config", "set", "user.name", "trekmax"])
        .current_dir(workspace.path())
        .assert()
        .success();

    east_cmd(config_home.path())
        .args(["config", "get", "user.name"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("trekmax"));
}

#[test]
fn config_set_int() {
    let (workspace, config_home) = setup_workspace();

    east_cmd(config_home.path())
        .args(["config", "set", "--int", "update.parallelism", "16"])
        .current_dir(workspace.path())
        .assert()
        .success();

    east_cmd(config_home.path())
        .args(["config", "get", "update.parallelism"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("16"));
}

#[test]
fn config_set_bool() {
    let (workspace, config_home) = setup_workspace();

    east_cmd(config_home.path())
        .args(["config", "set", "--bool", "feature.enabled", "true"])
        .current_dir(workspace.path())
        .assert()
        .success();

    east_cmd(config_home.path())
        .args(["config", "get", "feature.enabled"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("true"));
}

#[test]
fn config_unset() {
    let (workspace, config_home) = setup_workspace();

    east_cmd(config_home.path())
        .args(["config", "set", "user.name", "trekmax"])
        .current_dir(workspace.path())
        .assert()
        .success();

    east_cmd(config_home.path())
        .args(["config", "unset", "user.name"])
        .current_dir(workspace.path())
        .assert()
        .success();

    east_cmd(config_home.path())
        .args(["config", "get", "user.name"])
        .current_dir(workspace.path())
        .assert()
        .failure();
}

#[test]
fn config_list() {
    let (workspace, config_home) = setup_workspace();

    east_cmd(config_home.path())
        .args(["config", "set", "user.name", "trekmax"])
        .current_dir(workspace.path())
        .assert()
        .success();

    east_cmd(config_home.path())
        .args(["config", "set", "user.email", "t@e.com"])
        .current_dir(workspace.path())
        .assert()
        .success();

    east_cmd(config_home.path())
        .args(["config", "list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("user.name"))
        .stdout(predicate::str::contains("user.email"));
}

#[test]
fn config_get_missing_key_fails() {
    let (workspace, config_home) = setup_workspace();

    east_cmd(config_home.path())
        .args(["config", "get", "nonexistent.key"])
        .current_dir(workspace.path())
        .assert()
        .failure();
}
