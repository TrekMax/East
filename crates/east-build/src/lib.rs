#![forbid(unsafe_code)]
//! `CMake` wrapper for east.
//!
//! Provides build directory management, CMake preset support, pristine
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
            Ok((major, minor, _patch)) => {
                assert!(major >= 3, "CMake major version should be >= 3");
                assert!(minor >= 0);
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
}
