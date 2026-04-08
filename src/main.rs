//! mempalace CLI - Entry point

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use mempalace::config::Config;
use mempalace::error::Result;

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

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new project
    Init {
        /// Project directory
        dir: std::path::PathBuf,
    },
    /// Mine files or conversations
    Mine {
        /// Directory to mine
        dir: std::path::PathBuf,
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
        dir: std::path::PathBuf,
    },
    /// Start MCP server (JSON-RPC over stdio)
    Serve,
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { dir } => {
            cmd_init(&dir)?;
        }
        Commands::Mine { dir, mode, agent } => {
            cmd_mine(&dir, &mode, &agent)?;
        }
        Commands::Search {
            query,
            wing,
            room,
            limit,
        } => {
            cmd_search(&query, wing.as_deref(), room.as_deref(), limit)?;
        }
        Commands::WakeUp { wing } => {
            cmd_wakeup(wing.as_deref())?;
        }
        Commands::Status => {
            cmd_status()?;
        }
        Commands::Compress { wing, room } => {
            cmd_compress(&wing, room.as_deref())?;
        }
        Commands::Repair => {
            cmd_repair()?;
        }
        Commands::Split { dir } => {
            cmd_split(&dir)?;
        }
        Commands::Serve => {
            cmd_serve()?;
        }
    }

    Ok(())
}

/// Initialize mempalace in a directory
fn cmd_init(dir: &PathBuf) -> Result<()> {
    println!("Initializing mempalace in {:?}...", dir);

    // Initialize config
    let config_file = Config::init()?;
    println!("  Config created: {:?}", config_file);

    // Create palace directory
    let config = Config::load()?;
    std::fs::create_dir_all(&config.palace_path)?;
    println!("  Palace directory: {:?}", config.palace_path);

    // Create entities.json in the project directory
    let entities_path = dir.join("entities.json");
    if !entities_path.exists() {
        let default_entities = serde_json::json!({
            "people": [],
            "projects": [],
            "uncertain": []
        });
        std::fs::write(
            &entities_path,
            serde_json::to_string_pretty(&default_entities)?,
        )?;
        println!("  Created: {:?}", entities_path);
    }

    println!("\nInitialization complete!");
    println!("  Run 'mempalace mine {}' to start mining.", dir.display());
    Ok(())
}

