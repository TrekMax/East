#![forbid(unsafe_code)]
//! CLI entrypoint for east.

use clap::{Parser, Subcommand};

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
        Commands::List => cmd_list().await,
        Commands::Status => cmd_status().await,
        Commands::Manifest { resolve } => {
            if resolve {
                cmd_manifest_resolve().await
            } else {
                anyhow::bail!("use --resolve to print the resolved manifest")
            }
        }
    }
}

async fn cmd_init(_manifest: &str) -> anyhow::Result<()> {
    anyhow::bail!("east init is not yet implemented")
}

async fn cmd_update() -> anyhow::Result<()> {
    anyhow::bail!("east update is not yet implemented")
}

async fn cmd_list() -> anyhow::Result<()> {
    anyhow::bail!("east list is not yet implemented")
}

async fn cmd_status() -> anyhow::Result<()> {
    anyhow::bail!("east status is not yet implemented")
}

async fn cmd_manifest_resolve() -> anyhow::Result<()> {
    anyhow::bail!("east manifest --resolve is not yet implemented")
}
