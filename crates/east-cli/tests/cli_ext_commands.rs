//! Integration tests for extension command dispatch.

use std::fs;
use std::path::Path;

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

/// Build an east command with config dir isolation for all platforms.
fn east_cmd(config_home: &Path) -> AssertCmd {
    let mut cmd = AssertCmd::cargo_bin("east").unwrap();
    cmd.env("XDG_CONFIG_HOME", config_home);
    cmd.env("APPDATA", config_home);
    cmd
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

    east_cmd(config_home.path())
        .args(["hello"])
        .current_dir(workspace.path())
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

    east_cmd(config_home.path())
        .args(["show-root"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("root="));
}

#[test]
#[cfg(unix)]
fn dispatch_exec_command_with_env() {
    // This test uses $MY_VAR shell expansion syntax which is Unix-only.
    // The env var injection itself is cross-platform; only the echo syntax differs.
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

    east_cmd(config_home.path())
        .args(["show-env"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("MY_VAR=hello-env"));
}

// ── script commands ─────────────────────────────────────────────────

#[test]
#[cfg(unix)]
fn dispatch_script_command() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TempDir::new().unwrap();
    fs::create_dir_all(workspace.path().join(".east")).unwrap();

    // Create script
    let scripts_dir = workspace.path().join("scripts");
    fs::create_dir_all(&scripts_dir).unwrap();
    let script_path = scripts_dir.join("greet.sh");
    fs::write(&script_path, "#!/bin/sh\necho \"hello from script\"\n").unwrap();
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

    east_cmd(config_home.path())
        .args(["greet"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from script"));
}

// ── PATH-based commands ─────────────────────────────────────────────

#[test]
#[cfg(unix)]
fn dispatch_path_command() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TempDir::new().unwrap();
    fs::create_dir_all(workspace.path().join(".east")).unwrap();
    fs::write(workspace.path().join("east.yml"), "version: 1\n").unwrap();

    // Create east-mytool on fake PATH
    let bin_dir = TempDir::new().unwrap();
    let tool_path = bin_dir.path().join("east-mytool");
    fs::write(&tool_path, "#!/bin/sh\necho \"from-path-tool $@\"\n").unwrap();
    fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o755)).unwrap();

    let config_home = TempDir::new().unwrap();
    let path_env = format!(
        "{}:{}",
        bin_dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );

    east_cmd(config_home.path())
        .args(["mytool", "--some-flag"])
        .current_dir(workspace.path())
        .env("PATH", &path_env)
        .assert()
        .success()
        .stdout(predicate::str::contains("from-path-tool"));
}

// ── Unknown command ─────────────────────────────────────────────────

#[test]
fn unknown_command_fails() {
    let (workspace, config_home) = setup_workspace_with_commands("version: 1\n");

    east_cmd(config_home.path())
        .args(["nonexistent-cmd"])
        .current_dir(workspace.path())
        .assert()
        .failure();
}