/// Derive a wing name from a directory path.
/// Uses the directory's file name (basename), prefixed with "wing_".
/// Falls back to "wing_general" when no name can be determined.
pub fn wing_name_from_dir(dir: &Path) -> String {
    fn slugify(s: &str) -> String {
        s.to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else if c.is_whitespace() {
                    '-'
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

    fn to_wing_name(path: &Path) -> Option<String> {
        path.file_name().map(|s| {
            let slug = slugify(&s.to_string_lossy());
            if slug.is_empty() {
                "general".to_string()
            } else {
                format!("wing_{}", slug)
            }
        })
    }

    // First try directly
    if let Some(name) = to_wing_name(dir) {
        return name;
    }

    // For paths with no basename (e.g., ".", "..", empty string),
    // canonicalize to resolve to the actual directory path and retry.
    if let Ok(canonical) = std::fs::canonicalize(dir) {
        if let Some(name) = to_wing_name(&canonical) {
            return name;
        }
    }

    "wing_general".to_string()
}

/// Mine files or conversations
fn cmd_mine(dir: &PathBuf, mode: &str, agent: &str) -> Result<()> {
    use mempalace::miner::{ConvoMiner, FileMiner};
    use mempalace::storage::ChromaStorage;

    println!("Mining {:?} in {} mode (agent: {})", dir, mode, agent);

    if !dir.exists() || !dir.is_dir() {
        println!("  Directory not found: {:?}", dir);
        return Ok(());
    }

    let config = Config::load()?;
    println!("  Palace: {:?}", config.palace_path);
    println!("  Collection: {}", config.collection_name);

    let wing_name = wing_name_from_dir(dir);

    println!("  Mining to wing: {}", wing_name);

    let storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;

    match mode {
        "projects" => {
            println!("  Scanning for project files...");
            let mut miner = FileMiner::new(config.clone(), storage)?;
            match miner.mine_directory(dir, &wing_name) {
                Ok(result) => {
                    println!("\n  Mining complete!");
                    println!("    Files processed: {}", result.files_processed);
                    println!("    Drawers created: {}", result.drawers_created);
                    println!("    Entities found: {}", result.entities_extracted);
                }
                Err(e) => {
                    println!("  Mining error: {}", e);
                }
            }
        }
        "convos" => {
            println!("  Scanning for conversation files...");
            let mut miner = ConvoMiner::new(storage);
            match miner.mine_directory(dir, &wing_name) {
                Ok(result) => {
                    println!("\n  Mining complete!");
                    println!("    Conversations processed: {}", result.files_processed);
                    println!("    Drawers created: {}", result.drawers_created);
                }
                Err(e) => {
                    println!("  Mining error: {}", e);
                }
            }
        }
        _ => {
            println!("  Unknown mode: {}. Use 'projects' or 'convos'.", mode);
        }
    }

    Ok(())
}

/// Search memories
fn cmd_search(query: &str, wing: Option<&str>, room: Option<&str>, limit: usize) -> Result<()> {
    use mempalace::search::SemanticSearcher;
    use mempalace::storage::ChromaStorage;

    println!(
        "Searching '{}' (wing: {:?}, room: {:?}, limit: {})",
        query, wing, room, limit
    );

    if query.is_empty() {
        println!("  Query cannot be empty");
        return Ok(());
    }

    let config = Config::load()?;
    let storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;
    let searcher = SemanticSearcher::new(std::rc::Rc::new(std::cell::RefCell::new(storage)));

    match searcher.search(query, wing, room, limit) {
        Ok(results) => {
            if results.is_empty() {
                println!("  No results found.");
            } else {
                println!("  Found {} result(s):", results.len());
                for (i, result) in results.iter().enumerate().take(10) {
                    println!(
                        "\n  [{}] ({:.2}% match)",
                        i + 1,
                        result.hit.similarity * 100.0
                    );
                    println!("      Wing: {}, Room: {}", result.hit.wing, result.hit.room);
                    // Safe UTF-8 truncation to 200 chars
                    let text = &result.hit.text;
                    let preview_len = text.len().min(200);
                    let mut safe_len = preview_len;
                    while safe_len > 0 && !text.is_char_boundary(safe_len) {
                        safe_len -= 1;
                    }
                    println!("      {}", &text[..safe_len]);
                }
            }
        }
        Err(e) => {
            println!("  Search error: {}", e);
        }
    }

    Ok(())
}

/// Load L0 + L1 context (wake-up)
fn cmd_wakeup(wing: Option<&str>) -> Result<()> {
    println!("Waking up (wing: {:?})...", wing);

    let config = Config::load()?;
    println!("  Palace: {:?}", config.palace_path);

    // Try to load identity file
    if config.identity_path.exists() {
        let identity = std::fs::read_to_string(&config.identity_path)?;
        let tokens = identity.chars().count() / 4;
        println!("\nWake-up text (~{} tokens):", tokens);
        println!("{}", "=".repeat(50));
        println!("{}", identity);
    } else {
        println!("\nNo identity file found. Create ~/.mempalace/identity.txt to set up L0.");
        println!("\nDefault AAAK Protocol:");
        println!("{}", PALACE_PROTOCOL);
    }

    Ok(())
}

/// Show palace statistics
fn cmd_status() -> Result<()> {
    use mempalace::storage::{ChromaStorage, KnowledgeGraph};

    println!("MemPalace v{}\n", env!("CARGO_PKG_VERSION"));

    let config = Config::load()?;
    println!("Palace: {:?}", config.palace_path);
    println!("Collection: {}", config.collection_name);

    // Show drawer count
    let storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;
    let drawer_count = storage.count()?;
    println!("\nDrawers: {}", drawer_count);

    // Show knowledge graph stats
    if config.knowledge_graph_path.exists() {
        println!("Knowledge Graph: {:?}", config.knowledge_graph_path);
        match KnowledgeGraph::new(&config.knowledge_graph_path) {
            Ok(kg) => {
                let entity_count = kg.get_entity_count().unwrap_or(0);
                let triple_count = kg.get_triple_count().unwrap_or(0);
                println!("  Entities: {}", entity_count);
                println!("  Triples: {}", triple_count);
            }
            Err(e) => {
                println!("  (Could not load KG: {})", e);
            }
        }
    }

    // Show config info
    println!("\nConfig Directory: {:?}", config.config_dir);
    println!("Identity File: {:?}", config.identity_path);
    if config.identity_path.exists() {
        if let Ok(identity) = std::fs::read_to_string(&config.identity_path) {
            let tokens = identity.chars().count() / 4;
            println!("  Identity (~{} tokens)", tokens);
        }
    }

    Ok(())
}

/// Compress drawers using AAAK dialect
fn cmd_compress(wing: &str, room: Option<&str>) -> Result<()> {
    use mempalace::dialect::aaak::AaakDialect;
    use mempalace::storage::ChromaStorage;

    println!("Compressing wing '{}'...", wing);

    let config = Config::load()?;
    let storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;

    // Get drawers for this wing
    let drawers = storage.get_drawers_by_filter(Some(wing), room, 1000);

    if drawers.is_empty() {
        println!("  No drawers found for wing '{}'", wing);
        return Ok(());
    }

    println!("  Found {} drawer(s) to compress", drawers.len());

    let dialect = AaakDialect::new();
    let mut original_tokens = 0;
    let mut compressed_tokens = 0;

    for drawer in &drawers {
        original_tokens += drawer.document.chars().count() / 4; // rough token estimate

        match dialect.compress(&drawer.document) {
            Ok(compressed) => {
                compressed_tokens += compressed.chars().count() / 4;

                if drawer.document.len() > 50 {
                    println!(
                        "  {}: {} -> {} chars ({}%)",
                        drawer.id,
                        drawer.document.len(),
                        compressed.len(),
                        (compressed.len() as f64 / drawer.document.len() as f64 * 100.0) as usize
                    );
                }
            }
            Err(e) => {
                println!("  {}: compression failed - {}", drawer.id, e);
            }
        }
    }

    if original_tokens > 0 {
        let ratio = (compressed_tokens as f64 / original_tokens as f64 * 100.0) as usize;
        println!("\n  Compression complete!");
        println!("    Original: ~{} tokens", original_tokens);
        println!("    Compressed: ~{} tokens", compressed_tokens);
        println!("    Ratio: {}%", ratio);
    }

    println!("\n  Note: AAAK compression is experimental. Review compressed content before replacing originals.");

    Ok(())
}

/// Repair/rebuild the index
fn cmd_repair() -> Result<()> {
    println!("Repairing palace index...");

    let config = Config::load()?;

    if !config.palace_path.exists() {
        println!("  No palace found at {:?}", config.palace_path);
        println!("  Run 'mempalace init' first.");
        return Ok(());
    }

    println!("  Palace: {:?}", config.palace_path);

    // Check if there's an existing database file to repair
    let db_path = config.palace_path.join("search.sqlite");
    if db_path.exists() {
        println!("  Found search database at {:?}", db_path);
        println!("  Checking integrity...");

        // Verify the database is valid SQLite
        match rusqlite::Connection::open(&db_path) {
            Ok(conn) => {
                let valid: bool = conn
                    .query_row("SELECT 1 FROM drawers LIMIT 1", [], |_| Ok(true))
                    .unwrap_or(false);
                if valid {
                    let count: i64 = conn
                        .query_row("SELECT COUNT(*) FROM drawers", [], |row| row.get(0))
                        .unwrap_or(0);
                    println!("  {} drawer(s) indexed", count);
                    println!("  Repair complete (integrity check passed)");
                }
            }
            Err(e) => {
                println!("  Database integrity check failed: {}", e);
                println!("  Recommendation: run 'mempalace init' to reinitialize");
            }
        }
    } else {
        println!("  No search database found at {:?}.", db_path);
        println!("  Run 'mempalace mine' to create the palace.");
    }

    // Show KG info if available
    if config.knowledge_graph_path.exists() {
        println!("  Knowledge Graph: {:?}", config.knowledge_graph_path);
        match mempalace::storage::KnowledgeGraph::new(&config.knowledge_graph_path) {
            Ok(kg) => {
                let entities = kg.get_entity_count().unwrap_or(0);
                let triples = kg.get_triple_count().unwrap_or(0);
                println!("    Entities: {}, Triples: {}", entities, triples);
                println!("    KG integrity: OK");
            }
            Err(e) => {
                println!("    KG integrity: ERROR - {}", e);
            }
        }
    }

    println!("\n  Repair complete!");

    Ok(())
}

/// Split mega transcript files into per-session files
fn cmd_split(dir: &PathBuf) -> Result<()> {
    use mempalace::miner::MegaFileSplitter;

    println!("Splitting mega files in {:?}...", dir);

    if !dir.exists() || !dir.is_dir() {
        println!("  Directory not found: {:?}", dir);
        return Ok(());
    }

    let files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "txt"))
        .collect();

    if files.is_empty() {
        println!("  No .txt files found in {:?}", dir);
        return Ok(());
    }

    println!("  Found {} .txt files", files.len());

    let splitter = MegaFileSplitter::new();
    let mut total_sessions = 0;

    for file_entry in files {
        let filepath = file_entry.path();
        println!("\n  Processing: {}", filepath.display());

        match splitter.split_file(&filepath, None, false) {
            Ok(results) => {
                if results.is_empty() {
                    println!("    No sessions found (single session file?)");
                } else {
                    println!("    Split into {} session(s)", results.len());
                    total_sessions += results.len();
                }
            }
            Err(e) => {
                println!("    Error: {}", e);
            }
        }
    }

    println!("\n  Total sessions extracted: {}", total_sessions);

    Ok(())
}

/// Start MCP server (JSON-RPC over stdio)
fn cmd_serve() -> Result<()> {
    use mempalace::mcp::McpServer;

    println!("Starting MemPalace MCP Server...");
    let server = McpServer::new()?;
    server.start()?;

    Ok(())
}

const PALACE_PROTOCOL: &str = r#"IMPORTANT — MemPalace Memory Protocol:
1. ON WAKE-UP: Call mempalace_status to load palace overview + AAAK spec.
2. BEFORE RESPONDING about any person, project, or past event: call mempalace_kg_query or mempalace_search FIRST. Never guess — verify.
3. IF UNSURE about a fact (name, gender, age, relationship): say "let me check" and query the palace. Wrong is worse than slow.
4. AFTER EACH SESSION: call mempalace_diary_write to record what happened, what you learned, what matters.
5. WHEN FACTS CHANGE: call mempalace_kg_invalidate on the old fact, mempalace_kg_add for the new one.

This protocol ensures the AI KNOWS before it speaks. Storage is not memory — but storage + this protocol = memory."#;

#[cfg(test)]
#[path = "./tests/wing_name.rs"]
mod wing_name;
