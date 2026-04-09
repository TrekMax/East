#![allow(clippy::doc_markdown)]

use crate::error::ConfigError;
use crate::path::PathProvider;
use crate::store::ConfigStore;
use crate::value::ConfigValue;

/// Which configuration layer to target for writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
pub enum ConfigLayer {
    /// System-wide config (e.g. `/etc/east/config.toml`).
    System,
    /// Per-user global config (e.g. `~/.config/east/config.toml`).
    Global,
    /// Per-workspace config (e.g. `<root>/.east/config.toml`).
    Workspace,
}

/// Layered configuration that merges system, global, and workspace TOML files.
///
/// # Example
///
/// ```no_run
/// use east_config::{Config, ConfigLayer, ConfigValue};
/// use east_config::path::DefaultPathProvider;
///
/// let provider = DefaultPathProvider::new(Some("/my/workspace".into()));
/// let config = Config::load_with_provider(&provider).unwrap();
/// if let Some(name) = config.get_str("user.name") {
///     println!("Hello, {name}!");
/// }
/// ```
pub struct Config {
    /// Per-layer stores, kept separate for targeted writes.
    system: ConfigStore,
    global: ConfigStore,
    workspace: ConfigStore,
    /// Merged view (system + global + workspace).
    merged: ConfigStore,
}

impl Config {
    /// Load configuration from all layers using the given path provider.
    ///
    /// Missing files are silently skipped (the layer is an empty store).
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if a file exists but cannot be parsed.
    pub fn load_with_provider(paths: &dyn PathProvider) -> Result<Self, ConfigError> {
        let system = match paths.system_config_path() {
            Some(p) => ConfigStore::load_from_file(&p)?,
            None => ConfigStore::new(),
        };
        let global = match paths.global_config_path() {
            Some(p) => ConfigStore::load_from_file(&p)?,
            None => ConfigStore::new(),
        };
        let workspace = match paths.workspace_config_path() {
            Some(p) => ConfigStore::load_from_file(&p)?,
            None => ConfigStore::new(),
        };

        let mut merged = system.clone();
        merged.merge(&global);
        merged.merge(&workspace);

        Ok(Self {
            system,
            global,
            workspace,
            merged,
        })
    }

    /// Get a value from the merged config by dotted key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        self.merged.get(key)
    }

    /// Get a string value from the merged config.
    #[must_use]
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.merged.get(key).and_then(ConfigValue::as_str)
    }

    /// Get an integer value from the merged config.
    #[must_use]
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.merged.get(key).and_then(ConfigValue::as_i64)
    }

    /// Get a boolean value from the merged config.
    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.merged.get(key).and_then(ConfigValue::as_bool)
    }

    /// Get a float value from the merged config.
    #[must_use]
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.merged.get(key).and_then(ConfigValue::as_f64)
    }

    /// Set a value in a specific layer and update the merged view.
    pub fn set(&mut self, layer: ConfigLayer, key: &str, value: ConfigValue) {
        self.layer_mut(layer).set(key, value);
        self.rebuild_merged();
    }

    /// Remove a value from a specific layer and update the merged view.
    pub fn unset(&mut self, layer: ConfigLayer, key: &str) {
        self.layer_mut(layer).unset(key);
        self.rebuild_merged();
    }

    /// Save a specific layer to disk.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if the path is `None` or I/O fails.
    pub fn save(&self, paths: &dyn PathProvider, layer: ConfigLayer) -> Result<(), ConfigError> {
        let path = match layer {
            ConfigLayer::System => paths.system_config_path(),
            ConfigLayer::Global => paths.global_config_path(),
            ConfigLayer::Workspace => paths.workspace_config_path(),
        };
        if let Some(p) = path {
            self.layer_ref(layer).save_to_file(&p)?;
        }
        Ok(())
    }

    /// Iterate over all merged leaf entries.
    pub fn iter(&self) -> impl Iterator<Item = (String, &ConfigValue)> {
        self.merged.iter()
    }

    fn layer_mut(&mut self, layer: ConfigLayer) -> &mut ConfigStore {
        match layer {
            ConfigLayer::System => &mut self.system,
            ConfigLayer::Global => &mut self.global,
            ConfigLayer::Workspace => &mut self.workspace,
        }
    }

    const fn layer_ref(&self, layer: ConfigLayer) -> &ConfigStore {
        match layer {
            ConfigLayer::System => &self.system,
            ConfigLayer::Global => &self.global,
            ConfigLayer::Workspace => &self.workspace,
        }
    }

    fn rebuild_merged(&mut self) {
        let mut merged = self.system.clone();
        merged.merge(&self.global);
        merged.merge(&self.workspace);
        self.merged = merged;
    }
}
