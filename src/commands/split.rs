use std::path::PathBuf;

use crate::error::Result;

pub fn run(dir: &PathBuf) -> Result<()> {
    use crate::miner::MegaFileSplitter;

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
