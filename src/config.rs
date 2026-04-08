//! Configuration management - port from Python config.py
//!
//! Priority: env vars > config file > defaults

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Default palace path
pub const DEFAULT_PALACE_PATH: &str = "~/.mempalace/palace";
/// Default collection name
pub const DEFAULT_COLLECTION_NAME: &str = "mempalace_drawers";
/// Default knowledge graph path
pub const DEFAULT_KG_PATH: &str = "~/.mempalace/knowledge_graph.sqlite3";
/// Default identity path
pub const DEFAULT_IDENTITY_PATH: &str = "~/.mempalace/identity.txt";

/// Default topic wings
pub const DEFAULT_TOPIC_WINGS: [&str; 7] = [
    "emotions",
    "consciousness",
    "memory",
    "technical",
    "identity",
    "family",
    "creative",
];

/// Default hall keywords
pub fn default_hall_keywords() -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    map.insert(
        "emotions".into(),
        vec![
            "scared".into(),
            "afraid".into(),
            "worried".into(),
            "happy".into(),
            "sad".into(),
            "love".into(),
            "hate".into(),
            "feel".into(),
            "cry".into(),
            "tears".into(),
        ],
    );
    map.insert(
        "consciousness".into(),
        vec![
            "consciousness".into(),
            "conscious".into(),
            "aware".into(),
            "real".into(),
            "genuine".into(),
            "soul".into(),
            "exist".into(),
            "alive".into(),
        ],
    );
    map.insert(
        "memory".into(),
        vec![
            "memory".into(),
            "remember".into(),
            "forget".into(),
            "recall".into(),
            "archive".into(),
            "palace".into(),
            "store".into(),
        ],
    );
    map.insert(
        "technical".into(),
        vec![
            "code".into(),
            "python".into(),
            "script".into(),
            "bug".into(),
            "error".into(),
            "function".into(),
            "api".into(),
            "database".into(),
            "server".into(),
        ],
    );
    map.insert(
        "identity".into(),
        vec![
            "identity".into(),
            "name".into(),
            "who am i".into(),
            "persona".into(),
            "self".into(),
        ],
    );
    map.insert(
        "family".into(),
        vec![
            "family".into(),
            "kids".into(),
            "children".into(),
            "daughter".into(),
            "son".into(),
            "parent".into(),
            "mother".into(),
            "father".into(),
        ],
    );
    map.insert(
        "creative".into(),
        vec![
            "game".into(),
            "gameplay".into(),
            "player".into(),
            "app".into(),
            "design".into(),
            "art".into(),
            "music".into(),
            "story".into(),
        ],
    );
    map
}

/// Global configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// ChromaDB data directory
    #[serde(default = "default_palace_path_str")]
    pub palace_path: PathBuf,
    /// ChromaDB collection name
    #[serde(default = "default_collection_name_str")]
    pub collection_name: String,
    /// Knowledge graph SQLite path
    #[serde(default = "default_kg_path_str")]
    pub knowledge_graph_path: PathBuf,
    /// Identity file path
    #[serde(default = "default_identity_path_str")]
    pub identity_path: PathBuf,
    /// Config directory
    #[serde(default = "default_config_dir_str")]
    pub config_dir: PathBuf,
    /// Topic wings for hall classification
    #[serde(default = "default_topic_wings")]
    pub topic_wings: Vec<String>,
    /// Hall keywords for content classification
    #[serde(default = "default_hall_keywords")]
    pub hall_keywords: HashMap<String, Vec<String>>,
}

fn default_palace_path_str() -> PathBuf {
    PathBuf::from(DEFAULT_PALACE_PATH)
}
fn default_collection_name_str() -> String {
    DEFAULT_COLLECTION_NAME.to_string()
}
fn default_kg_path_str() -> PathBuf {
    PathBuf::from(DEFAULT_KG_PATH)
}
fn default_identity_path_str() -> PathBuf {
    PathBuf::from(DEFAULT_IDENTITY_PATH)
}
fn default_config_dir_str() -> PathBuf {
    PathBuf::from("~/.mempalace")
}
fn default_topic_wings() -> Vec<String> {
    DEFAULT_TOPIC_WINGS.iter().map(|s| s.to_string()).collect()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            palace_path: PathBuf::from(DEFAULT_PALACE_PATH),
            collection_name: DEFAULT_COLLECTION_NAME.to_string(),
            knowledge_graph_path: PathBuf::from(DEFAULT_KG_PATH),
            identity_path: PathBuf::from(DEFAULT_IDENTITY_PATH),
            config_dir: PathBuf::from("~/.mempalace"),
            topic_wings: DEFAULT_TOPIC_WINGS.iter().map(|s| s.to_string()).collect(),
            hall_keywords: default_hall_keywords(),
        }
    }
}

