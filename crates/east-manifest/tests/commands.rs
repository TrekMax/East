//! Tests for manifest command declaration schema validation.

use east_manifest::Manifest;

#[test]
fn parse_manifest_with_exec_command() {
    let yaml = r#"
version: 1
commands:
  - name: hello
    help: "Say hello"
    exec: "echo hello from ${workspace.root}"
"#;
    let m = Manifest::from_yaml_str(yaml).unwrap();
    assert_eq!(m.commands.len(), 1);
    assert_eq!(m.commands[0].name, "hello");
    assert_eq!(m.commands[0].help, "Say hello");
    assert_eq!(
        m.commands[0].exec.as_deref(),
        Some("echo hello from ${workspace.root}")
    );
    assert!(m.commands[0].executable.is_none());
    assert!(m.commands[0].script.is_none());
}

#[test]
fn parse_manifest_with_executable_command() {
    let yaml = r#"
version: 1
commands:
  - name: my-tool
    help: "Run my tool"
    executable: east-mytool
"#;
    let m = Manifest::from_yaml_str(yaml).unwrap();
    assert_eq!(m.commands[0].executable.as_deref(), Some("east-mytool"));
}

#[test]
fn parse_manifest_with_script_command() {
    let yaml = r#"
version: 1
commands:
  - name: setup
    help: "Run setup script"
    script: scripts/setup.sh
"#;
    let m = Manifest::from_yaml_str(yaml).unwrap();
    assert_eq!(m.commands[0].script.as_deref(), Some("scripts/setup.sh"));
}

#[test]
fn parse_command_with_args_env_cwd() {
    let yaml = r#"
version: 1
commands:
  - name: greet
    help: "Greet someone"
    exec: "echo hello ${arg.target}"
    args:
      - name: target
        help: "Who to greet"
        required: false
        default: "world"
    env:
      LANG: "en_US.UTF-8"
    cwd: "${workspace.root}"
"#;
    let m = Manifest::from_yaml_str(yaml).unwrap();
    let cmd = &m.commands[0];
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0].name, "target");
    assert_eq!(cmd.args[0].help, "Who to greet");
    assert!(!cmd.args[0].required);
    assert_eq!(cmd.args[0].default.as_deref(), Some("world"));
    assert_eq!(cmd.env.get("LANG").map(String::as_str), Some("en_US.UTF-8"));
    assert_eq!(cmd.cwd.as_deref(), Some("${workspace.root}"));
}

#[test]
fn parse_command_with_long_help() {
    let yaml = r#"
version: 1
commands:
  - name: info
    help: "Show info"
    long-help: |
      Shows detailed information about the workspace.
      This is a multi-line description.
    exec: "echo info"
"#;
    let m = Manifest::from_yaml_str(yaml).unwrap();
    assert!(m.commands[0].long_help.is_some());
    assert!(m.commands[0]
        .long_help
        .as_ref()
        .unwrap()
        .contains("multi-line"));
}

#[test]
fn reject_command_with_no_exec_or_executable_or_script() {
    let yaml = r#"
version: 1
commands:
  - name: bad
    help: "Missing execution field"
"#;
    let err = Manifest::from_yaml_str(yaml).unwrap_err();
    assert!(
        err.to_string().contains("exec")
            || err.to_string().contains("executable")
            || err.to_string().contains("script"),
        "error should mention missing field: {err}"
    );
}

#[test]
fn reject_command_with_both_exec_and_script() {
    let yaml = r#"
version: 1
commands:
  - name: bad
    help: "Has both"
    exec: "echo"
    script: "run.sh"
"#;
    let err = Manifest::from_yaml_str(yaml).unwrap_err();
    assert!(
        err.to_string().contains("mutually exclusive") || err.to_string().contains("exactly one"),
        "error should mention mutual exclusivity: {err}"
    );
}

#[test]
fn reject_command_with_invalid_name() {
    let yaml = r#"
version: 1
commands:
  - name: Invalid-Name
    help: "Bad name"
    exec: "echo"
"#;
    let err = Manifest::from_yaml_str(yaml).unwrap_err();
    assert!(
        err.to_string().contains("name") || err.to_string().contains("invalid"),
        "error should mention invalid name: {err}"
    );
}

#[test]
fn reject_command_with_reserved_name() {
    for name in &["build", "flash", "debug", "attach", "reset", "import-west"] {
        let yaml = format!(
            r#"
version: 1
commands:
  - name: {name}
    help: "reserved"
    exec: "echo"
"#
        );
        let err = Manifest::from_yaml_str(&yaml).unwrap_err();
        assert!(
            err.to_string().contains("reserved"),
            "error for '{name}' should mention reserved: {err}"
        );
    }
}

#[test]
fn reject_command_with_builtin_name() {
    for name in &[
        "init", "update", "list", "status", "manifest", "config", "help", "version",
    ] {
        let yaml = format!(
            r#"
version: 1
commands:
  - name: {name}
    help: "builtin"
    exec: "echo"
"#
        );
        let err = Manifest::from_yaml_str(&yaml).unwrap_err();
        assert!(
            err.to_string().contains("reserved") || err.to_string().contains("builtin"),
            "error for '{name}' should mention reserved/builtin: {err}"
        );
    }
}
