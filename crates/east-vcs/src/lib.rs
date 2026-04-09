#![forbid(unsafe_code)]
//! Git operations via shell-out to system `git` for east.

pub mod error;
mod git;

pub use git::Git;

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    use tempfile::TempDir;

    use super::*;

    /// Create a non-bare repo with one commit on a branch.
    fn create_repo_with_commit(dir: &Path, branch: &str) {
        let d = dir.to_str().unwrap();

        Command::new("git")
            .args(["init", "-b", branch])
            .arg(dir)
            .output()
            .expect("git init failed");

        // Configure user for commit (disable signing for test repos)
        for (key, val) in [
            ("user.email", "test@test.com"),
            ("user.name", "Test"),
            ("commit.gpgsign", "false"),
        ] {
            Command::new("git")
                .args(["-C", d, "config", key, val])
                .output()
                .unwrap_or_else(|_| panic!("git config {key} failed"));
        }

        // Create a file and commit
        std::fs::write(dir.join("README.md"), "# test\n").unwrap();
        Command::new("git")
            .args(["-C", d, "add", "."])
            .output()
            .expect("git add failed");
        let out = Command::new("git")
            .args(["-C", d, "commit", "-m", "initial commit"])
            .output()
            .expect("git commit failed");
        assert!(
            out.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // ── Clone ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn clone_repo() {
        let remote_dir = TempDir::new().unwrap();
        create_repo_with_commit(remote_dir.path(), "main");

        let work_dir = TempDir::new().unwrap();
        let clone_path = work_dir.path().join("cloned");

        Git::clone(
            remote_dir.path().to_str().unwrap(),
            &clone_path,
            Some("main"),
        )
        .await
        .unwrap();

        assert!(clone_path.join("README.md").exists());
    }

    #[tokio::test]
    async fn clone_nonexistent_remote_fails() {
        let work_dir = TempDir::new().unwrap();
        let clone_path = work_dir.path().join("cloned");

        let result = Git::clone("/nonexistent/repo", &clone_path, Some("main")).await;
        assert!(result.is_err());
    }

    // ── Fetch ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn fetch_updates() {
        let remote_dir = TempDir::new().unwrap();
        create_repo_with_commit(remote_dir.path(), "main");

        let work_dir = TempDir::new().unwrap();
        let clone_path = work_dir.path().join("cloned");
        Git::clone(
            remote_dir.path().to_str().unwrap(),
            &clone_path,
            Some("main"),
        )
        .await
        .unwrap();

        // Fetch should succeed (no new commits, but no error)
        Git::fetch(&clone_path).await.unwrap();
    }

    // ── Checkout ────────────────────────────────────────────────────

    #[tokio::test]
    async fn checkout_branch() {
        let remote_dir = TempDir::new().unwrap();
        create_repo_with_commit(remote_dir.path(), "main");

        let work_dir = TempDir::new().unwrap();
        let clone_path = work_dir.path().join("cloned");
        Git::clone(
            remote_dir.path().to_str().unwrap(),
            &clone_path,
            Some("main"),
        )
        .await
        .unwrap();

        Git::checkout(&clone_path, "main").await.unwrap();
    }

    // ── HEAD / branch queries ───────────────────────────────────────

    #[tokio::test]
    async fn head_returns_sha() {
        let remote_dir = TempDir::new().unwrap();
        create_repo_with_commit(remote_dir.path(), "main");

        let work_dir = TempDir::new().unwrap();
        let clone_path = work_dir.path().join("cloned");
        Git::clone(
            remote_dir.path().to_str().unwrap(),
            &clone_path,
            Some("main"),
        )
        .await
        .unwrap();

        let sha = Git::head(&clone_path).await.unwrap();
        assert_eq!(sha.len(), 40, "SHA should be 40 hex chars: {sha}");
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn current_branch_name() {
        let remote_dir = TempDir::new().unwrap();
        create_repo_with_commit(remote_dir.path(), "main");

        let work_dir = TempDir::new().unwrap();
        let clone_path = work_dir.path().join("cloned");
        Git::clone(
            remote_dir.path().to_str().unwrap(),
            &clone_path,
            Some("main"),
        )
        .await
        .unwrap();

        let branch = Git::current_branch(&clone_path).await.unwrap();
        assert_eq!(branch, "main");
    }

    // ── Dirty check ─────────────────────────────────────────────────

    #[tokio::test]
    async fn clean_repo_is_not_dirty() {
        let remote_dir = TempDir::new().unwrap();
        create_repo_with_commit(remote_dir.path(), "main");

        let work_dir = TempDir::new().unwrap();
        let clone_path = work_dir.path().join("cloned");
        Git::clone(
            remote_dir.path().to_str().unwrap(),
            &clone_path,
            Some("main"),
        )
        .await
        .unwrap();

        assert!(!Git::is_dirty(&clone_path).await.unwrap());
    }

    #[tokio::test]
    async fn modified_repo_is_dirty() {
        let remote_dir = TempDir::new().unwrap();
        create_repo_with_commit(remote_dir.path(), "main");

        let work_dir = TempDir::new().unwrap();
        let clone_path = work_dir.path().join("cloned");
        Git::clone(
            remote_dir.path().to_str().unwrap(),
            &clone_path,
            Some("main"),
        )
        .await
        .unwrap();

        // Modify a file
        std::fs::write(clone_path.join("README.md"), "modified\n").unwrap();

        assert!(Git::is_dirty(&clone_path).await.unwrap());
    }
}
