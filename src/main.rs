mod commands;
mod config;
mod import;
mod linker;
mod manifest;
mod repo;
mod status;
mod strategy;
mod template;
mod vars;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config::GlobalConfig;
use crate::linker::LinkOptions;

#[derive(Parser)]
#[command(name = "homie")]
#[command(about = "Dotfiles symlink orchestrator with templates and multiple repo support")]
#[command(version)]
struct Cli {
    /// Show what would happen without making changes
    #[arg(short = 'n', long, global = true)]
    dry_run: bool,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create symlinks for one or all repos
    Link {
        /// Repo name (optional, links all if not specified)
        repo: Option<String>,

        /// Replace conflicts with backup
        #[arg(short, long)]
        force: bool,

        /// Skip fetching git imports
        #[arg(long)]
        no_fetch: bool,
    },

    /// Remove symlinks for one or all repos
    Unlink {
        /// Repo name (optional, unlinks all if not specified)
        repo: Option<String>,
    },

    /// Show symlink status
    Status {
        /// Repo name (optional, shows all if not specified)
        repo: Option<String>,
    },

    /// Add a file to a repo (move + symlink)
    Add {
        /// Repo to add to
        repo: String,

        /// File to add
        file: String,
    },

    /// Show differences between repo and target
    Diff {
        /// Repo name (optional, shows all if not specified)
        repo: Option<String>,
    },

    /// Initialize a new repo
    Init {
        /// Name for the new repo
        name: String,

        /// Target directory for links (default: ~)
        #[arg(short, long)]
        target: Option<String>,
    },

    /// Clone an existing dotfiles repo
    Clone {
        /// Git URL to clone
        url: String,

        /// Name for the repo (derived from URL if not specified)
        #[arg(long)]
        name: Option<String>,
    },

    /// List discovered repos
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = GlobalConfig::load()?;

    match cli.command {
        Commands::Link { repo, force, no_fetch } => {
            let options = LinkOptions {
                dry_run: cli.dry_run,
                force,
                verbose: cli.verbose,
                no_fetch,
            };
            commands::link::run(&config, repo.as_deref(), options)
        }

        Commands::Unlink { repo } => {
            let options = LinkOptions {
                dry_run: cli.dry_run,
                force: false,
                verbose: cli.verbose,
                no_fetch: false,
            };
            commands::unlink::run(&config, repo.as_deref(), options)
        }

        Commands::Status { repo } => commands::status::run(repo.as_deref(), cli.verbose),

        Commands::Add { repo, file } => commands::add::run(&repo, &file, cli.dry_run),

        Commands::Diff { repo } => commands::diff::run(repo.as_deref()),

        Commands::Init { name, target } => {
            commands::init::run(&name, target.as_deref(), cli.dry_run)
        }

        Commands::Clone { url, name } => {
            commands::clone::run(&url, name.as_deref(), cli.dry_run)
        }

        Commands::List => commands::list::run(),
    }
}
