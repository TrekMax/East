#![forbid(unsafe_code)]
//! Command trait, extension discovery, and template engine for east.

pub mod error;
pub mod registry;
pub mod template;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::error::TemplateError;
    use crate::registry::{CommandRegistry, CommandSource};
    use crate::template::TemplateEngine;

    // ── CommandRegistry from manifest ───────────────────────────────

    #[test]
    fn registry_from_manifest_commands() {
        let yaml = r#"
version: 1
commands:
  - name: hello
    help: "Say hello"
    exec: "echo hello"
  - name: greet
    help: "Greet someone"
    exec: "echo hi ${arg.target}"
    args:
      - name: target
        help: "Who"
        required: false
        default: "world"
"#;
        let manifest = east_manifest::Manifest::from_yaml_str(yaml).unwrap();
        let registry = CommandRegistry::from_manifest(&manifest);

        assert_eq!(registry.len(), 2);
        let hello = registry.get("hello").unwrap();
        assert_eq!(hello.name, "hello");
        assert!(matches!(hello.source, CommandSource::Manifest));
        let greet = registry.get("greet").unwrap();
        assert_eq!(greet.name, "greet");
    }

    #[test]
    fn registry_get_missing_returns_none() {
        let manifest = east_manifest::Manifest::from_yaml_str("version: 1\n").unwrap();
        let registry = CommandRegistry::from_manifest(&manifest);
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn registry_iter_lists_all_commands() {
        let yaml = r#"
version: 1
commands:
  - name: aaa
    help: "A"
    exec: "echo a"
  - name: bbb
    help: "B"
    exec: "echo b"
"#;
        let manifest = east_manifest::Manifest::from_yaml_str(yaml).unwrap();
        let registry = CommandRegistry::from_manifest(&manifest);
        let names: Vec<&str> = registry.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"aaa"));
        assert!(names.contains(&"bbb"));
    }

    // ── PATH discovery ──────────────────────────────────────────────

    #[test]
    fn registry_discovers_path_executables() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create fake east-foo executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.path().join("east-foo");
            std::fs::write(&path, "#!/bin/sh\necho foo\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        {
            let path = dir.path().join("east-foo.exe");
            std::fs::write(&path, "dummy").unwrap();
        }

        let manifest = east_manifest::Manifest::from_yaml_str("version: 1\n").unwrap();
        let mut registry = CommandRegistry::from_manifest(&manifest);
        registry.discover_path(dir.path().to_str().unwrap());

        let foo = registry.get("foo").unwrap();
        assert_eq!(foo.name, "foo");
        assert!(matches!(foo.source, CommandSource::Path { .. }));
    }

    #[test]
    fn registry_path_ignores_non_east_executables() {
        let dir = tempfile::TempDir::new().unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.path().join("other-tool");
            std::fs::write(&path, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let manifest = east_manifest::Manifest::from_yaml_str("version: 1\n").unwrap();
        let mut registry = CommandRegistry::from_manifest(&manifest);
        registry.discover_path(dir.path().to_str().unwrap());

        assert!(registry.get("other-tool").is_none());
    }

    // ── Collision precedence ────────────────────────────────────────

    #[test]
    fn manifest_command_wins_over_path() {
        let dir = tempfile::TempDir::new().unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.path().join("east-hello");
            std::fs::write(&path, "#!/bin/sh\necho from-path\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        {
            let path = dir.path().join("east-hello.exe");
            std::fs::write(&path, "dummy").unwrap();
        }

        let yaml = r#"
version: 1
commands:
  - name: hello
    help: "From manifest"
    exec: "echo from-manifest"
"#;
        let manifest = east_manifest::Manifest::from_yaml_str(yaml).unwrap();
        let mut registry = CommandRegistry::from_manifest(&manifest);
        registry.discover_path(dir.path().to_str().unwrap());

        let hello = registry.get("hello").unwrap();
        assert!(
            matches!(hello.source, CommandSource::Manifest),
            "manifest should win over PATH"
        );
    }

    // ── TemplateEngine ──────────────────────────────────────────────

    #[test]
    fn template_no_variables() {
        let engine = TemplateEngine::new();
        let result = engine
            .render("hello world", &BTreeMap::new(), "test")
            .unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn template_simple_variable() {
        let engine = TemplateEngine::new();
        let mut vars = BTreeMap::new();
        vars.insert("workspace.root".to_string(), "/my/workspace".to_string());
        let result = engine
            .render("path is ${workspace.root}", &vars, "test")
            .unwrap();
        assert_eq!(result, "path is /my/workspace");
    }

    #[test]
    fn template_multiple_variables() {
        let engine = TemplateEngine::new();
        let mut vars = BTreeMap::new();
        vars.insert("workspace.root".to_string(), "/ws".to_string());
        vars.insert("config.user.name".to_string(), "trekmax".to_string());
        let result = engine
            .render(
                "${workspace.root}/bin as ${config.user.name}",
                &vars,
                "test",
            )
            .unwrap();
        assert_eq!(result, "/ws/bin as trekmax");
    }

    #[test]
    fn template_escape_produces_literal() {
        let engine = TemplateEngine::new();
        let result = engine
            .render("use $${workspace.root} literally", &BTreeMap::new(), "test")
            .unwrap();
        assert_eq!(result, "use ${workspace.root} literally");
    }

    #[test]
    fn template_missing_key_is_error() {
        let engine = TemplateEngine::new();
        let err = engine
            .render("${nonexistent.key}", &BTreeMap::new(), "test.yml")
            .unwrap_err();
        assert!(matches!(err, TemplateError::MissingKey { .. }));
        assert!(err.to_string().contains("nonexistent.key"));
        assert!(err.to_string().contains("test.yml"));
    }

    #[test]
    fn template_unterminated_variable_is_error() {
        let engine = TemplateEngine::new();
        let err = engine
            .render("${workspace.root", &BTreeMap::new(), "test.yml")
            .unwrap_err();
        assert!(matches!(err, TemplateError::UnterminatedVariable { .. }));
    }

    #[test]
    fn template_adjacent_variables() {
        let engine = TemplateEngine::new();
        let mut vars = BTreeMap::new();
        vars.insert("a".to_string(), "X".to_string());
        vars.insert("b".to_string(), "Y".to_string());
        let result = engine.render("${a}${b}", &vars, "test").unwrap();
        assert_eq!(result, "XY");
    }

    #[test]
    fn template_dollar_without_brace_is_literal() {
        let engine = TemplateEngine::new();
        let result = engine
            .render("price is $5", &BTreeMap::new(), "test")
            .unwrap();
        assert_eq!(result, "price is $5");
    }
}
