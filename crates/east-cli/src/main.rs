#![forbid(unsafe_code)]
//! CLI entrypoint for east.

use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use clap::{Parser, Subcommand};
use east_command::registry::{CommandRegistry, CommandSource};
use east_command::template::TemplateEngine;
use east_config::path::DefaultPathProvider;
use east_config::{Config, ConfigLayer, ConfigValue};
use east_manifest::Manifest;
use east_vcs::Git;
use east_workspace::Workspace;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use miette::{IntoDiagnostic, WrapErr, bail};
use tokio::sync::Semaphore;
use tracing::info;

/// Maximum concurrent git operations.
const MAX_CONCURRENT_GIT: usize = 8;

/// A fast, manifest-driven development toolkit.
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
#[command(allow_external_subcommands = true)]
enum Commands {
    /// Initialize a new east workspace from a manifest.
    Init {
        /// URL or local path to the manifest repository.
        manifest: String,
        /// Branch or tag to use when fetching from a remote repository.
        #[arg(short, long)]
        revision: Option<String>,
    },
    /// Update (fetch/checkout) all projects in the workspace.
    Update {
        /// Force checkout even if the working tree has uncommitted changes.
        /// When project names are given, only those projects are force-checked-out.
        /// Without names, force applies to all projects.
        #[arg(short, long)]
        force: bool,
        /// Project names to force checkout (only used with --force).
        #[arg(requires = "force")]
        projects: Vec<String>,
    },
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
    /// Read or write configuration values.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// External/extension command (captured by `allow_external_subcommands`).
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Get a configuration value.
    Get {
        /// Dotted key (e.g. user.name).
        key: String,
    },
    /// Set a configuration value.
    Set {
        /// Parse the value as an integer.
        #[arg(long)]
        int: bool,
        /// Parse the value as a boolean.
        #[arg(long = "bool")]
        as_bool: bool,
        /// Parse the value as a float.
        #[arg(long)]
        float: bool,
        /// Target layer (global or workspace). Defaults to global.
        #[arg(long, default_value = "global")]
        layer: String,
        /// Dotted key.
        key: String,
        /// Value to set.
        value: String,
    },
    /// Remove a configuration value.
    Unset {
        /// Target layer. Defaults to global.
        #[arg(long, default_value = "global")]
        layer: String,
        /// Dotted key.
        key: String,
    },
    /// List all configuration values.
    List,
}

fn main() -> miette::Result<()> {
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

    let runtime = tokio::runtime::Runtime::new().into_diagnostic()?;
    runtime.block_on(run(cli))
}

async fn run(cli: Cli) -> miette::Result<()> {
    match cli.command {
        Commands::Init { manifest, revision } => cmd_init(&manifest, revision.as_deref()).await,
        Commands::Update { force, projects } => cmd_update(force, &projects).await,
        Commands::List => cmd_list(),
        Commands::Status => cmd_status().await,
        Commands::Manifest { resolve } => {
            if resolve {
                cmd_manifest_resolve()
            } else {
                bail!("use --resolve to print the resolved manifest")
            }
        }
        Commands::Config { action } => cmd_config(action),
        Commands::External(args) => cmd_external(&args).await,
    }
}

