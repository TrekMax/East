#![forbid(unsafe_code)]
//! CLI entrypoint for east.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use east_manifest::Manifest;
use east_vcs::Git;
use east_workspace::Workspace;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;
use tracing::info;

/// Maximum concurrent git operations.
const MAX_CONCURRENT_GIT: usize = 8;

/// A fast, SDK-agnostic multi-repo and toolchain front-end for MCU/SoC development.
#[derive(Parser)]
#[command(name = "east", version, about)]
struct Cli {
    /// Increase verbosity (-v for debug, -vv for trace).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new east workspace from a manifest.
    Init {
        /// URL or local path to the manifest repository.
        manifest: String,
    },
    /// Update (fetch/checkout) all projects in the workspace.
    Update,
    /// List all projects in the workspace.
    List,
    /// Show status of all projects in the workspace.
    Status,
    /// Manifest operations.
    Manifest {
        /// Print the fully resolved manifest.
        #[arg(long)]
        resolve: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Configure tracing based on verbosity
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info,east=debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(run(cli))
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init { manifest } => cmd_init(&manifest).await,
        Commands::Update => cmd_update().await,
        Commands::List => cmd_list(),
        Commands::Status => cmd_status().await,
        Commands::Manifest { resolve } => {
            if resolve {
                cmd_manifest_resolve()
            } else {
                bail!("use --resolve to print the resolved manifest")
            }
        }
    }
}

async fn cmd_init(manifest_source: &str) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let manifest_path = PathBuf::from(manifest_source);

    if manifest_path.is_dir() {
        // Local directory: it's a git repo containing east.yml
        let source_manifest = manifest_path.join("east.yml");
        if !source_manifest.exists() {
            bail!("no east.yml found in {}", manifest_path.display());
        }
        std::fs::copy(&source_manifest, cwd.join("east.yml")).context("failed to copy east.yml")?;
    } else if manifest_path.is_file() {
        // Local file: copy it directly
        std::fs::copy(&manifest_path, cwd.join("east.yml"))
            .context("failed to copy manifest file")?;
    } else {
        // Treat as a git URL: clone to a temp dir, extract east.yml
        let temp_dir = tempfile::tempdir().context("failed to create temp dir")?;
        let clone_dest = temp_dir.path().join("manifest");
        Git::clone(manifest_source, &clone_dest, None)
            .await
            .context("failed to clone manifest repository")?;

        let source_manifest = clone_dest.join("east.yml");
        if !source_manifest.exists() {
            bail!("no east.yml found in cloned manifest repository");
        }
        std::fs::copy(&source_manifest, cwd.join("east.yml"))
            .context("failed to copy east.yml from cloned repo")?;
    }

    // Initialize workspace
    Workspace::init(&cwd).context("failed to initialize workspace")?;
    info!("initialized east workspace at {}", cwd.display());

    // Run update to clone all projects
    do_update(&cwd).await
}

async fn cmd_update() -> anyhow::Result<()> {
    let ws =
        Workspace::discover(&std::env::current_dir()?).context("not inside an east workspace")?;
    do_update(ws.root()).await
}

