//! mempalace CLI - Entry point

use clap::Parser;

use mempalace::commands::{self, Commands};
use mempalace::error::Result;
use mempalace::logger;

/// mempalace - A local-first memory palace system
#[derive(Parser, Debug)]
#[command(
    name = "mempalace",
    version,
    about = "A local-first memory palace system",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _log_guard = logger::init()?;
    let cli = Cli::parse();
    commands::run(cli.command).await
}

#[cfg(test)]
#[path = "./tests/wing_name.rs"]
mod wing_name;
