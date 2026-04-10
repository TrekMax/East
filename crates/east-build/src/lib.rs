#![forbid(unsafe_code)]
//! `CMake` wrapper for east.
//!
//! Provides build directory management, `CMake` preset support, pristine
//! detection, and build artifact tracking.

pub mod cmake;
pub mod error;

#[cfg(test)]
mod tests {
    use super::cmake::{detect_cmake, parse_cmake_version};

    #[test]
    fn parse_cmake_version_standard() {
        let v = parse_cmake_version("cmake version 3.28.1").unwrap();
        assert_eq!(v, (3, 28, 1));
    }

    #[test]
    fn parse_cmake_version_with_extra_lines() {
        let output = "cmake version 3.21.0\n\nCMake suite maintained and supported by Kitware\n";
        let v = parse_cmake_version(output).unwrap();
        assert_eq!(v, (3, 21, 0));
    }

    #[test]
    fn parse_cmake_version_pre_release() {
        let v = parse_cmake_version("cmake version 3.30.0-rc1");
        // Should parse the numeric part
        assert!(v.is_some());
        let (major, minor, _) = v.unwrap();
        assert_eq!(major, 3);
        assert_eq!(minor, 30);
    }

    #[test]
    fn parse_cmake_version_garbage_returns_none() {
        assert!(parse_cmake_version("not cmake output").is_none());
    }

    #[tokio::test]
    async fn detect_cmake_finds_system_cmake() {
        // This test assumes cmake is installed on the CI host.
        // If cmake is not available, the test verifies the error is clean.
        match detect_cmake().await {
            Ok((major, _minor, _patch)) => {
                assert!(major >= 3, "CMake major version should be >= 3");
            }
            Err(e) => {
                // cmake not installed — acceptable in some environments
                assert!(
                    e.to_string().contains("not found") || e.to_string().contains("cmake"),
                    "error should mention cmake: {e}"
                );
            }
        }
    }

    // ── Build directory resolution ──────────────────────────────────

    use super::cmake::{
        discover_artifacts, resolve_build_dir, resolve_source_dir, should_clean_auto,
    };

    #[test]
    fn build_dir_default() {
        let ws = std::path::Path::new("/ws");
        let dir = resolve_build_dir(ws, None, None);
        assert_eq!(dir, std::path::PathBuf::from("/ws/build/default"));
    }

    #[test]
    fn build_dir_with_preset() {
        let ws = std::path::Path::new("/ws");
        let dir = resolve_build_dir(ws, None, Some("my-preset"));
        assert_eq!(dir, std::path::PathBuf::from("/ws/build/my-preset"));
    }

    #[test]
    fn build_dir_explicit_override() {
        let ws = std::path::Path::new("/ws");
        let dir = resolve_build_dir(ws, Some(std::path::Path::new("/custom/build")), Some("x"));
        assert_eq!(dir, std::path::PathBuf::from("/custom/build"));
    }

    // ── Source directory resolution ──────────────────────────────────

    #[test]
    fn source_dir_workspace_root_fallback() {
        let dir = tempfile::TempDir::new().unwrap();
        let result = resolve_source_dir(dir.path(), None, None).unwrap();
        assert_eq!(result, dir.path().to_path_buf());
    }

    #[test]
    fn source_dir_app_preferred_if_exists() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("app")).unwrap();
        let result = resolve_source_dir(dir.path(), None, None).unwrap();
        assert_eq!(result, dir.path().join("app"));
    }

    #[test]
    fn source_dir_explicit_override() {
        let dir = tempfile::TempDir::new().unwrap();
        let src = dir.path().join("my-src");
        std::fs::create_dir_all(&src).unwrap();
        let result = resolve_source_dir(dir.path(), Some(&src), None).unwrap();
        assert_eq!(result, src);
    }

    #[test]
    fn source_dir_explicit_missing_errors() {
        let dir = tempfile::TempDir::new().unwrap();
        let missing = dir.path().join("nope");
        let err = resolve_source_dir(dir.path(), Some(&missing), None).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    // ── Pristine auto ───────────────────────────────────────────────

    #[test]
    fn pristine_auto_no_cache_returns_false() {
        let dir = tempfile::TempDir::new().unwrap();
        let build_dir = dir.path().join("build");
        std::fs::create_dir_all(&build_dir).unwrap();
        assert!(!should_clean_auto(&build_dir, dir.path(), None));
    }

    #[test]
    fn pristine_auto_source_dir_changed_returns_true() {
        let dir = tempfile::TempDir::new().unwrap();
        let build_dir = dir.path().join("build");
        std::fs::create_dir_all(&build_dir).unwrap();
        std::fs::write(
            build_dir.join("CMakeCache.txt"),
            "CMAKE_HOME_DIRECTORY:INTERNAL=/old/source\n",
        )
        .unwrap();
        assert!(should_clean_auto(&build_dir, dir.path(), None));
    }

    // ── Artifact discovery ──────────────────────────────────────────

    #[test]
    fn discover_artifacts_finds_elf() {
        let dir = tempfile::TempDir::new().unwrap();
        let build_dir = dir.path().join("build");
        std::fs::create_dir_all(build_dir.join("app")).unwrap();
        std::fs::write(build_dir.join("app/firmware.elf"), "fake elf").unwrap();

        let (elf, bin, hex) = discover_artifacts(&build_dir);
        assert!(elf.is_some());
        assert!(bin.is_none());
        assert!(hex.is_none());
    }

    #[test]
    fn discover_artifacts_empty_build_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let build_dir = dir.path().join("build");
        std::fs::create_dir_all(&build_dir).unwrap();

        let (elf, bin, hex) = discover_artifacts(&build_dir);
        assert!(elf.is_none());
        assert!(bin.is_none());
        assert!(hex.is_none());
    }
}
