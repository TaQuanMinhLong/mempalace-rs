use crate::commands::load_config;
use crate::error::Result;

pub async fn run(query: &str, wing: Option<&str>, room: Option<&str>, limit: usize) -> Result<()> {
    use crate::search::SemanticSearcher;
    use crate::storage::ChromaStorage;

    println!(
        "Searching '{}' (wing: {:?}, room: {:?}, limit: {})",
        query, wing, room, limit
    );

    if query.is_empty() {
        println!("  Query cannot be empty");
        return Ok(());
    }

    let config = load_config()?;
    let storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;
    let searcher = SemanticSearcher::new(std::sync::Arc::new(tokio::sync::Mutex::new(storage)));

    match searcher.search(query, wing, room, limit).await {
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
