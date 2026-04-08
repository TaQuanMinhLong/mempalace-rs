use std::path::PathBuf;

use crate::commands::{load_config, wing_name_from_dir};
use crate::error::Result;

pub fn run(dir: &PathBuf, mode: &str, agent: &str) -> Result<()> {
    use crate::miner::{ConvoMiner, FileMiner};
    use crate::storage::ChromaStorage;

    println!("Mining {:?} in {} mode (agent: {})", dir, mode, agent);

    if !dir.exists() || !dir.is_dir() {
        println!("  Directory not found: {:?}", dir);
        return Ok(());
    }

    let config = load_config()?;
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