async fn do_update(workspace_root: &Path) -> anyhow::Result<()> {
    let manifest_path = workspace_root.join("east.yml");
    let manifest = Manifest::resolve(&manifest_path).context("failed to resolve manifest")?;
    let projects = manifest.filtered_projects();

    if projects.is_empty() {
        println!("no projects to update");
        return Ok(());
    }

    let mp = MultiProgress::new();
    let style = ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .expect("valid template");

    let semaphore = std::sync::Arc::new(Semaphore::new(MAX_CONCURRENT_GIT));
    let mut handles = Vec::new();

    for project in &projects {
        let project_path = workspace_root.join(project.effective_path());
        let revision = manifest.project_revision(project).map(String::from);
        let clone_url = manifest.project_clone_url(project).ok().map(String::from);
        let project_name = project.name.clone();
        let sem = semaphore.clone();
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(style.clone());

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            pb.set_message(format!("{project_name}: starting..."));

            let result = if project_path.exists() {
                // Already cloned: fetch + checkout
                pb.set_message(format!("{project_name}: fetching..."));
                Git::fetch(&project_path).await?;
                if let Some(rev) = &revision {
                    pb.set_message(format!("{project_name}: checking out {rev}..."));
                    Git::checkout(&project_path, rev).await?;
                }
                Ok(())
            } else if let Some(url) = &clone_url {
                // Clone
                pb.set_message(format!("{project_name}: cloning..."));
                Git::clone(url, &project_path, revision.as_deref()).await
            } else {
                Err(east_vcs::error::VcsError::GitFailed {
                    path: project_path.clone(),
                    stderr: format!("no remote URL for project {project_name}"),
                })
            };

            match &result {
                Ok(()) => pb.finish_with_message(format!("{project_name}: done")),
                Err(e) => pb.finish_with_message(format!("{project_name}: FAILED ({e})")),
            }
            result
        });
        handles.push((project.name.clone(), handle));
    }

    let mut errors = Vec::new();
    for (name, handle) in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => errors.push(format!("{name}: {e}")),
            Err(e) => errors.push(format!("{name}: task panicked: {e}")),
        }
    }

    if errors.is_empty() {
        println!("updated {} projects", projects.len());
        Ok(())
    } else {
        bail!("errors updating projects:\n{}", errors.join("\n"));
    }
}

fn cmd_list() -> anyhow::Result<()> {
    let ws =
        Workspace::discover(&std::env::current_dir()?).context("not inside an east workspace")?;
    let manifest_path = ws.manifest_path();
    let manifest = Manifest::resolve(&manifest_path).context("failed to resolve manifest")?;
    let projects = manifest.filtered_projects();

    println!(
        "{:<20} {:<30} {:<15} {:<10}",
        "NAME", "PATH", "REVISION", "CLONED"
    );
    for project in &projects {
        let project_path = ws.root().join(project.effective_path());
        let revision = manifest.project_revision(project).unwrap_or("-");
        let cloned = if project_path.exists() { "yes" } else { "no" };
        println!(
            "{:<20} {:<30} {:<15} {:<10}",
            project.name,
            project.effective_path(),
            revision,
            cloned
        );
    }

    Ok(())
}

async fn cmd_status() -> anyhow::Result<()> {
    let ws =
        Workspace::discover(&std::env::current_dir()?).context("not inside an east workspace")?;
    let manifest_path = ws.manifest_path();
    let manifest = Manifest::resolve(&manifest_path).context("failed to resolve manifest")?;
    let projects = manifest.filtered_projects();

    println!(
        "{:<20} {:<12} {:<42} {:<10}",
        "NAME", "STATUS", "HEAD", "BRANCH"
    );
    for project in &projects {
        let project_path = ws.root().join(project.effective_path());
        if !project_path.exists() {
            println!(
                "{:<20} {:<12} {:<42} {:<10}",
                project.name, "not cloned", "-", "-"
            );
            continue;
        }

        let dirty = Git::is_dirty(&project_path).await.unwrap_or(true);
        let head = Git::head(&project_path)
            .await
            .unwrap_or_else(|_| "unknown".into());
        let branch = Git::current_branch(&project_path)
            .await
            .unwrap_or_else(|_| "unknown".into());
        let status = if dirty { "dirty" } else { "clean" };

        println!(
            "{:<20} {:<12} {:<42} {:<10}",
            project.name, status, head, branch
        );
    }

    Ok(())
}

fn cmd_manifest_resolve() -> anyhow::Result<()> {
    let ws =
        Workspace::discover(&std::env::current_dir()?).context("not inside an east workspace")?;
    let manifest_path = ws.manifest_path();
    let manifest = Manifest::resolve(&manifest_path).context("failed to resolve manifest")?;
    let yaml = serde_yaml::to_string(&manifest).context("failed to serialize resolved manifest")?;
    print!("{yaml}");
    Ok(())
}