async fn cmd_init(manifest_source: &str, revision: Option<&str>) -> miette::Result<()> {
    let cwd = std::env::current_dir().into_diagnostic()?;
    let manifest_path = PathBuf::from(manifest_source);

    if manifest_path.is_dir() {
        // Local directory: it's a git repo containing east.yml
        let source_manifest = manifest_path.join("east.yml");
        if !source_manifest.exists() {
            bail!("no east.yml found in {}", manifest_path.display());
        }
        std::fs::copy(&source_manifest, cwd.join("east.yml"))
            .into_diagnostic()
            .wrap_err("failed to copy east.yml")?;
    } else if manifest_path.is_file() {
        // Local file: copy it directly
        std::fs::copy(&manifest_path, cwd.join("east.yml"))
            .into_diagnostic()
            .wrap_err("failed to copy manifest file")?;
    } else {
        // Treat as a git URL: sparse-checkout only east.yml
        let temp_dir = tempfile::tempdir()
            .into_diagnostic()
            .wrap_err("failed to create temp dir")?;
        let clone_dest = temp_dir.path().join("manifest");
        Git::fetch_file(manifest_source, "east.yml", &clone_dest, revision)
            .await
            .into_diagnostic()
            .wrap_err("failed to fetch east.yml from manifest repository")?;

        let source_manifest = clone_dest.join("east.yml");
        if !source_manifest.exists() {
            bail!("no east.yml found in manifest repository");
        }
        std::fs::copy(&source_manifest, cwd.join("east.yml"))
            .into_diagnostic()
            .wrap_err("failed to copy east.yml from cloned repo")?;
    }

    // Initialize workspace
    Workspace::init(&cwd)
        .into_diagnostic()
        .wrap_err("failed to initialize workspace")?;
    info!("initialized east workspace at {}", cwd.display());

    // Run update to clone all projects
    do_update(&cwd, false, &[]).await
}

async fn cmd_update(force: bool, force_projects: &[String]) -> miette::Result<()> {
    let ws = Workspace::discover(&std::env::current_dir().into_diagnostic()?)
        .into_diagnostic()
        .wrap_err("not inside an east workspace")?;
    do_update(ws.root(), force, force_projects).await
}

