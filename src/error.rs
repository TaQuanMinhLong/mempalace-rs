//! Error types for mempalace.

use thiserror::Error;

pub type Result<T, E = MempalaceError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum MempalaceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Mining error: {0}")]
    Mining(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Entity error: {0}")]
    Entity(String),

    #[error("Normalization error: {0}")]
    Normalization(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Not found: {0}")]
    NotFound(String),
}