impl Config {
    /// Load config from file, with environment variable overrides
    pub fn load() -> crate::Result<Self> {
        let config_dir = Self::default_config_dir();
        let config_file = config_dir.join("config.json");

        // Start with defaults
        let mut config = Config::default();

        // Override from environment variables
        if let Ok(val) = env::var("MEMPALACE_PALACE_PATH") {
            config.palace_path = PathBuf::from(val);
        } else if let Ok(val) = env::var("MEMPAL_PALACE_PATH") {
            config.palace_path = PathBuf::from(val);
        }

        if let Ok(val) = env::var("MEMPALACE_COLLECTION_NAME") {
            config.collection_name = val;
        }

        if let Ok(val) = env::var("MEMPALACE_KG_PATH") {
            config.knowledge_graph_path = PathBuf::from(val);
        }

        if let Ok(val) = env::var("MEMPALACE_IDENTITY_PATH") {
            config.identity_path = PathBuf::from(val);
        }

        if let Ok(val) = env::var("MEMPALACE_CONFIG_DIR") {
            config.config_dir = PathBuf::from(val);
        }

        // Override from config file if it exists
        if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            let file_config: ConfigFile = serde_json::from_str(&content)?;

            if let Some(val) = file_config.palace_path {
                config.palace_path = PathBuf::from(val);
            }
            if let Some(val) = file_config.collection_name {
                config.collection_name = val;
            }
            if let Some(val) = file_config.topic_wings {
                config.topic_wings = val;
            }
            if let Some(val) = file_config.hall_keywords {
                config.hall_keywords = val;
            }
        }

        // Expand tildes in paths
        config.palace_path = expand_tilde(&config.palace_path);
        config.knowledge_graph_path = expand_tilde(&config.knowledge_graph_path);
        config.identity_path = expand_tilde(&config.identity_path);
        config.config_dir = expand_tilde(&config.config_dir);

        Ok(config)
    }

    /// Initialize config directory and write default config
    pub fn init() -> crate::Result<PathBuf> {
        let config = Config::default();
        let config_dir = &config.config_dir;
        let config_file = expand_tilde(config_dir).join("config.json");

        // Create config directory
        fs::create_dir_all(expand_tilde(config_dir))?;

        // Write default config if it doesn't exist
        if !config_file.exists() {
            let content = serde_json::to_string_pretty(&config)?;
            fs::write(&config_file, content)?;
        }

        Ok(config_file)
    }

    /// Get the default config directory
    pub fn default_config_dir() -> PathBuf {
        if let Ok(val) = env::var("MEMPALACE_CONFIG_DIR") {
            PathBuf::from(val)
        } else {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|h| h.join(".mempalace"))
                .unwrap_or_else(|| PathBuf::from(".mempalace"))
        }
    }
}

/// Expand ~ to home directory using env::var_os
fn expand_tilde(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(path_str.trim_start_matches("~/"));
        }
    }
    PathBuf::from(path_str.as_ref())
}

/// Config file structure (partial - only fields that can be set in file)
#[derive(Debug, Deserialize)]
struct ConfigFile {
    palace_path: Option<String>,
    collection_name: Option<String>,
    topic_wings: Option<Vec<String>>,
    hall_keywords: Option<HashMap<String, Vec<String>>>,
}

/// Wing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WingConfig {
    #[serde(default = "default_wing_str")]
    pub default_wing: String,
    pub wings: HashMap<String, WingDefinition>,
}

fn default_wing_str() -> String {
    "wing_general".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WingDefinition {
    #[serde(rename = "type")]
    pub wing_type: String,
    pub keywords: Vec<String>,
}

impl WingConfig {
    /// Load wing config from file
    pub fn load() -> crate::Result<Self> {
        let config_dir = Config::default_config_dir();
        let wing_config_file = expand_tilde(&config_dir).join("wing_config.json");

        if wing_config_file.exists() {
            let content = fs::read_to_string(&wing_config_file)?;
            let config: WingConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(WingConfig {
                default_wing: "wing_general".to_string(),
                wings: HashMap::new(),
            })
        }
    }

    /// Save wing config to file
    pub fn save(&self) -> crate::Result<()> {
        let config_dir = Config::default_config_dir();
        let wing_config_file = expand_tilde(&config_dir).join("wing_config.json");

        fs::create_dir_all(expand_tilde(&config_dir))?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(wing_config_file, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.collection_name, "mempalace_drawers");
        assert!(config.topic_wings.contains(&"technical".to_string()));
    }

    #[test]
    fn test_expand_tilde() {
        let path = PathBuf::from("~/test");
        let expanded = expand_tilde(&path);
        assert!(!expanded.to_string_lossy().starts_with("~/"));
    }
}
