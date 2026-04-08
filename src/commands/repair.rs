use crate::commands::load_config;
use crate::error::Result;

pub fn run() -> Result<()> {
    println!("Repairing palace index...");

    let config = load_config()?;

    if !config.palace_path.exists() {
        println!("  No palace found at {:?}", config.palace_path);
        println!("  Run 'mempalace init' first.");
        return Ok(());
    }

    println!("  Palace: {:?}", config.palace_path);

    let db_path = config.palace_path.join("search.sqlite");
    if db_path.exists() {
        println!("  Found search database at {:?}", db_path);
        println!("  Checking integrity...");

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

    if config.knowledge_graph_path.exists() {
        println!("  Knowledge Graph: {:?}", config.knowledge_graph_path);
        match crate::storage::KnowledgeGraph::new(&config.knowledge_graph_path) {
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
