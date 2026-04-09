#![forbid(unsafe_code)]
//! Three-layer TOML configuration for east.
//!
//! Provides a layered configuration system that merges system, global,
//! and workspace-level TOML files. Higher-precedence layers override
//! lower ones on a per-key basis.

mod config;
pub mod error;
pub mod path;
mod store;
mod value;

pub use config::{Config, ConfigLayer};
pub use store::ConfigStore;
pub use value::ConfigValue;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathProvider;

    // ── ConfigValue ─────────────────────────────────────────────────

    #[test]
    fn config_value_string() {
        let v = ConfigValue::String("hello".into());
        assert_eq!(v.as_str(), Some("hello"));
        assert_eq!(v.as_i64(), None);
        assert_eq!(v.as_bool(), None);
    }

    #[test]
    fn config_value_integer() {
        let v = ConfigValue::Integer(42);
        assert_eq!(v.as_i64(), Some(42));
        assert_eq!(v.as_str(), None);
    }

    #[test]
    fn config_value_bool() {
        let v = ConfigValue::Boolean(true);
        assert_eq!(v.as_bool(), Some(true));
        assert_eq!(v.as_str(), None);
    }

    #[test]
    fn config_value_float() {
        let v = ConfigValue::Float(1.5);
        let f = v.as_f64().unwrap();
        assert!((f - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn config_value_display_string() {
        let v = ConfigValue::String("hello".into());
        assert_eq!(v.to_string(), "hello");
    }

    #[test]
    fn config_value_display_integer() {
        let v = ConfigValue::Integer(42);
        assert_eq!(v.to_string(), "42");
    }

    #[test]
    fn config_value_display_bool() {
        let v = ConfigValue::Boolean(false);
        assert_eq!(v.to_string(), "false");
    }

    // ── ConfigStore get/set with dotted keys ────────────────────────

    #[test]
    fn store_set_and_get_simple_key() {
        let mut store = ConfigStore::new();
        store.set("name", ConfigValue::String("east".into()));
        assert_eq!(
            store.get("name").and_then(ConfigValue::as_str),
            Some("east")
        );
    }

    #[test]
    fn store_set_and_get_dotted_key() {
        let mut store = ConfigStore::new();
        store.set("user.name", ConfigValue::String("trekmax".into()));
        assert_eq!(
            store.get("user.name").and_then(ConfigValue::as_str),
            Some("trekmax")
        );
    }

    #[test]
    fn store_set_and_get_deeply_nested_key() {
        let mut store = ConfigStore::new();
        store.set("a.b.c.d", ConfigValue::Integer(99));
        assert_eq!(store.get("a.b.c.d").and_then(ConfigValue::as_i64), Some(99));
    }

    #[test]
    fn store_get_missing_key_returns_none() {
        let store = ConfigStore::new();
        assert!(store.get("nonexistent").is_none());
        assert!(store.get("a.b.c").is_none());
    }

    #[test]
    fn store_overwrite_existing_key() {
        let mut store = ConfigStore::new();
        store.set("user.name", ConfigValue::String("old".into()));
        store.set("user.name", ConfigValue::String("new".into()));
        assert_eq!(
            store.get("user.name").and_then(ConfigValue::as_str),
            Some("new")
        );
    }

    #[test]
    fn store_unset_key() {
        let mut store = ConfigStore::new();
        store.set("user.name", ConfigValue::String("x".into()));
        store.unset("user.name");
        assert!(store.get("user.name").is_none());
    }

    #[test]
    fn store_unset_nonexistent_is_noop() {
        let mut store = ConfigStore::new();
        store.unset("nothing.here"); // should not panic
    }

    #[test]
    fn store_multiple_keys_in_same_namespace() {
        let mut store = ConfigStore::new();
        store.set("user.name", ConfigValue::String("trekmax".into()));
        store.set("user.email", ConfigValue::String("t@e.com".into()));
        assert_eq!(
            store.get("user.name").and_then(ConfigValue::as_str),
            Some("trekmax")
        );
        assert_eq!(
            store.get("user.email").and_then(ConfigValue::as_str),
            Some("t@e.com")
        );
    }

    #[test]
    fn store_iter_returns_all_leaf_keys() {
        let mut store = ConfigStore::new();
        store.set("user.name", ConfigValue::String("trekmax".into()));
        store.set("user.email", ConfigValue::String("t@e.com".into()));
        store.set("update.parallelism", ConfigValue::Integer(4));

        let mut entries: Vec<(String, String)> =
            store.iter().map(|(k, v)| (k, v.to_string())).collect();
        entries.sort();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].0, "update.parallelism");
        assert_eq!(entries[1].0, "user.email");
        assert_eq!(entries[2].0, "user.name");
    }

    // ── ConfigStore merge ───────────────────────────────────────────

    #[test]
    fn store_merge_higher_precedence_wins() {
        let mut base = ConfigStore::new();
        base.set("user.name", ConfigValue::String("base".into()));
        base.set("user.email", ConfigValue::String("base@e.com".into()));

        let mut overlay = ConfigStore::new();
        overlay.set("user.name", ConfigValue::String("overlay".into()));
        overlay.set("extra.key", ConfigValue::Integer(1));

        base.merge(&overlay);

        assert_eq!(
            base.get("user.name").and_then(ConfigValue::as_str),
            Some("overlay")
        );
        assert_eq!(
            base.get("user.email").and_then(ConfigValue::as_str),
            Some("base@e.com")
        );
        assert_eq!(base.get("extra.key").and_then(ConfigValue::as_i64), Some(1));
    }

    // ── TOML load/save round-trip (P2-04) ───────────────────────────

    #[test]
    fn store_from_toml_string() {
        let toml = r#"
[user]
name = "trekmax"
email = "t@e.com"

[update]
parallelism = 8
"#;
        let store = ConfigStore::from_toml_str(toml).unwrap();
        assert_eq!(
            store.get("user.name").and_then(ConfigValue::as_str),
            Some("trekmax")
        );
        assert_eq!(
            store.get("user.email").and_then(ConfigValue::as_str),
            Some("t@e.com")
        );
        assert_eq!(
            store
                .get("update.parallelism")
                .and_then(ConfigValue::as_i64),
            Some(8)
        );
    }

    #[test]
    fn store_from_toml_dotted_keys() {
        let toml = r#"
user.name = "trekmax"
user.email = "t@e.com"
"#;
        let store = ConfigStore::from_toml_str(toml).unwrap();
        assert_eq!(
            store.get("user.name").and_then(ConfigValue::as_str),
            Some("trekmax")
        );
    }

    #[test]
    fn store_from_toml_with_bool_and_float() {
        let toml = r"
feature.enabled = true
feature.threshold = 1.5
";
        let store = ConfigStore::from_toml_str(toml).unwrap();
        assert_eq!(
            store.get("feature.enabled").and_then(ConfigValue::as_bool),
            Some(true)
        );
        assert_eq!(
            store.get("feature.threshold").and_then(ConfigValue::as_f64),
            Some(1.5)
        );
    }

    #[test]
    fn store_to_toml_and_back() {
        let mut store = ConfigStore::new();
        store.set("user.name", ConfigValue::String("trekmax".into()));
        store.set("user.email", ConfigValue::String("t@e.com".into()));
        store.set("update.parallelism", ConfigValue::Integer(4));
        store.set("flag", ConfigValue::Boolean(true));

        let toml_str = store.to_toml_string().unwrap();
        let store2 = ConfigStore::from_toml_str(&toml_str).unwrap();

        assert_eq!(
            store2.get("user.name").and_then(ConfigValue::as_str),
            Some("trekmax")
        );
        assert_eq!(
            store2
                .get("update.parallelism")
                .and_then(ConfigValue::as_i64),
            Some(4)
        );
        assert_eq!(
            store2.get("flag").and_then(ConfigValue::as_bool),
            Some(true)
        );
    }

    #[test]
    fn store_save_and_load_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let mut store = ConfigStore::new();
        store.set("user.name", ConfigValue::String("trekmax".into()));
        store.set("level", ConfigValue::Integer(42));

        store.save_to_file(&path).unwrap();
        assert!(path.exists());

        let loaded = ConfigStore::load_from_file(&path).unwrap();
        assert_eq!(
            loaded.get("user.name").and_then(ConfigValue::as_str),
            Some("trekmax")
        );
        assert_eq!(loaded.get("level").and_then(ConfigValue::as_i64), Some(42));
    }

    #[test]
    fn store_load_missing_file_returns_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.toml");

        let store = ConfigStore::load_from_file(&path).unwrap();
        assert!(store.get("anything").is_none());
    }

    #[test]
    fn store_from_toml_invalid_syntax_errors() {
        let toml = "this is not valid toml [[[";
        let result = ConfigStore::from_toml_str(toml);
        assert!(result.is_err());
    }

    // ── Three-layer merge + PathProvider (P2-06/08) ─────────────────

    #[test]
    fn config_load_merges_three_layers() {
        let dir = tempfile::TempDir::new().unwrap();
        let system_path = dir.path().join("system/config.toml");
        let global_path = dir.path().join("global/config.toml");
        let workspace_path = dir.path().join("ws/.east/config.toml");

        // System: lowest precedence
        let mut system = ConfigStore::new();
        system.set("user.name", ConfigValue::String("system-user".into()));
        system.set("update.parallelism", ConfigValue::Integer(2));
        system.set("system.only", ConfigValue::Boolean(true));
        system.save_to_file(&system_path).unwrap();

        // Global: middle precedence
        let mut global = ConfigStore::new();
        global.set("user.name", ConfigValue::String("global-user".into()));
        global.set("user.email", ConfigValue::String("g@e.com".into()));
        global.save_to_file(&global_path).unwrap();

        // Workspace: highest precedence
        let mut ws = ConfigStore::new();
        ws.set("user.name", ConfigValue::String("ws-user".into()));
        ws.save_to_file(&workspace_path).unwrap();

        let paths = TestPathProvider {
            system: Some(system_path),
            global: Some(global_path),
            workspace: Some(workspace_path),
        };
        let config = Config::load_with_provider(&paths).unwrap();

        // Workspace wins for user.name
        assert_eq!(config.get_str("user.name"), Some("ws-user"));
        // Global contributes user.email
        assert_eq!(config.get_str("user.email"), Some("g@e.com"));
        // System contributes update.parallelism
        assert_eq!(config.get_i64("update.parallelism"), Some(2));
        // System contributes system.only
        assert_eq!(config.get_bool("system.only"), Some(true));
    }

    #[test]
    fn config_load_skips_missing_layers() {
        let dir = tempfile::TempDir::new().unwrap();
        let global_path = dir.path().join("global/config.toml");

        let mut global = ConfigStore::new();
        global.set("user.name", ConfigValue::String("global".into()));
        global.save_to_file(&global_path).unwrap();

        let paths = TestPathProvider {
            system: None,
            global: Some(global_path),
            workspace: None,
        };
        let config = Config::load_with_provider(&paths).unwrap();
        assert_eq!(config.get_str("user.name"), Some("global"));
    }

    #[test]
    fn config_load_empty_when_no_files() {
        let paths = TestPathProvider {
            system: None,
            global: None,
            workspace: None,
        };
        let config = Config::load_with_provider(&paths).unwrap();
        assert!(config.get_str("anything").is_none());
    }

    #[test]
    fn config_set_and_save_to_layer() {
        let dir = tempfile::TempDir::new().unwrap();
        let global_path = dir.path().join("global/config.toml");

        let paths = TestPathProvider {
            system: None,
            global: Some(global_path),
            workspace: None,
        };
        let mut config = Config::load_with_provider(&paths).unwrap();

        config.set(
            ConfigLayer::Global,
            "user.name",
            ConfigValue::String("new".into()),
        );
        config.save(&paths, ConfigLayer::Global).unwrap();

        // Reload and verify
        let config2 = Config::load_with_provider(&paths).unwrap();
        assert_eq!(config2.get_str("user.name"), Some("new"));
    }

    #[test]
    fn config_unset_and_save() {
        let dir = tempfile::TempDir::new().unwrap();
        let global_path = dir.path().join("global/config.toml");

        let mut global = ConfigStore::new();
        global.set("user.name", ConfigValue::String("existing".into()));
        global.set("user.email", ConfigValue::String("e@e.com".into()));
        global.save_to_file(&global_path).unwrap();

        let paths = TestPathProvider {
            system: None,
            global: Some(global_path),
            workspace: None,
        };
        let mut config = Config::load_with_provider(&paths).unwrap();

        config.unset(ConfigLayer::Global, "user.name");
        config.save(&paths, ConfigLayer::Global).unwrap();

        let config2 = Config::load_with_provider(&paths).unwrap();
        assert!(config2.get_str("user.name").is_none());
        assert_eq!(config2.get_str("user.email"), Some("e@e.com"));
    }

    #[test]
    fn config_iter_returns_merged_entries() {
        let dir = tempfile::TempDir::new().unwrap();
        let system_path = dir.path().join("system/config.toml");
        let global_path = dir.path().join("global/config.toml");

        let mut system = ConfigStore::new();
        system.set("a", ConfigValue::String("from-system".into()));
        system.save_to_file(&system_path).unwrap();

        let mut global = ConfigStore::new();
        global.set("b", ConfigValue::String("from-global".into()));
        global.save_to_file(&global_path).unwrap();

        let paths = TestPathProvider {
            system: Some(system_path),
            global: Some(global_path),
            workspace: None,
        };
        let config = Config::load_with_provider(&paths).unwrap();
        assert_eq!(config.iter().count(), 2);
    }

    /// Test-only `PathProvider` implementation.
    struct TestPathProvider {
        system: Option<std::path::PathBuf>,
        global: Option<std::path::PathBuf>,
        workspace: Option<std::path::PathBuf>,
    }

    impl PathProvider for TestPathProvider {
        fn system_config_path(&self) -> Option<std::path::PathBuf> {
            self.system.clone()
        }
        fn global_config_path(&self) -> Option<std::path::PathBuf> {
            self.global.clone()
        }
        fn workspace_config_path(&self) -> Option<std::path::PathBuf> {
            self.workspace.clone()
        }
    }
}
