use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::ConfigError;
use crate::value::ConfigValue;

/// An in-memory configuration store using nested `BTreeMap`s.
///
/// Keys are dotted paths (e.g. `"user.name"`). Internally the store
/// is a tree of `Node`s — branches hold child nodes, leaves hold values.
///
/// # Example
///
/// ```
/// use east_config::{ConfigStore, ConfigValue};
///
/// let mut store = ConfigStore::new();
/// store.set("user.name", ConfigValue::String("alice".into()));
/// store.set("update.jobs", ConfigValue::Integer(4));
///
/// assert_eq!(store.get("user.name").and_then(|v| v.as_str()), Some("alice"));
/// assert_eq!(store.get("update.jobs").and_then(|v| v.as_i64()), Some(4));
/// ```
#[derive(Debug, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct ConfigStore {
    root: BTreeMap<String, Node>,
}

#[derive(Debug, Clone)]
enum Node {
    Leaf(ConfigValue),
    Branch(BTreeMap<String, Node>),
}

impl ConfigStore {
    /// Create an empty store.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: BTreeMap::new(),
        }
    }

    /// Get a value by dotted key (e.g. `"user.name"`).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        let segments: Vec<&str> = key.split('.').collect();
        Self::get_in(&self.root, &segments)
    }

    /// Set a value at a dotted key, creating intermediate nodes as needed.
    pub fn set(&mut self, key: &str, value: ConfigValue) {
        let segments: Vec<&str> = key.split('.').collect();
        Self::set_in(&mut self.root, &segments, value);
    }

    /// Remove a value at a dotted key. No-op if the key does not exist.
    pub fn unset(&mut self, key: &str) {
        let segments: Vec<&str> = key.split('.').collect();
        Self::unset_in(&mut self.root, &segments);
    }

    /// Iterate over all leaf entries as `(dotted_key, value)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (String, &ConfigValue)> {
        let mut result = Vec::new();
        Self::collect_leaves(&self.root, "", &mut result);
        result.into_iter()
    }

    /// Merge another store into this one. Values from `other` override
    /// values in `self` on a per-key basis (deep merge).
    pub fn merge(&mut self, other: &Self) {
        Self::merge_maps(&mut self.root, &other.root);
    }

    /// Parse a TOML string into a `ConfigStore`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::TomlParse`] if the TOML is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use east_config::ConfigStore;
    ///
    /// let store = ConfigStore::from_toml_str(r#"
    /// [user]
    /// name = "alice"
    /// "#).unwrap();
    /// assert_eq!(store.get("user.name").and_then(|v| v.as_str()), Some("alice"));
    /// ```
    pub fn from_toml_str(toml_str: &str) -> Result<Self, ConfigError> {
        let table: toml::Table = toml::from_str(toml_str)?;
        let mut store = Self::new();
        Self::import_toml_table(&mut store.root, &table);
        Ok(store)
    }

    /// Serialize this store to a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::TomlSerialize`] on serialization failure.
    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        let table = Self::export_toml_table(&self.root);
        Ok(toml::to_string_pretty(&table)?)
    }

    /// Load a `ConfigStore` from a TOML file.
    ///
    /// Returns an empty store if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] on I/O or parse errors.
    pub fn load_from_file(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::from_toml_str(&content)
    }

    /// Save this store to a TOML file, creating parent directories as needed.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] on I/O or serialization errors.
    pub fn save_to_file(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        let content = self.to_toml_string()?;
        fs::write(path, &content).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(())
    }

    // ── private helpers ─────────────────────────────────────────────

    fn get_in<'a>(map: &'a BTreeMap<String, Node>, segments: &[&str]) -> Option<&'a ConfigValue> {
        match segments {
            [] => None,
            [last] => match map.get(*last) {
                Some(Node::Leaf(v)) => Some(v),
                _ => None,
            },
            [head, tail @ ..] => match map.get(*head) {
                Some(Node::Branch(children)) => Self::get_in(children, tail),
                _ => None,
            },
        }
    }

    fn set_in(map: &mut BTreeMap<String, Node>, segments: &[&str], value: ConfigValue) {
        match segments {
            [] => {}
            [last] => {
                map.insert((*last).to_string(), Node::Leaf(value));
            }
            [head, tail @ ..] => {
                let entry = map
                    .entry((*head).to_string())
                    .or_insert_with(|| Node::Branch(BTreeMap::new()));
                match entry {
                    Node::Branch(children) => Self::set_in(children, tail, value),
                    Node::Leaf(_) => {
                        // Overwrite leaf with branch
                        let mut children = BTreeMap::new();
                        Self::set_in(&mut children, tail, value);
                        *entry = Node::Branch(children);
                    }
                }
            }
        }
    }

    fn unset_in(map: &mut BTreeMap<String, Node>, segments: &[&str]) {
        match segments {
            [] => {}
            [last] => {
                map.remove(*last);
            }
            [head, tail @ ..] => {
                if let Some(Node::Branch(children)) = map.get_mut(*head) {
                    Self::unset_in(children, tail);
                }
            }
        }
    }

    fn collect_leaves<'a>(
        map: &'a BTreeMap<String, Node>,
        prefix: &str,
        result: &mut Vec<(String, &'a ConfigValue)>,
    ) {
        for (key, node) in map {
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            match node {
                Node::Leaf(v) => result.push((full_key, v)),
                Node::Branch(children) => {
                    Self::collect_leaves(children, &full_key, result);
                }
            }
        }
    }

    fn merge_maps(base: &mut BTreeMap<String, Node>, overlay: &BTreeMap<String, Node>) {
        for (key, overlay_node) in overlay {
            match (base.get_mut(key), overlay_node) {
                (Some(Node::Branch(base_children)), Node::Branch(overlay_children)) => {
                    Self::merge_maps(base_children, overlay_children);
                }
                _ => {
                    base.insert(key.clone(), overlay_node.clone());
                }
            }
        }
    }

    fn import_toml_table(map: &mut BTreeMap<String, Node>, table: &toml::Table) {
        for (key, value) in table {
            match value {
                toml::Value::Table(sub) => {
                    let mut children = BTreeMap::new();
                    Self::import_toml_table(&mut children, sub);
                    map.insert(key.clone(), Node::Branch(children));
                }
                toml::Value::String(s) => {
                    map.insert(key.clone(), Node::Leaf(ConfigValue::String(s.clone())));
                }
                toml::Value::Integer(i) => {
                    map.insert(key.clone(), Node::Leaf(ConfigValue::Integer(*i)));
                }
                toml::Value::Float(f) => {
                    map.insert(key.clone(), Node::Leaf(ConfigValue::Float(*f)));
                }
                toml::Value::Boolean(b) => {
                    map.insert(key.clone(), Node::Leaf(ConfigValue::Boolean(*b)));
                }
                // Arrays and datetimes are not supported in config; skip silently
                _ => {}
            }
        }
    }

    fn export_toml_table(map: &BTreeMap<String, Node>) -> toml::Table {
        let mut table = toml::Table::new();
        for (key, node) in map {
            match node {
                Node::Leaf(ConfigValue::String(s)) => {
                    table.insert(key.clone(), toml::Value::String(s.clone()));
                }
                Node::Leaf(ConfigValue::Integer(i)) => {
                    table.insert(key.clone(), toml::Value::Integer(*i));
                }
                Node::Leaf(ConfigValue::Float(f)) => {
                    table.insert(key.clone(), toml::Value::Float(*f));
                }
                Node::Leaf(ConfigValue::Boolean(b)) => {
                    table.insert(key.clone(), toml::Value::Boolean(*b));
                }
                Node::Branch(children) => {
                    table.insert(
                        key.clone(),
                        toml::Value::Table(Self::export_toml_table(children)),
                    );
                }
            }
        }
        table
    }
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self::new()
    }
}
