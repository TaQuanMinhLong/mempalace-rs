use crate::commands::load_config;
use crate::error::Result;

const PALACE_PROTOCOL: &str = r#"IMPORTANT — MemPalace Memory Protocol:
1. ON WAKE-UP: Call mempalace_status to load palace overview + AAAK spec.
2. BEFORE RESPONDING about any person, project, or past event: call mempalace_kg_query or mempalace_search FIRST. Never guess — verify.
3. IF UNSURE about a fact (name, gender, age, relationship): say "let me check" and query the palace. Wrong is worse than slow.
4. AFTER EACH SESSION: call mempalace_diary_write to record what happened, what you learned, what matters.
5. WHEN FACTS CHANGE: call mempalace_kg_invalidate on the old fact, mempalace_kg_add for the new one.

This protocol ensures the AI KNOWS before it speaks. Storage is not memory — but storage + this protocol = memory."#;

pub fn run(wing: Option<&str>) -> Result<()> {
    println!("Waking up (wing: {:?})...", wing);

    let config = load_config()?;
    println!("  Palace: {:?}", config.palace_path);

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
