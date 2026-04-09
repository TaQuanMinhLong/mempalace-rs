//! SQLite FTS5 storage - embedded full-text search for semantic memory
//!
//! Uses SQLite FTS5 with bm25 ranking for full-text search. This replaces
//! the previous LanceDB approach which was not implemented. FTS5 provides
//! battle-tested full-text search with relevance ranking, file-based persistence,
//! and zero additional dependencies.

use crate::error::{MempalaceError, Result};
use crate::layers::SearchHit;
use crate::palace::Drawer;
use rusqlite::{params, Connection};
use std::path::Path;

/// Search storage using SQLite FTS5
#[derive(Debug)]
pub struct ChromaStorage {
    conn: Connection,
    palace_path: std::path::PathBuf,
    collection_name: String,
}

impl ChromaStorage {
    /// Create a new FTS5-backed ChromaStorage
    pub fn new(palace_path: &Path, collection_name: &str) -> Result<Self> {
        let db_path = palace_path.join("search.sqlite");
        std::fs::create_dir_all(palace_path)?;

        let conn = Connection::open(&db_path)?;

        // Enable WAL mode for concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        // Create drawers table for persisted drawer storage
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS drawers (
                id TEXT PRIMARY KEY,
                document TEXT NOT NULL,
                wing TEXT NOT NULL,
                room TEXT NOT NULL,
                source_file TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
            params![],
        )?;

        // Create FTS5 virtual table for full-text search
        // Self-contained FTS table (not external content) to allow upserts
        conn.execute(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS drawers_fts USING fts5(
                id,
                document,
                wing,
                room,
                source_file,
                tokenize='porter unicode61'
            )
            "#,
            params![],
        )?;

        Ok(Self {
            conn,
            palace_path: palace_path.to_path_buf(),
            collection_name: collection_name.to_string(),
        })
    }

