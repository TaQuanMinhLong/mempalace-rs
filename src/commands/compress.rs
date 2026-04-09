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
    let mut original_tokens_measured = 0usize;
    let mut compressed_tokens_measured = 0usize;
    let mut original_tokens_est = 0usize;
    let mut compressed_tokens_est = 0usize;

    for drawer in &drawers {
        match dialect.compress(&drawer.document) {
            Ok(compressed) => {
                let stats = dialect.compression_stats(&drawer.document, &compressed);
                original_tokens_measured +=
                    stats["original_tokens_measured"].as_u64().unwrap_or(0) as usize;
                compressed_tokens_measured +=
                    stats["summary_tokens_measured"].as_u64().unwrap_or(0) as usize;
                original_tokens_est += stats["original_tokens_est"].as_u64().unwrap_or(0) as usize;
                compressed_tokens_est += stats["summary_tokens_est"].as_u64().unwrap_or(0) as usize;

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

    if original_tokens_measured > 0 {
        let measured_ratio =
            (compressed_tokens_measured as f64 / original_tokens_measured as f64 * 100.0) as usize;
        let estimated_ratio =
            (compressed_tokens_est as f64 / original_tokens_est.max(1) as f64 * 100.0) as usize;
        println!("\n  Compression complete!");
        println!(
            "    Measured by local tokenizer (not model-accurate): {} -> {} tokens ({}%)",
            original_tokens_measured, compressed_tokens_measured, measured_ratio
        );
        println!(
            "    Estimated (openai heuristic): {} -> {} tokens ({}%)",
            original_tokens_est, compressed_tokens_est, estimated_ratio
        );
    }

    println!(
        "\n  Note: AAAK compression is lossy. Local measured counts come from a deterministic local tokenizer and are not model-accurate; vendor-specific token counts remain estimates until dedicated tokenizers are added."
    );

    Ok(())
}
