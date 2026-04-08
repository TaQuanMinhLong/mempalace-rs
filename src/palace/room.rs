//! Room - topic within a wing

use serde::{Deserialize, Serialize};

/// Room - a topic/idea within a wing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub wing: String,
    pub keywords: Vec<String>,
}

impl Room {
    pub fn new(name: impl Into<String>, wing: impl Into<String>, keywords: Vec<String>) -> Self {
        Self {
            name: name.into(),
            wing: wing.into(),
            keywords,
        }
    }

    /// Create a slug from a room name (for use in ChromaDB queries)
    pub fn slugify(name: &str) -> String {
        name.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[cfg(test)]
#[path = "../tests/palace_room.rs"]
mod tests;