    /// Add a drawer to storage and FTS index
    pub fn add_drawer(&mut self, drawer: &Drawer) -> Result<()> {
        let metadata_json =
            serde_json::to_string(&drawer.metadata).map_err(MempalaceError::Json)?;

        // Insert into drawers table
        self.conn.execute(
            r#"
            INSERT INTO drawers (id, document, wing, room, source_file, metadata, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                document = excluded.document,
                wing = excluded.wing,
                room = excluded.room,
                source_file = excluded.source_file,
                metadata = excluded.metadata
            "#,
            params![
                drawer.id,
                drawer.document,
                drawer.metadata.wing,
                drawer.metadata.room,
                drawer.metadata.source_file,
                metadata_json,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        // Insert into FTS index (self-contained FTS table)
        self.conn.execute(
            r#"
            INSERT INTO drawers_fts (id, document, wing, room, source_file)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                drawer.id,
                drawer.document,
                drawer.metadata.wing,
                drawer.metadata.room,
                drawer.metadata.source_file,
            ],
        )?;

        Ok(())
    }

    /// Get top drawers by importance/recency
    pub fn get_top_drawers(&self, limit: usize, wing: Option<&str>) -> Vec<Drawer> {
        let query = if wing.is_some() {
            "SELECT id, document, wing, room, source_file, metadata FROM drawers WHERE wing = ?1 ORDER BY created_at DESC LIMIT ?2"
        } else {
            "SELECT id, document, wing, room, source_file, metadata FROM drawers ORDER BY created_at DESC LIMIT ?1"
        };

        let mut stmt = match self.conn.prepare(query) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = if let Some(w) = wing {
            stmt.query_map(params![w, limit as i64], Self::row_to_drawer)
        } else {
            stmt.query_map(params![limit as i64], Self::row_to_drawer)
        };

        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Get drawers filtered by wing and/or room
    pub fn get_drawers_by_filter(
        &self,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Vec<Drawer> {
        let mut sql = String::from(
            "SELECT id, document, wing, room, source_file, metadata FROM drawers WHERE 1=1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(w) = wing {
            sql.push_str(" AND wing = ?");
            params_vec.push(Box::new(w.to_string()));
        }
        if let Some(r) = room {
            sql.push_str(" AND room = ?");
            params_vec.push(Box::new(r.to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");
        params_vec.push(Box::new(limit as i64));

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = stmt.query_map(params_refs.as_slice(), Self::row_to_drawer);
        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Search for drawers using FTS5 with bm25 ranking
    pub fn search(
        &self,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Vec<SearchHit> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        // Build FTS5 query - escape special chars and use prefix matching
        let fts_query = Self::build_fts_query(query);
        if fts_query.is_empty() {
            return Vec::new();
        }

        // Build the search SQL with optional wing/room filters applied to FTS results
        let mut sql = r#"
            SELECT d.id, d.document, d.wing, d.room, d.source_file, d.metadata,
                   bm25(drawers_fts) as rank
            FROM drawers_fts
            JOIN drawers d ON drawers_fts.id = d.id
            WHERE drawers_fts MATCH ?1
            "#
        .to_string();

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(fts_query)];

        if let Some(w) = wing {
            sql.push_str(" AND d.wing = ?");
            params_vec.push(Box::new(w.to_string()));
        }
        if let Some(r) = room {
            sql.push_str(" AND d.room = ?");
            params_vec.push(Box::new(r.to_string()));
        }

        sql.push_str(" ORDER BY rank LIMIT ?");
        params_vec.push(Box::new(limit as i64));

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("FTS search prepare error: {}", e);
                return Vec::new();
            }
        };

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let rank: f64 = row.get(6).unwrap_or(0.0);
            // bm25 returns negative scores (more negative = better match)
            // Convert to a positive similarity score (higher = better)
            // bm25 values are typically in range [-20, 0]; normalize to [0, 1]
            let similarity = ((rank / 20.0).exp()).clamp(0.0, 1.0);

            Ok(SearchHit {
                document_id: Some(row.get(0)?),
                text: row.get(1)?,
                wing: row.get(2)?,
                room: row.get(3)?,
                source_file: row.get(4)?,
                similarity,
                distance: Some(-rank),
            })
        });

        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                eprintln!("FTS search query error: {}", e);
                Vec::new()
            }
        }
    }

    /// Build an FTS5 query string from user input
    fn build_fts_query(input: &str) -> String {
        let input = input.trim();
        if input.is_empty() {
            return String::new();
        }

        // Escape FTS5 special characters: " * ^ - :
        let escaped = input
            .replace('"', "\"\"")
            .replace(['*', '^'], "")
            .replace(['-', ':'], " ");

        // Split into terms and build prefix-matched query
        let terms: Vec<&str> = escaped.split_whitespace().collect();
        if terms.is_empty() {
            return String::new();
        }

        // Use prefix matching (term*) for each word
        // FTS5 prefix matching: append * to each term, no quotes needed
        terms
            .iter()
            .map(|t| format!("{}*", t))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Delete a drawer by ID
    pub fn delete_drawer(&mut self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM drawers WHERE id = ?1", params![id])?;

        // Remove from FTS index
        let _ = self
            .conn
            .execute("DELETE FROM drawers_fts WHERE id = ?1", params![id]);

        Ok(())
    }

    /// Count total drawers
    pub fn count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM drawers", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get all drawers
    pub fn get_all_drawers(&self) -> Vec<Drawer> {
        let mut stmt = match self
            .conn
            .prepare("SELECT id, document, wing, room, source_file, metadata FROM drawers")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = stmt.query_map([], Self::row_to_drawer);
        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Get palace path
    pub fn palace_path(&self) -> &Path {
        &self.palace_path
    }

    /// Get collection name
    pub fn collection_name(&self) -> &str {
        &self.collection_name
    }

    /// Helper to convert a SQL row to a Drawer
    fn row_to_drawer(row: &rusqlite::Row) -> rusqlite::Result<Drawer> {
        let metadata_str: String = row.get(5)?;
        let metadata: crate::palace::DrawerMetadata = serde_json::from_str(&metadata_str)
            .unwrap_or_else(|_| {
                crate::palace::DrawerMetadata::new(
                    row.get::<_, String>(2).unwrap_or_default(),
                    row.get::<_, String>(3).unwrap_or_default(),
                    row.get::<_, String>(4).unwrap_or_default(),
                    0,
                    "",
                    crate::palace::IngestMode::Projects,
                )
            });

        Ok(Drawer {
            id: row.get(0)?,
            document: row.get(1)?,
            metadata,
        })
    }
}

#[cfg(test)]
#[path = "../tests/storage_lancedb.rs"]
mod tests;
