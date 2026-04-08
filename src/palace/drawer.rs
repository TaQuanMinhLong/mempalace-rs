//! Drawer - verbatim content stored in ChromaDB

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Mode of ingestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum IngestMode {
    #[default]
    Projects,
    Convos,
}

/// Metadata associated with a drawer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawerMetadata {
    pub wing: String,
    pub room: String,
    pub source_file: String,
    pub chunk_index: usize,
    pub added_by: String,
    pub filed_at: DateTime<Utc>,
    pub ingest_mode: IngestMode,
    /// Importance score (for L1 layer sorting)
    #[serde(default)]
    pub importance: Option<f64>,
    /// Emotional weight (for L1 layer sorting)
    #[serde(default)]
    pub emotional_weight: Option<f64>,
}

impl DrawerMetadata {
    pub fn new(
        wing: impl Into<String>,
        room: impl Into<String>,
        source_file: impl Into<String>,
        chunk_index: usize,
        added_by: impl Into<String>,
        ingest_mode: IngestMode,
    ) -> Self {
        Self {
            wing: wing.into(),
            room: room.into(),
            source_file: source_file.into(),
            chunk_index,
            added_by: added_by.into(),
            filed_at: Utc::now(),
            ingest_mode,
            importance: None,
            emotional_weight: None,
        }
    }
}

/// Drawer - a chunk of verbatim content stored in the memory palace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drawer {
    pub id: String,
    pub document: String,
    pub metadata: DrawerMetadata,
}

impl Drawer {
    pub fn new(
        id: impl Into<String>,
        document: impl Into<String>,
        metadata: DrawerMetadata,
    ) -> Self {
        Self {
            id: id.into(),
            document: document.into(),
            metadata,
        }
    }

    /// Generate a drawer ID from content hash
    pub fn generate_id(wing: &str, room: &str, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(format!("{}_{}_{}", wing, room, content).as_bytes());
        let result = hasher.finalize();
        format!("drawer_{}_{}_{}", wing, room, hex::encode(&result[..8]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = Drawer::generate_id("wing_kai", "auth", "hello world");
        let id2 = Drawer::generate_id("wing_kai", "auth", "hello world");
        let id3 = Drawer::generate_id("wing_kai", "auth", "different");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert!(id1.starts_with("drawer_wing_kai_auth_"));
    }

    #[test]
    fn test_drawer_metadata() {
        let metadata = DrawerMetadata::new(
            "wing_kai",
            "auth",
            "/path/to/file.py",
            0,
            "cli",
            IngestMode::Projects,
        );

        assert_eq!(metadata.wing, "wing_kai");
        assert_eq!(metadata.room, "auth");
        assert_eq!(metadata.ingest_mode, IngestMode::Projects);
    }
}
