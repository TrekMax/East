//! Tests for the optional `self:` section in manifest.

use east_manifest::Manifest;

#[test]
fn manifest_without_self_section() {
    let yaml = "version: 1\n";
    let m = Manifest::from_yaml_str(yaml).unwrap();
    assert!(m.manifest_self.is_none());
}

#[test]
fn manifest_with_self_path() {
    let yaml = r"
version: 1
self:
  path: my-app
";
    let m = Manifest::from_yaml_str(yaml).unwrap();
    let s = m.manifest_self.as_ref().unwrap();
    assert_eq!(s.path.as_deref(), Some("my-app"));
}

#[test]
fn manifest_self_with_no_path() {
    // self: section present but path omitted
    let yaml = r"
version: 1
self: {}
";
    let m = Manifest::from_yaml_str(yaml).unwrap();
    let s = m.manifest_self.as_ref().unwrap();
    assert!(s.path.is_none());
}

#[test]
fn manifest_self_with_reserved_fields_ignored() {
    // Future fields like description, maintainers should parse without error
    let yaml = r#"
version: 1
self:
  path: my-app
  description: "My SDK"
  maintainers: ["alice"]
  repo-url: "https://example.com"
"#;
    let m = Manifest::from_yaml_str(yaml).unwrap();
    let s = m.manifest_self.as_ref().unwrap();
    assert_eq!(s.path.as_deref(), Some("my-app"));
}

#[test]
fn manifest_self_coexists_with_projects_and_commands() {
    let yaml = r#"
version: 1
self:
  path: sdk
projects:
  - name: core
commands:
  - name: hello
    help: "Say hi"
    exec: "echo hi"
"#;
    let m = Manifest::from_yaml_str(yaml).unwrap();
    assert!(m.manifest_self.is_some());
    assert_eq!(m.projects.len(), 1);
    assert_eq!(m.commands.len(), 1);
}
