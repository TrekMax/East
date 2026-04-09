//! Integration tests for script: path resolution relative to declaring manifest.

use std::fs;
use std::path::Path;

use assert_cmd::Command as AssertCmd;
use predicates::prelude::*;
use tempfile::TempDir;

fn east_cmd(config_home: &Path) -> AssertCmd {
    let mut cmd = AssertCmd::cargo_bin("east").unwrap();
    cmd.env("XDG_CONFIG_HOME", config_home);
    cmd.env("APPDATA", config_home);
    cmd
}

/// Test that a script declared in an imported manifest (two dirs deep)
/// resolves relative to that manifest, not the workspace root.
#[test]
#[cfg(unix)]
fn script_resolves_relative_to_declaring_manifest() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TempDir::new().unwrap();
    fs::create_dir_all(workspace.path().join(".east")).unwrap();

    // Top-level manifest imports sub/sdk/east.yml
    let top_yaml = r"
version: 1
imports:
  - file: sub/sdk/east.yml
";
    fs::write(workspace.path().join("east.yml"), top_yaml).unwrap();

    // Imported manifest at sub/sdk/east.yml declares a script command
    let sub_dir = workspace.path().join("sub/sdk");
    fs::create_dir_all(&sub_dir).unwrap();
    let sub_yaml = r"
version: 1
commands:
  - name: sdk-tool
    help: Run SDK tool
    script: helpers/tool.sh
";
    fs::write(sub_dir.join("east.yml"), sub_yaml).unwrap();

    // The script lives at sub/sdk/helpers/tool.sh (relative to the imported manifest)
    let helpers_dir = sub_dir.join("helpers");
    fs::create_dir_all(&helpers_dir).unwrap();
    let script = helpers_dir.join("tool.sh");
    fs::write(&script, "#!/bin/sh\necho from-imported-script\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let config_home = TempDir::new().unwrap();

    east_cmd(config_home.path())
        .args(["sdk-tool"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("from-imported-script"));
}

/// Script at workspace root still works (regression check).
#[test]
#[cfg(unix)]
fn script_at_workspace_root_still_works() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TempDir::new().unwrap();
    fs::create_dir_all(workspace.path().join(".east")).unwrap();

    let yaml = r"
version: 1
commands:
  - name: root-script
    help: Run root script
    script: scripts/hello.sh
";
    fs::write(workspace.path().join("east.yml"), yaml).unwrap();

    let scripts_dir = workspace.path().join("scripts");
    fs::create_dir_all(&scripts_dir).unwrap();
    let script = scripts_dir.join("hello.sh");
    fs::write(&script, "#!/bin/sh\necho from-root-script\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let config_home = TempDir::new().unwrap();

    east_cmd(config_home.path())
        .args(["root-script"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("from-root-script"));
}
