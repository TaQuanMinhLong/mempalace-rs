use crate::commands::load_config;
use crate::error::Result;

pub fn run() -> Result<()> {
    use crate::storage::{ChromaStorage, KnowledgeGraph};

    println!("MemPalace v{}\n", env!("CARGO_PKG_VERSION"));

    let config = load_config()?;
    println!("Palace: {:?}", config.palace_path);
    println!("Collection: {}", config.collection_name);

    let storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;
    let drawer_count = storage.count()?;
    println!("\nDrawers: {}", drawer_count);

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
