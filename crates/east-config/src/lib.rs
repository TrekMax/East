#![forbid(unsafe_code)]
//! Three-layer TOML configuration for east.
//!
//! Provides a layered configuration system that merges system, global,
//! and workspace-level TOML files. Higher-precedence layers override
//! lower ones on a per-key basis.

mod store;
mod value;

pub use store::ConfigStore;
pub use value::ConfigValue;

#[cfg(test)]
mod tests {
    use super::*;

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
}
