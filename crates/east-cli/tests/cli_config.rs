//! Integration tests for `east config` subcommand.

use std::fs;

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

#[test]
fn config_set_and_get_string() {
    let (workspace, config_home) = setup_workspace();

    // Set a value
    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "set", "user.name", "trekmax"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success();

    // Get the value
    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "get", "user.name"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("trekmax"));
}

#[test]
fn config_set_int() {
    let (workspace, config_home) = setup_workspace();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "set", "--int", "update.parallelism", "16"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "get", "update.parallelism"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("16"));
}

#[test]
fn config_set_bool() {
    let (workspace, config_home) = setup_workspace();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "set", "--bool", "feature.enabled", "true"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "get", "feature.enabled"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("true"));
}

#[test]
fn config_unset() {
    let (workspace, config_home) = setup_workspace();

    // Set then unset
    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "set", "user.name", "trekmax"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "unset", "user.name"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success();

    // Get should fail or show nothing
    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "get", "user.name"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .failure();
}

#[test]
fn config_list() {
    let (workspace, config_home) = setup_workspace();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "set", "user.name", "trekmax"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "set", "user.email", "t@e.com"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "list"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("user.name"))
        .stdout(predicate::str::contains("user.email"));
}

#[test]
fn config_get_missing_key_fails() {
    let (workspace, config_home) = setup_workspace();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["config", "get", "nonexistent.key"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .failure();
}
