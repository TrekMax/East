#![allow(clippy::doc_markdown)]

use std::path::Path;

use tokio::process::Command;

use crate::error::VcsError;

/// Shell-out wrapper for system `git` operations.
///
/// All methods are async and call the `git` binary as a child process.
/// No `libgit2` or `git2-rs` binding is used.
pub struct Git;

impl Git {
    /// Clone a repository into `dest`.
    ///
    /// If `revision` is provided and looks like a branch/tag name, clones only
    /// that branch with `--single-branch -b`. If `revision` looks like a full
    /// hex SHA, clones without `--single-branch` and then checks out the SHA.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if the git command fails.
    pub async fn clone(url: &str, dest: &Path, revision: Option<&str>) -> Result<(), VcsError> {
        let is_sha =
            revision.is_some_and(|r| r.len() >= 40 && r.chars().all(|c| c.is_ascii_hexdigit()));

        let mut cmd = Command::new("git");
        cmd.arg("clone");
        if let Some(rev) = revision {
            if !is_sha {
                cmd.args(["--single-branch", "-b", rev]);
            }
        }
        cmd.arg(url);
        cmd.arg(dest);

        run_git(cmd, dest).await?;

        // For SHA revisions, checkout the specific commit after cloning
        if is_sha {
            if let Some(rev) = revision {
                Self::checkout(dest, rev).await?;
            }
        }

        Ok(())
    }

    /// Initialize a git repo in an existing non-empty directory, add the
    /// remote, fetch, and checkout.
    ///
    /// This handles the case where sibling project clones have already created
    /// subdirectories inside the target path, so `git clone` would refuse.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if any git command fails.
    pub async fn init_and_fetch(
        url: &str,
        dest: &Path,
        revision: Option<&str>,
    ) -> Result<(), VcsError> {
        // git init
        let mut cmd = Command::new("git");
        cmd.arg("init");
        cmd.arg(dest);
        run_git(cmd, dest).await?;

        // git remote add origin <url>
        let mut cmd = Command::new("git");
        cmd.args(["-C"]);
        cmd.arg(dest);
        cmd.args(["remote", "add", "origin", url]);
        run_git(cmd, dest).await?;

        // git fetch origin
        Self::fetch(dest).await?;

        // git checkout
        if let Some(rev) = revision {
            Self::checkout(dest, rev).await?;
        }

        Ok(())
    }

    /// Fetch from origin in the repository at `repo_path`.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if the git command fails.
    pub async fn fetch(repo_path: &Path) -> Result<(), VcsError> {
        let mut cmd = Command::new("git");
        cmd.args(["-C"]);
        cmd.arg(repo_path);
        cmd.args(["fetch", "origin"]);

        run_git(cmd, repo_path).await
    }

    /// Checkout a specific revision in the repository at `repo_path`.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if the git command fails.
    pub async fn checkout(repo_path: &Path, revision: &str) -> Result<(), VcsError> {
        let mut cmd = Command::new("git");
        cmd.args(["-C"]);
        cmd.arg(repo_path);
        cmd.args(["checkout", revision]);

        run_git(cmd, repo_path).await
    }

    /// Get the current HEAD SHA (full 40-character hex).
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if the git command fails.
    pub async fn head(repo_path: &Path) -> Result<String, VcsError> {
        let mut cmd = Command::new("git");
        cmd.args(["-C"]);
        cmd.arg(repo_path);
        cmd.args(["rev-parse", "HEAD"]);

        run_git_output(cmd, repo_path).await
    }

    /// Get the current branch name.
    ///
    /// Returns `"HEAD"` if in detached HEAD state.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if the git command fails.
    pub async fn current_branch(repo_path: &Path) -> Result<String, VcsError> {
        let mut cmd = Command::new("git");
        cmd.args(["-C"]);
        cmd.arg(repo_path);
        cmd.args(["rev-parse", "--abbrev-ref", "HEAD"]);

        run_git_output(cmd, repo_path).await
    }

    /// Check whether the working tree has uncommitted changes.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if the git command fails.
    pub async fn is_dirty(repo_path: &Path) -> Result<bool, VcsError> {
        let mut cmd = Command::new("git");
        cmd.args(["-C"]);
        cmd.arg(repo_path);
        cmd.args(["status", "--porcelain"]);

        let output = run_git_output(cmd, repo_path).await?;
        Ok(!output.is_empty())
    }

    /// Sparse-checkout a single file from a remote repository.
    ///
    /// Uses `--depth 1 --filter=blob:none --sparse` to avoid downloading
    /// the full repo, then `sparse-checkout set` to fetch only the
    /// requested file.
    ///
    /// When `revision` is provided, only that branch or tag is fetched
    /// (`--single-branch --branch`). Commit SHAs are **not** supported
    /// — use [`Git::clone`] for SHA-based checkouts.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if any git command fails.
    pub async fn fetch_file(
        url: &str,
        file: &str,
        dest: &Path,
        revision: Option<&str>,
    ) -> Result<(), VcsError> {
        let mut cmd = Command::new("git");
        cmd.args(["clone", "--depth", "1", "--filter=blob:none", "--sparse"]);
        if let Some(rev) = revision {
            cmd.args(["--single-branch", "--branch", rev]);
        }
        cmd.arg(url);
        cmd.arg(dest);
        run_git(cmd, dest).await?;

        // git -C <dest> sparse-checkout set --no-cone <file>
        // --no-cone is required to match individual files (cone mode only matches directories)
        let mut cmd = Command::new("git");
        cmd.args(["-C"]);
        cmd.arg(dest);
        cmd.args(["sparse-checkout", "set", "--no-cone", file]);
        run_git(cmd, dest).await?;

        Ok(())
    }

    /// Get the remote URL for origin.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if the git command fails.
    pub async fn remote_url(repo_path: &Path) -> Result<String, VcsError> {
        let mut cmd = Command::new("git");
        cmd.args(["-C"]);
        cmd.arg(repo_path);
        cmd.args(["remote", "get-url", "origin"]);

        run_git_output(cmd, repo_path).await
    }
}

/// Run a git command, returning `Ok(())` on success or an error with stderr.
async fn run_git(mut cmd: Command, context_path: &Path) -> Result<(), VcsError> {
    let output = cmd.output().await?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(VcsError::GitFailed {
            path: context_path.to_path_buf(),
            stderr,
        })
    }
}

/// Run a git command and return its trimmed stdout on success.
async fn run_git_output(mut cmd: Command, context_path: &Path) -> Result<String, VcsError> {
    let output = cmd.output().await?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(VcsError::GitFailed {
            path: context_path.to_path_buf(),
            stderr,
        })
    }
}
