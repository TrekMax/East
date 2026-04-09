//! Integration tests for extension command dispatch.

use std::fs;
use std::process::Command;

use assert_cmd::Command as AssertCmd;
use predicates::prelude::*;
use tempfile::TempDir;

/// Set up a workspace with an east.yml containing commands.
fn setup_workspace_with_commands(manifest_yaml: &str) -> (TempDir, TempDir) {
    let workspace = TempDir::new().unwrap();
    fs::create_dir_all(workspace.path().join(".east")).unwrap();
    fs::write(workspace.path().join("east.yml"), manifest_yaml).unwrap();
    let config_home = TempDir::new().unwrap();
    (workspace, config_home)
}

// ── exec commands ───────────────────────────────────────────────────

#[test]
fn dispatch_exec_command() {
    let yaml = r#"
version: 1
commands:
  - name: hello
    help: "Say hello"
    exec: "echo hello-from-east"
"#;
    let (workspace, config_home) = setup_workspace_with_commands(yaml);

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["hello"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("hello-from-east"));
}

#[test]
fn dispatch_exec_command_with_template_variable() {
    let yaml = r#"
version: 1
commands:
  - name: show-root
    help: "Show workspace root"
    exec: "echo root=${workspace.root}"
"#;
    let (workspace, config_home) = setup_workspace_with_commands(yaml);

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["show-root"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("root="));
}

#[test]
fn dispatch_exec_command_with_env() {
    let yaml = r#"
version: 1
commands:
  - name: show-env
    help: "Show env var"
    exec: "echo MY_VAR=$MY_VAR"
    env:
      MY_VAR: "hello-env"
"#;
    let (workspace, config_home) = setup_workspace_with_commands(yaml);

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["show-env"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("MY_VAR=hello-env"));
}

// ── script commands ─────────────────────────────────────────────────

#[test]
#[cfg(unix)]
fn dispatch_script_command() {
    let workspace = TempDir::new().unwrap();
    fs::create_dir_all(workspace.path().join(".east")).unwrap();

    // Create script
    let scripts_dir = workspace.path().join("scripts");
    fs::create_dir_all(&scripts_dir).unwrap();
    let script_path = scripts_dir.join("greet.sh");
    fs::write(&script_path, "#!/bin/sh\necho \"hello from script\"\n").unwrap();

    // Make executable
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

    let yaml = r#"
version: 1
commands:
  - name: greet
    help: "Run greet script"
    script: scripts/greet.sh
"#;
    fs::write(workspace.path().join("east.yml"), yaml).unwrap();
    let config_home = TempDir::new().unwrap();

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["greet"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from script"));
}

// ── PATH-based commands ─────────────────────────────────────────────

#[test]
#[cfg(unix)]
fn dispatch_path_command() {
    let workspace = TempDir::new().unwrap();
    fs::create_dir_all(workspace.path().join(".east")).unwrap();
    fs::write(workspace.path().join("east.yml"), "version: 1\n").unwrap();

    // Create east-mytool on fake PATH
    let bin_dir = TempDir::new().unwrap();
    let tool_path = bin_dir.path().join("east-mytool");
    fs::write(&tool_path, "#!/bin/sh\necho \"from-path-tool $@\"\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o755)).unwrap();

    let config_home = TempDir::new().unwrap();
    let path_env = format!(
        "{}:{}",
        bin_dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["mytool", "--some-flag"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .env("PATH", &path_env)
        .assert()
        .success()
        .stdout(predicate::str::contains("from-path-tool"));
}

// ── Unknown command ─────────────────────────────────────────────────

#[test]
fn unknown_command_fails() {
    let (workspace, config_home) = setup_workspace_with_commands("version: 1\n");

    AssertCmd::cargo_bin("east")
        .unwrap()
        .args(["nonexistent-cmd"])
        .current_dir(workspace.path())
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .failure();
}