#[allow(clippy::too_many_lines)]
async fn do_update(
    workspace_root: &Path,
    force: bool,
    force_projects: &[String],
) -> miette::Result<()> {
    let manifest_path = workspace_root.join("east.yml");
    let manifest = Manifest::resolve(&manifest_path)
        .into_diagnostic()
        .wrap_err("failed to resolve manifest")?;
    let projects = manifest.filtered_projects();

    if projects.is_empty() {
        println!("no projects to update");
        return Ok(());
    }

    // Validate that --force project names actually exist in the manifest
    if !force_projects.is_empty() {
        let known: std::collections::HashSet<&str> =
            projects.iter().map(|p| p.name.as_str()).collect();
        let mut unknown = Vec::new();
        for name in force_projects {
            if !known.contains(name.as_str()) {
                unknown.push(name.as_str());
            }
        }
        if !unknown.is_empty() {
            bail!("unknown project(s) for --force: {}", unknown.join(", "));
        }
    }

    let total = projects.len() as u64;
    let mp = MultiProgress::new();

    // Top-level progress bar showing overall completion
    let overall_style = ProgressStyle::default_bar()
        .template("[{bar:30.cyan/dim}] {pos}/{len} {msg}")
        .expect("valid template")
        .progress_chars("##-");
    let overall = mp.add(ProgressBar::new(total));
    overall.set_style(overall_style);
    overall.set_message("updating...");

    // Style for per-task spinners (inserted below the overall bar)
    let spinner_style = ProgressStyle::default_spinner()
        .template("  {spinner:.green} {msg}")
        .expect("valid template");

    let semaphore = std::sync::Arc::new(Semaphore::new(MAX_CONCURRENT_GIT));
    let overall = std::sync::Arc::new(overall);
    let force_set: std::sync::Arc<std::collections::HashSet<String>> =
        std::sync::Arc::new(force_projects.iter().cloned().collect());
    let mut handles = Vec::new();

    for project in &projects {
        let project_path = workspace_root.join(project.effective_path());
        let revision = manifest.project_revision(project).map(String::from);
        let clone_url = manifest.project_clone_url(project).ok();
        let project_name = project.name.clone();
        let project_rel_path = project.effective_path().to_string();
        let sem = semaphore.clone();
        let overall = overall.clone();
        let force_set = force_set.clone();
        let mp_handle = mp.clone();
        let spinner_style_clone = spinner_style.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            // Add spinner only after acquiring permit to avoid empty lines
            let pb = mp_handle.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style_clone);
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb.set_message(format!("{project_name}: starting..."));

            // A directory may exist without being a git repo (e.g. parent
            // dirs created by sibling project clones). Treat non-repo dirs
            // as needing a fresh clone.
            let is_git_repo = project_path.join(".git").exists();

            let result = if is_git_repo {
                // Already cloned: fetch + checkout
                pb.set_message(format!("{project_name}: fetching..."));
                Git::fetch(&project_path).await?;
                if let Some(rev) = &revision {
                    let dirty = Git::is_dirty(&project_path).await.unwrap_or(false);
                    let force_this =
                        force && (force_set.is_empty() || force_set.contains(&project_name));
                    if dirty && !force_this {
                        pb.finish_with_message(format!(
                            "{project_name} ({project_rel_path}): skipped checkout (uncommitted changes, use --force to override)"
                        ));
                        overall.inc(1);
                        return Ok(());
                    }
                    pb.set_message(format!("{project_name}: checking out {rev}..."));
                    if force_this {
                        Git::force_checkout(&project_path, rev).await?;
                    } else {
                        Git::checkout(&project_path, rev).await?;
                    }
                }
                Ok(())
            } else if let Some(url) = &clone_url {
                if project_path.exists() {
                    // Directory exists but is not a git repo (created by
                    // sibling clones); use init+fetch instead of clone.
                    pb.set_message(format!("{project_name}: initializing..."));
                    Git::init_and_fetch(url, &project_path, revision.as_deref()).await
                } else {
                    // Clone — fallback to init+fetch if the directory was
                    // created by a concurrent sibling clone in the meantime.
                    pb.set_message(format!("{project_name}: cloning..."));
                    let clone_result = Git::clone(url, &project_path, revision.as_deref()).await;
                    match &clone_result {
                        Err(east_vcs::error::VcsError::GitFailed { stderr, .. })
                            if (stderr.contains("already exists")
                                || stderr.contains("File exists"))
                                && project_path.exists()
                                && !project_path.join(".git").exists() =>
                        {
                            // Directory was created by a concurrent sibling clone;
                            // fallback to init+fetch only for this specific case.
                            pb.set_message(format!("{project_name}: initializing (fallback)..."));
                            Git::init_and_fetch(url, &project_path, revision.as_deref()).await
                        }
                        _ => clone_result,
                    }
                }
            } else {
                Err(east_vcs::error::VcsError::GitFailed {
                    path: project_path.clone(),
                    stderr: format!("no remote URL for project {project_name}"),
                })
            };

            // Update UI: remove spinner on success, keep failure visible
            match &result {
                Ok(()) => pb.finish_and_clear(),
                Err(e) => pb.finish_with_message(format!("{project_name}: FAILED ({e})")),
            }
            overall.inc(1);
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
    overall.finish_and_clear();

    // Maintain .git/info/exclude for parent repos that contain child project paths.
    // This prevents nested project directories from showing as untracked in the parent.
    update_git_excludes(workspace_root, &projects);

    if errors.is_empty() {
        println!("updated {} projects", projects.len());
        Ok(())
    } else {
        bail!("errors updating projects:\n{}", errors.join("\n"));
    }
}

