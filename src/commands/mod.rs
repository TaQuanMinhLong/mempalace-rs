use clap::Subcommand;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::Result;

#[cfg(feature = "bench")]
pub mod benchmark;
pub mod compress;
pub mod init;
pub mod mine;
pub mod repair;
pub mod search;
pub mod serve;
pub mod split;
pub mod status;
pub mod wakeup;

/// mempalace subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new project
    Init {
        /// Project directory
        dir: PathBuf,
    },
    /// Run benchmark fixtures
    #[cfg(feature = "bench")]
    Benchmark {
        /// Fixture file path
        #[arg(long)]
        fixture: Option<String>,
        /// Recall cutoff
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
    /// Mine files or conversations
    Mine {
        /// Directory to mine
        dir: PathBuf,
        /// Mining mode: "projects" or "convos"
        #[arg(long, default_value = "projects")]
        mode: String,
        /// Agent name for attribution
        #[arg(long, default_value = "cli")]
        agent: String,
    },
    /// Search memories
    Search {
        /// Search query
        query: String,
        /// Filter by wing
        #[arg(long)]
        wing: Option<String>,
        /// Filter by room
        #[arg(long)]
        room: Option<String>,
        /// Maximum results
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Load L0 + L1 context (identity + essential story)
    WakeUp {
        /// Wing to wake up in
        #[arg(long)]
        wing: Option<String>,
    },
    /// Show palace statistics
    Status,
    /// Compress drawers using AAAK dialect
    Compress {
        /// Wing to compress
        #[arg(long)]
        wing: String,
        /// Room to compress (all rooms if not specified)
        #[arg(long)]
        room: Option<String>,
    },
    /// Repair/rebuild the index
    Repair,
    /// Split mega transcript files into per-session files
    Split {
        /// Directory containing mega files
        dir: PathBuf,
    },
    /// Start MCP server (JSON-RPC over stdio)
    Serve,
}

pub async fn run(command: Commands) -> Result<()> {
    match command {
        #[cfg(feature = "bench")]
        Commands::Benchmark { fixture, limit } => benchmark::run(fixture.as_deref(), limit),
        Commands::Init { dir } => init::run(&dir),
        Commands::Mine { dir, mode, agent } => mine::run(&dir, &mode, &agent),
        Commands::Search {
            query,
            wing,
            room,
            limit,
        } => search::run(&query, wing.as_deref(), room.as_deref(), limit).await,
        Commands::WakeUp { wing } => wakeup::run(wing.as_deref()),
        Commands::Status => status::run(),
        Commands::Compress { wing, room } => compress::run(&wing, room.as_deref()),
        Commands::Repair => repair::run(),
        Commands::Split { dir } => split::run(&dir),
        Commands::Serve => serve::run().await,
    }
}

/// Derive a wing name from a directory path.
/// Uses the directory's file name (basename), prefixed with "wing_".
/// Falls back to "wing_general" when no name can be determined.
pub fn wing_name_from_dir(dir: &std::path::Path) -> String {
    fn slugify(s: &str) -> String {
        s.to_lowercase()
            .chars()
            .map(|c: char| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    fn to_wing_name(path: &std::path::Path) -> Option<String> {
        path.file_name().map(|s| {
            let slug = slugify(&s.to_string_lossy());
            if slug.is_empty() {
                "general".to_string()
            } else {
                format!("wing_{}", slug)
            }
        })
    }

    if let Some(name) = to_wing_name(dir) {
        return name;
    }

    if let Ok(canonical) = std::fs::canonicalize(dir) {
        if let Some(name) = to_wing_name(&canonical) {
            return name;
        }
    }

    "wing_general".to_string()
}

#[inline]
pub(crate) fn load_config() -> Result<Config> {
    Config::load()
}
