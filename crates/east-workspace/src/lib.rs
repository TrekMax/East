#![forbid(unsafe_code)]
//! `.east/` directory, workspace discovery, and state for east.

pub mod error;
pub mod state;
mod workspace;

pub use state::State;
pub use workspace::Workspace;

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn discover_from_workspace_root() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".east")).unwrap();

        let ws = Workspace::discover(dir.path()).unwrap();
        assert_eq!(ws.root(), dir.path().canonicalize().unwrap());
    }

    #[test]
    fn discover_from_subdirectory() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".east")).unwrap();
        let sub = dir.path().join("a/b/c");
        fs::create_dir_all(&sub).unwrap();

        let ws = Workspace::discover(&sub).unwrap();
        assert_eq!(ws.root(), dir.path().canonicalize().unwrap());
    }

    #[test]
    fn discover_fails_when_no_east_dir() {
        let dir = TempDir::new().unwrap();
        let err = Workspace::discover(dir.path()).unwrap_err();
        assert!(
            err.to_string().contains("workspace"),
            "error should mention workspace: {err}"
        );
    }

    #[test]
    fn init_creates_east_directory() {
        let dir = TempDir::new().unwrap();
        assert!(!dir.path().join(".east").exists());

        Workspace::init(dir.path()).unwrap();

        assert!(dir.path().join(".east").exists());
        assert!(dir.path().join(".east").is_dir());
    }

    #[test]
    fn init_is_idempotent() {
        let dir = TempDir::new().unwrap();
        Workspace::init(dir.path()).unwrap();
        Workspace::init(dir.path()).unwrap(); // should not fail
        assert!(dir.path().join(".east").is_dir());
    }

    #[test]
    fn east_dir_path() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".east")).unwrap();

        let ws = Workspace::discover(dir.path()).unwrap();
        assert_eq!(
            ws.east_dir(),
            dir.path().canonicalize().unwrap().join(".east")
        );
    }

    #[test]
    fn manifest_path() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".east")).unwrap();

        let ws = Workspace::discover(dir.path()).unwrap();
        assert_eq!(
            ws.manifest_path(),
            dir.path().canonicalize().unwrap().join("east.yml")
        );
    }

    // ── State module ────────────────────────────────────────────────

    #[test]
    fn state_default_has_schema_version_1() {
        let state = State::default();
        assert_eq!(state.schema_version, 1);
        assert!(state.build.last_build_dir.is_empty());
        assert!(state.build.last_elf.is_empty());
        assert!(state.runner.default_runner.is_empty());
    }

    #[test]
    fn state_round_trip_toml() {
        let mut state = State::default();
        state.build.last_build_dir = "build/default".into();
        state.build.last_preset = "default".into();
        state.build.last_elf = "build/default/fw.elf".into();
        state.runner.default_runner = "wch-link".into();

        let toml_str = state.to_toml_string().unwrap();
        let loaded = State::from_toml_str(&toml_str).unwrap();

        assert_eq!(loaded.schema_version, 1);
        assert_eq!(loaded.build.last_build_dir, "build/default");
        assert_eq!(loaded.build.last_elf, "build/default/fw.elf");
        assert_eq!(loaded.runner.default_runner, "wch-link");
    }

    #[test]
    fn state_save_and_load_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.toml");

        let mut state = State::default();
        state.build.last_preset = "mypreset".into();

        state.save_to_file(&path).unwrap();
        assert!(path.exists());

        let loaded = State::load_from_file(&path).unwrap();
        assert_eq!(loaded.build.last_preset, "mypreset");
    }

    #[test]
    fn state_atomic_write_no_leftover_tmp() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.toml");
        let tmp_path = dir.path().join("state.toml.tmp");

        let state = State::default();
        state.save_to_file(&path).unwrap();

        assert!(path.exists());
        assert!(!tmp_path.exists());
    }

    #[test]
    fn state_load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.toml");

        let state = State::load_from_file(&path).unwrap();
        assert_eq!(state.schema_version, 1);
        assert!(state.build.last_build_dir.is_empty());
    }

    #[test]
    fn state_schema_version_mismatch_errors() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.toml");
        fs::write(&path, "schema_version = 99\n").unwrap();

        let err = State::load_from_file(&path).unwrap_err();
        assert!(
            err.to_string().contains("schema") || err.to_string().contains("version"),
            "error should mention schema version: {err}"
        );
    }

    #[test]
    fn state_invalid_toml_errors() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.toml");
        fs::write(&path, "not valid toml [[[").unwrap();

        let err = State::load_from_file(&path).unwrap_err();
        assert!(err.to_string().contains("parse") || err.to_string().contains("TOML"));
    }
}