/// For each project whose path is a prefix of another project's path (parent/child
/// relationship), add the child's relative path to the parent's `.git/info/exclude`.
fn update_git_excludes(workspace_root: &Path, projects: &[&east_manifest::Project]) {
    use std::collections::BTreeMap;

    // Build a map of project path -> list of child relative paths
    let paths: Vec<String> = projects
        .iter()
        .map(|p| p.effective_path().to_string())
        .collect();
    let mut parent_children: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (i, parent_path) in paths.iter().enumerate() {
        for (j, child_path) in paths.iter().enumerate() {
            if i == j {
                continue;
            }
            let prefix = format!("{parent_path}/");
            if child_path.starts_with(&prefix) {
                // child_path relative to parent_path
                let relative = &child_path[prefix.len()..];
                parent_children
                    .entry(parent_path.clone())
                    .or_default()
                    .push(format!("/{relative}/"));
            }
        }
    }

    let marker = "# managed by east — do not edit this block";
    let end_marker = "# end east managed block";

    for (parent_path, children) in &parent_children {
        let git_dir = workspace_root.join(parent_path).join(".git");
        if !git_dir.is_dir() {
            continue;
        }
        let info_dir = git_dir.join("info");
        let _ = std::fs::create_dir_all(&info_dir);
        let exclude_path = info_dir.join("exclude");

        // Read existing content, strip old east block if present
        let existing = std::fs::read_to_string(&exclude_path).unwrap_or_default();
        let mut lines: Vec<&str> = Vec::new();
        let mut in_block = false;
        for line in existing.lines() {
            if line == marker {
                in_block = true;
                continue;
            }
            if line == end_marker {
                in_block = false;
                continue;
            }
            if !in_block {
                lines.push(line);
            }
        }

        // Remove trailing empty lines
        while lines.last() == Some(&"") {
            lines.pop();
        }

        // Append east managed block
        if !lines.is_empty() {
            lines.push("");
        }
        let mut block = vec![marker.to_string()];
        let mut sorted_children = children.clone();
        sorted_children.sort();
        sorted_children.dedup();
        for child in &sorted_children {
            block.push(child.clone());
        }
        block.push(end_marker.to_string());

        let mut output = lines.join("\n");
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&block.join("\n"));
        output.push('\n');

        let _ = std::fs::write(&exclude_path, output);
    }
}

