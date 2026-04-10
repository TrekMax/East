//! Tests for `ManifestConfig` — the `[manifest]` section in config.

use east_config::manifest_config::ManifestConfig;

#[test]
fn manifest_config_from_store_with_both_fields() {
    let mut store = east_config::ConfigStore::new();
    store.set("manifest.path", east_config::ConfigValue::String("my-app".into()));
    store.set(
        "manifest.file",
        east_config::ConfigValue::String("east.yml".into()),
    );

    let mc = ManifestConfig::from_store(&store).unwrap();
    assert_eq!(mc.path(), "my-app");
    assert_eq!(mc.file(), "east.yml");
}

#[test]
fn manifest_config_file_defaults_to_east_yml() {
    let mut store = east_config::ConfigStore::new();
    store.set("manifest.path", east_config::ConfigValue::String("sdk".into()));

    let mc = ManifestConfig::from_store(&store).unwrap();
    assert_eq!(mc.path(), "sdk");
    assert_eq!(mc.file(), "east.yml");
}

#[test]
fn manifest_config_missing_path_errors() {
    let store = east_config::ConfigStore::new();
    let err = ManifestConfig::from_store(&store).unwrap_err();
    assert!(
        err.to_string().contains("manifest")
            || err.to_string().contains("older")
            || err.to_string().contains("missing"),
        "error should mention missing manifest section: {err}"
    );
}

#[test]
fn manifest_config_rejects_absolute_path() {
    let mut store = east_config::ConfigStore::new();
    store.set(
        "manifest.path",
        east_config::ConfigValue::String("/abs/path".into()),
    );
    let err = ManifestConfig::from_store(&store).unwrap_err();
    assert!(
        err.to_string().contains("absolute") || err.to_string().contains("relative"),
        "error should mention absolute path: {err}"
    );
}

#[test]
fn manifest_config_rejects_dotdot() {
    let mut store = east_config::ConfigStore::new();
    store.set(
        "manifest.path",
        east_config::ConfigValue::String("../escape".into()),
    );
    let err = ManifestConfig::from_store(&store).unwrap_err();
    assert!(
        err.to_string().contains(".."),
        "error should mention dotdot: {err}"
    );
}

#[test]
fn manifest_config_rejects_empty_path() {
    let mut store = east_config::ConfigStore::new();
    store.set("manifest.path", east_config::ConfigValue::String(String::new()));
    let err = ManifestConfig::from_store(&store).unwrap_err();
    assert!(
        err.to_string().contains("empty"),
        "error should mention empty: {err}"
    );
}

#[test]
fn manifest_config_to_store_roundtrip() {
    let mc = ManifestConfig::new("my-app", "east.yml");
    let mut store = east_config::ConfigStore::new();
    mc.write_to_store(&mut store);

    let mc2 = ManifestConfig::from_store(&store).unwrap();
    assert_eq!(mc2.path(), "my-app");
    assert_eq!(mc2.file(), "east.yml");
}
