use std::fs;
use std::path::Path;

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current schema version for `.east/state.toml`.
const SCHEMA_VERSION: u32 = 1;

/// Errors from state operations.
#[derive(Debug, Error, Diagnostic)]
#[allow(clippy::module_name_repetitions)]
pub enum StateError {
    /// Schema version in the file does not match the expected version.
    #[error(
        "state.toml schema version mismatch: found {found}, expected {expected}. Delete .east/state.toml and rebuild."
    )]
    SchemaVersionMismatch {
        /// The version found in the file.
        found: u32,
        /// The version expected by this code.
        expected: u32,
    },

    /// TOML parse error.
    #[error("failed to parse state.toml: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization error.
    #[error("failed to serialize state.toml: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// I/O error.
    #[error("state I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Build state persisted across invocations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct BuildState {
    /// Last build directory used.
    #[serde(default)]
    pub last_build_dir: String,
    /// Last preset name.
    #[serde(default)]
    pub last_preset: String,
    /// Last source directory (absolute path).
    #[serde(default)]
    pub last_source_dir: String,
    /// Last discovered elf artifact path.
    #[serde(default)]
    pub last_elf: String,
    /// Last discovered bin artifact path.
    #[serde(default)]
    pub last_bin: String,
    /// Last discovered hex artifact path.
    #[serde(default)]
    pub last_hex: String,
    /// Timestamp of last successful configure (RFC 3339).
    #[serde(default)]
    pub last_configured_at: String,
}

/// Runner state persisted across invocations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct RunnerState {
    /// Default runner name.
    #[serde(default, rename = "default")]
    pub default_runner: String,
}

/// Persistent workspace state stored in `.east/state.toml`.
///
/// Created by `east init`, updated by `east build`, read by
/// `east flash`/`east debug`/`east attach`/`east reset`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Build-related state.
    #[serde(default)]
    pub build: BuildState,
    /// Runner-related state.
    #[serde(default)]
    pub runner: RunnerState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            build: BuildState::default(),
            runner: RunnerState::default(),
        }
    }
}

impl State {
    /// Parse state from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::TomlParse`] on parse failure, or
    /// [`StateError::SchemaVersionMismatch`] if the version doesn't match.
    pub fn from_toml_str(toml_str: &str) -> Result<Self, StateError> {
        let state: Self = toml::from_str(toml_str)?;
        if state.schema_version != SCHEMA_VERSION {
            return Err(StateError::SchemaVersionMismatch {
                found: state.schema_version,
                expected: SCHEMA_VERSION,
            });
        }
        Ok(state)
    }

    /// Serialize state to a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::TomlSerialize`] on failure.
    pub fn to_toml_string(&self) -> Result<String, StateError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Load state from a file. Returns `State::default()` if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`StateError`] on I/O errors, parse errors, or version mismatch.
    pub fn load_from_file(path: &Path) -> Result<Self, StateError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        Self::from_toml_str(&content)
    }

    /// Save state to a file atomically (write to .tmp, then rename).
    ///
    /// Creates parent directories as needed.
    ///
    /// # Errors
    ///
    /// Returns [`StateError`] on I/O or serialization errors.
    pub fn save_to_file(&self, path: &Path) -> Result<(), StateError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = self.to_toml_string()?;
        let tmp_path = path.with_extension("toml.tmp");
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }
}
