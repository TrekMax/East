#![allow(clippy::doc_markdown)]

use std::path::{Path, PathBuf};

use tokio::process::Command;
use tracing::{debug, warn};

use crate::error::BuildError;

/// Minimum required CMake version (Preset schema v3).
pub const MIN_CMAKE_VERSION: (u32, u32, u32) = (3, 21, 0);

/// Parse a CMake version string like `"cmake version 3.28.1"`.
///
/// Returns `(major, minor, patch)` or `None` if the format is unrecognized.
#[must_use]
pub fn parse_cmake_version(output: &str) -> Option<(u32, u32, u32)> {
    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("cmake version ") {
            let parts: Vec<&str> = rest.split('.').collect();
            if parts.len() >= 3 {
                let major = parts[0].parse().ok()?;
                let minor = parts[1].parse().ok()?;
                // Patch may have suffix like "0-rc1", take only digits
                let patch_str: String = parts[2].chars().take_while(char::is_ascii_digit).collect();
                let patch = patch_str.parse().unwrap_or(0);
                return Some((major, minor, patch));
            }
        }
    }
    None
}

/// Detect CMake on the system and return its version.
///
/// # Errors
///
/// Returns [`BuildError::CmakeNotFound`] if cmake is not on PATH.
#[allow(clippy::module_name_repetitions)]
pub async fn detect_cmake() -> Result<(u32, u32, u32), BuildError> {
    let output = Command::new("cmake")
        .arg("--version")
        .output()
        .await
        .map_err(|_| BuildError::CmakeNotFound)?;

    if !output.status.success() {
        return Err(BuildError::CmakeNotFound);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_cmake_version(&stdout).ok_or(BuildError::CmakeNotFound)
}

/// Check that the detected CMake version meets the minimum requirement.
///
/// # Errors
///
/// Returns [`BuildError::CmakeVersionTooLow`] if the version is below 3.21.0.
pub fn check_cmake_version(version: (u32, u32, u32)) -> Result<(), BuildError> {
    let (major, minor, patch) = version;
    let (req_major, req_minor, req_patch) = MIN_CMAKE_VERSION;

    if major > req_major
        || (major == req_major && minor > req_minor)
        || (major == req_major && minor == req_minor && patch >= req_patch)
    {
        Ok(())
    } else {
        Err(BuildError::CmakeVersionTooLow {
            found: format!("{major}.{minor}.{patch}"),
            required: format!("{req_major}.{req_minor}.{req_patch}"),
        })
    }
}

/// Resolve the build directory.
///
/// If `explicit` is provided, use it. Otherwise, construct
/// `<workspace_root>/build/<preset_name>` (or `default`).
#[must_use]
pub fn resolve_build_dir(
    workspace_root: &Path,
    explicit: Option<&Path>,
    preset: Option<&str>,
) -> PathBuf {
    if let Some(dir) = explicit {
        return dir.to_path_buf();
    }
    let name = preset.unwrap_or("default");
    workspace_root.join("build").join(name)
}

/// Resolve the source directory using the precedence chain.
///
/// 1. `explicit` (--source-dir CLI flag)
/// 2. `config_source_dir` (build.source_dir from config)
/// 3. `<workspace_root>/app` if it exists
/// 4. `<workspace_root>`
///
/// # Errors
///
/// Returns [`BuildError::SourceDirNotFound`] if the resolved directory does not exist.
pub fn resolve_source_dir(
    workspace_root: &Path,
    explicit: Option<&Path>,
    config_source_dir: Option<&str>,
) -> Result<PathBuf, BuildError> {
    if let Some(dir) = explicit {
        if dir.is_dir() {
            return Ok(dir.to_path_buf());
        }
        return Err(BuildError::SourceDirNotFound {
            path: dir.to_path_buf(),
        });
    }

    if let Some(cfg) = config_source_dir {
        let p = PathBuf::from(cfg);
        if p.is_dir() {
            return Ok(p);
        }
        return Err(BuildError::SourceDirNotFound { path: p });
    }

    let app_dir = workspace_root.join("app");
    if app_dir.is_dir() {
        return Ok(app_dir);
    }

    Ok(workspace_root.to_path_buf())
}

/// Pristine strategy for the build directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PristineStrategy {
    /// Always remove build dir before configure.
    Always,
    /// Never remove.
    Never,
    /// Remove if conditions suggest a stale build.
    Auto,
}

/// Check whether the build directory should be cleaned under the `Auto` strategy.
///
/// Returns `true` if the build dir should be removed.
#[must_use]
pub fn should_clean_auto(build_dir: &Path, source_dir: &Path, preset: Option<&str>) -> bool {
    let cache_path = build_dir.join("CMakeCache.txt");
    if !cache_path.exists() {
        // No cache means either first build or previously cleaned
        return false;
    }

    let Ok(cache_content) = std::fs::read_to_string(&cache_path) else {
        warn!("cannot read CMakeCache.txt, treating as stale");
        return true;
    };

    // Check if source directory changed
    if let Some(cached_source) = extract_cmake_cache_value(&cache_content, "CMAKE_HOME_DIRECTORY") {
        let source_str = source_dir.to_string_lossy();
        if cached_source != source_str.as_ref() {
            debug!("source dir changed: {cached_source} -> {source_str}");
            return true;
        }
    }

    // Check if preset changed (we store it as a comment in CMakeCache or check via state)
    // For simplicity in Phase 3: if the user specifies a preset and the build dir
    // already exists with a different CMakeCache, we don't auto-detect preset change.
    // This is a documented limitation.
    let _ = preset;

    false
}

/// Extract a value from `CMakeCache.txt` content.
fn extract_cmake_cache_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        // Format: KEY:TYPE=VALUE or KEY=VALUE
        if let Some(rest) = line.strip_prefix(key) {
            if let Some(value) = rest.strip_prefix('=') {
                return Some(value);
            }
            if let Some(after_type) = rest.strip_prefix(':') {
                if let Some(value) = after_type.split_once('=').map(|(_, v)| v) {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// Search for build artifacts in the build directory.
///
/// Returns paths to the first discovered elf, bin, and hex files.
#[must_use]
pub fn discover_artifacts(build_dir: &Path) -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
    let find = |pattern: &str| -> Option<PathBuf> {
        let full_pattern = format!("{}/{pattern}", build_dir.display());
        glob::glob(&full_pattern)
            .ok()?
            .filter_map(Result::ok)
            .max_by_key(|p| {
                p.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            })
    };

    let elf = find("**/*.elf");
    let bin = find("**/*.bin");
    let hex = find("**/*.hex");

    (elf, bin, hex)
}
