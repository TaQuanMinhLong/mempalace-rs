use crate::commands::load_config;
use crate::error::Result;

pub fn run(wing: &str, room: Option<&str>) -> Result<()> {
    use crate::dialect::aaak::AaakDialect;
    use crate::storage::ChromaStorage;

    println!("Compressing wing '{}'...", wing);

    let config = load_config()?;
    let storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;

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
        original_tokens += drawer.document.chars().count() / 4;

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
