#![forbid(unsafe_code)]
//! `.east/` directory, workspace discovery, and state for east.

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
        assert_eq!(ws.east_dir(), dir.path().canonicalize().unwrap().join(".east"));
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
}
