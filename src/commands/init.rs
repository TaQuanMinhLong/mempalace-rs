use std::path::PathBuf;

use crate::commands::load_config;
use crate::error::Result;

pub fn run(dir: &PathBuf) -> Result<()> {
    println!("Initializing mempalace in {:?}...", dir);

    let config_file = crate::config::Config::init()?;
    println!("  Config created: {:?}", config_file);

    let config = load_config()?;
    std::fs::create_dir_all(&config.palace_path)?;
    println!("  Palace directory: {:?}", config.palace_path);

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