fn cmd_list() -> miette::Result<()> {
    let ws = Workspace::discover(&std::env::current_dir().into_diagnostic()?)
        .into_diagnostic()
        .wrap_err("not inside an east workspace")?;
    let manifest_path = ws.manifest_path();
    let manifest = Manifest::resolve(&manifest_path)
        .into_diagnostic()
        .wrap_err("failed to resolve manifest")?;
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

async fn cmd_status() -> miette::Result<()> {
    let ws = Workspace::discover(&std::env::current_dir().into_diagnostic()?)
        .into_diagnostic()
        .wrap_err("not inside an east workspace")?;
    let manifest_path = ws.manifest_path();
    let manifest = Manifest::resolve(&manifest_path)
        .into_diagnostic()
        .wrap_err("failed to resolve manifest")?;
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

fn cmd_manifest_resolve() -> miette::Result<()> {
    let ws = Workspace::discover(&std::env::current_dir().into_diagnostic()?)
        .into_diagnostic()
        .wrap_err("not inside an east workspace")?;
    let manifest_path = ws.manifest_path();
    let manifest = Manifest::resolve(&manifest_path)
        .into_diagnostic()
        .wrap_err("failed to resolve manifest")?;
    let yaml = serde_yaml::to_string(&manifest)
        .into_diagnostic()
        .wrap_err("failed to serialize resolved manifest")?;
    print!("{yaml}");
    Ok(())
}

fn cmd_config(action: ConfigAction) -> miette::Result<()> {
    let workspace_root = Workspace::discover(&std::env::current_dir().into_diagnostic()?)
        .ok()
        .map(|ws| ws.root().to_path_buf());
    let provider = DefaultPathProvider::new(workspace_root);
    let mut config = Config::load_with_provider(&provider)
        .into_diagnostic()
        .wrap_err("failed to load config")?;

    match action {
        ConfigAction::Get { key } => {
            let value = config
                .get(&key)
                .ok_or_else(|| miette::miette!("key not found: {key}"))?;
            println!("{value}");
        }
        ConfigAction::Set {
            int,
            as_bool,
            float,
            layer,
            key,
            value,
        } => {
            let config_value = if int {
                ConfigValue::Integer(
                    value
                        .parse()
                        .into_diagnostic()
                        .wrap_err(format!("invalid integer: {value}"))?,
                )
            } else if as_bool {
                ConfigValue::Boolean(
                    value
                        .parse()
                        .into_diagnostic()
                        .wrap_err(format!("invalid boolean: {value}"))?,
                )
            } else if float {
                ConfigValue::Float(
                    value
                        .parse()
                        .into_diagnostic()
                        .wrap_err(format!("invalid float: {value}"))?,
                )
            } else {
                ConfigValue::String(value)
            };
            let layer = parse_layer(&layer)?;
            config.set(layer, &key, config_value);
            config
                .save(&provider, layer)
                .into_diagnostic()
                .wrap_err("failed to save config")?;
        }
        ConfigAction::Unset { layer, key } => {
            let layer = parse_layer(&layer)?;
            config.unset(layer, &key);
            config
                .save(&provider, layer)
                .into_diagnostic()
                .wrap_err("failed to save config")?;
        }
        ConfigAction::List => {
            for (key, value) in config.iter() {
                println!("{key} = {value}");
            }
        }
    }
    Ok(())
}

fn parse_layer(s: &str) -> miette::Result<ConfigLayer> {
    match s {
        "system" => Ok(ConfigLayer::System),
        "global" => Ok(ConfigLayer::Global),
        "workspace" => Ok(ConfigLayer::Workspace),
        _ => bail!("unknown layer: {s} (expected: system, global, workspace)"),
    }
}

async fn cmd_external(args: &[String]) -> miette::Result<()> {
    let cmd_name = args
        .first()
        .ok_or_else(|| miette::miette!("missing command name"))?;
    let extra_args = &args[1..];

    let ws = Workspace::discover(&std::env::current_dir().into_diagnostic()?)
        .into_diagnostic()
        .wrap_err("not inside an east workspace")?;
    let manifest_path = ws.manifest_path();
    let manifest = Manifest::resolve(&manifest_path)
        .into_diagnostic()
        .wrap_err("failed to resolve manifest")?;

    // Build command registry
    let mut registry = CommandRegistry::from_manifest(&manifest);
    if let Ok(path_env) = std::env::var("PATH") {
        registry.discover_path(&path_env);
    }

    let resolved = registry
        .get(cmd_name)
        .ok_or_else(|| miette::miette!("unknown command: {cmd_name}"))?;

    // Build template variables
    let mut vars = BTreeMap::new();
    vars.insert(
        "workspace.root".to_string(),
        ws.root().to_string_lossy().into_owned(),
    );
    vars.insert(
        "workspace.manifest".to_string(),
        ws.manifest_path().to_string_lossy().into_owned(),
    );

    // Add config variables
    let provider = DefaultPathProvider::new(Some(ws.root().to_path_buf()));
    if let Ok(config) = Config::load_with_provider(&provider) {
        for (key, value) in config.iter() {
            vars.insert(format!("config.{key}"), value.to_string());
        }
    }

    // Add env variables
    for (key, value) in std::env::vars() {
        vars.insert(format!("env.{key}"), value);
    }

    match &resolved.source {
        CommandSource::Manifest => {
            let decl = resolved
                .decl
                .as_ref()
                .expect("manifest command should have decl");
            dispatch_manifest_command(decl, &vars, ws.root(), extra_args).await
        }
        CommandSource::Path { executable } => {
            let status = tokio::process::Command::new(executable)
                .args(extra_args)
                .current_dir(strip_unc_prefix(ws.root()))
                .status()
                .await
                .into_diagnostic()
                .wrap_err(format!("failed to run {}", executable.display()))?;
            if status.success() {
                Ok(())
            } else {
                bail!(
                    "command '{}' exited with {}",
                    cmd_name,
                    status.code().unwrap_or(-1)
                );
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn dispatch_manifest_command(
    decl: &east_manifest::CommandDecl,
    vars: &BTreeMap<String, String>,
    workspace_root: &Path,
    extra_args: &[String],
) -> miette::Result<()> {
    let engine = TemplateEngine::new();

    // Determine working directory
    let work_dir = if let Some(cwd_template) = &decl.cwd {
        let rendered = engine
            .render(cwd_template, vars, "command cwd")
            .map_err(|e| miette::miette!("{e}"))?;
        PathBuf::from(rendered)
    } else {
        workspace_root.to_path_buf()
    };
    let work_dir = strip_unc_prefix(&work_dir);

    if let Some(exec_str) = &decl.exec {
        // Render template
        let rendered = engine
            .render(exec_str, vars, &format!("command '{}' exec", decl.name))
            .map_err(|e| miette::miette!("{e}"))?;

        // Shell out
        #[cfg(unix)]
        let mut cmd = {
            let mut c = tokio::process::Command::new("sh");
            c.arg("-c").arg(&rendered);
            c
        };
        #[cfg(windows)]
        let mut cmd = {
            let mut c = tokio::process::Command::new("cmd");
            c.arg("/C").arg(&rendered);
            c
        };

        cmd.current_dir(&work_dir);
        // Add extra args to the command
        for arg in extra_args {
            cmd.arg(arg);
        }
        // Set env vars
        for (key, value) in &decl.env {
            let rendered_val = engine
                .render(value, vars, &format!("command '{}' env.{key}", decl.name))
                .map_err(|e| miette::miette!("{e}"))?;
            cmd.env(key, rendered_val);
        }

        let status = cmd
            .status()
            .await
            .into_diagnostic()
            .wrap_err("failed to spawn exec command")?;
        if !status.success() {
            bail!(
                "command '{}' exited with {}",
                decl.name,
                status.code().unwrap_or(-1)
            );
        }
    } else if let Some(script_path) = &decl.script {
        // Script path is relative to the manifest that declared it
        let declaring_manifest = decl.declared_in.as_deref().unwrap_or(workspace_root);
        let mrp =
            east_manifest::path_resolve::ManifestRelativePath::new(declaring_manifest, script_path);
        let resolved_script = mrp.resolve().into_diagnostic().wrap_err(format!(
            "failed to resolve script '{}' for command '{}'",
            script_path, decl.name
        ))?;
        let mut cmd = tokio::process::Command::new(&resolved_script);
        cmd.current_dir(&work_dir);
        for arg in extra_args {
            cmd.arg(arg);
        }
        for (key, value) in &decl.env {
            let rendered_val = engine
                .render(value, vars, &format!("command '{}' env.{key}", decl.name))
                .map_err(|e| miette::miette!("{e}"))?;
            cmd.env(key, rendered_val);
        }

        let status = cmd
            .status()
            .await
            .into_diagnostic()
            .wrap_err("failed to spawn script command")?;
        if !status.success() {
            bail!(
                "command '{}' exited with {}",
                decl.name,
                status.code().unwrap_or(-1)
            );
        }
    } else if let Some(executable_name) = &decl.executable {
        let mut cmd = tokio::process::Command::new(executable_name);
        cmd.current_dir(&work_dir);
        for arg in extra_args {
            cmd.arg(arg);
        }
        for (key, value) in &decl.env {
            let rendered_val = engine
                .render(value, vars, &format!("command '{}' env.{key}", decl.name))
                .map_err(|e| miette::miette!("{e}"))?;
            cmd.env(key, rendered_val);
        }

        let status = cmd
            .status()
            .await
            .into_diagnostic()
            .wrap_err(format!("failed to spawn {executable_name}"))?;
        if !status.success() {
            bail!(
                "command '{}' exited with {}",
                decl.name,
                status.code().unwrap_or(-1)
            );
        }
    }

    Ok(())
}

/// Strip the `\\?\` extended-length prefix from Windows paths.
///
/// `cmd.exe` cannot use UNC paths as a working directory. Since
/// `std::fs::canonicalize` produces `\\?\` paths on Windows, we strip
/// the prefix before passing paths to child processes.
fn strip_unc_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    s.strip_prefix(r"\\?\")
        .map_or_else(|| path.to_path_buf(), PathBuf::from)
}
