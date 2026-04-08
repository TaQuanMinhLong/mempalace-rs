//! SQLite knowledge graph storage

use crate::error::Result;
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Knowledge graph stored in SQLite
#[derive(Debug)]
pub struct KnowledgeGraph {
    pub conn: Connection,
}

/// Entity in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: EntityType,
    pub properties: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Type of entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Person,
    Project,
    Place,
    Concept,
    Unknown,
}

impl EntityType {
    pub fn as_str(&self) -> &str {
        match self {
            EntityType::Person => "person",
            EntityType::Project => "project",
            EntityType::Place => "place",
            EntityType::Concept => "concept",
            EntityType::Unknown => "unknown",
        }
    }
}

impl From<&str> for EntityType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "person" => EntityType::Person,
            "project" => EntityType::Project,
            "place" => EntityType::Place,
            "concept" => EntityType::Concept,
            _ => EntityType::Unknown,
        }
    }
}

impl From<String> for EntityType {
    fn from(s: String) -> Self {
        EntityType::from(s.as_str())
    }
}

impl From<&String> for EntityType {
    fn from(s: &String) -> Self {
        EntityType::from(s.as_str())
    }
}

/// Triple - temporal relationship edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Triple {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from: Option<NaiveDate>,
    pub valid_to: Option<NaiveDate>,
    pub confidence: f64,
    pub source_closet: String,
    pub source_file: String,
    pub extracted_at: DateTime<Utc>,
}

impl KnowledgeGraph {
    /// Create a new knowledge graph at the given path
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Create tables if they don't exist
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                properties TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS triples (
                id TEXT PRIMARY KEY,
                subject TEXT NOT NULL,
                predicate TEXT NOT NULL,
                object TEXT NOT NULL,
                valid_from TEXT,
                valid_to TEXT,
                confidence REAL NOT NULL DEFAULT 1.0,
                source_closet TEXT NOT NULL DEFAULT '',
                source_file TEXT NOT NULL DEFAULT '',
                extracted_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_triples_subject ON triples(subject);
            CREATE INDEX IF NOT EXISTS idx_triples_object ON triples(object);
            "#,
        )?;

        // Enable WAL mode
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        Ok(Self { conn })
    }

    /// Upsert an entity
    pub fn upsert_entity(&self, entity: &Entity) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO entities (id, name, entity_type, properties, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                entity_type = excluded.entity_type,
                properties = excluded.properties
            "#,
            params![
                entity.id,
                entity.name,
                entity.entity_type.as_str(),
                serde_json::to_string(&entity.properties)?,
                entity.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get an entity by ID
    pub fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, properties, created_at FROM entities WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            let properties_str: String = row.get(3)?;
            let created_str: String = row.get(4)?;
            Ok(Entity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: EntityType::from(&row.get::<_, String>(2)?),
                properties: serde_json::from_str(&properties_str)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        });

        match result {
            Ok(entity) => Ok(Some(entity)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::MempalaceError::Database(e)),
        }
    }

    /// Upsert a triple
    pub fn upsert_triple(&self, triple: &Triple) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO triples (id, subject, predicate, object, valid_from, valid_to,
                               confidence, source_closet, source_file, extracted_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                subject = excluded.subject,
                predicate = excluded.predicate,
                object = excluded.object,
                valid_from = excluded.valid_from,
                valid_to = excluded.valid_to,
                confidence = excluded.confidence,
                source_closet = excluded.source_closet,
                source_file = excluded.source_file
            "#,
            params![
                triple.id,
                triple.subject,
                triple.predicate,
                triple.object,
                triple.valid_from.map(|d| d.to_string()),
                triple.valid_to.map(|d| d.to_string()),
                triple.confidence,
                triple.source_closet,
                triple.source_file,
                triple.extracted_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get triples for an entity
    pub fn get_triples_for_entity(&self, entity_id: &str) -> Result<Vec<Triple>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, subject, predicate, object, valid_from, valid_to,
                   confidence, source_closet, source_file, extracted_at
            FROM triples
            WHERE subject = ?1 OR object = ?1
            "#,
        )?;

        let triples = stmt
            .query_map(params![entity_id], |row| {
                let valid_from_str: Option<String> = row.get(4)?;
                let valid_to_str: Option<String> = row.get(5)?;
                let extracted_str: String = row.get(9)?;

                Ok(Triple {
                    id: row.get(0)?,
                    subject: row.get(1)?,
                    predicate: row.get(2)?,
                    object: row.get(3)?,
                    valid_from: valid_from_str
                        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                    valid_to: valid_to_str
                        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                    confidence: row.get(6)?,
                    source_closet: row.get(7)?,
                    source_file: row.get(8)?,
                    extracted_at: DateTime::parse_from_rfc3339(&extracted_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(triples)
    }

    /// Get total entity count
    pub fn get_entity_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get total triple count
    pub fn get_triple_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM triples", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_knowledge_graph() {
        let dir = tempdir().unwrap();
        let kg_path = dir.path().join("test_kg.sqlite3");

        let kg = KnowledgeGraph::new(&kg_path).unwrap();

        let entity = Entity {
            id: "test_entity".to_string(),
            name: "Test Entity".to_string(),
            entity_type: EntityType::Person,
            properties: serde_json::json!({"key": "value"}),
            created_at: Utc::now(),
        };

        kg.upsert_entity(&entity).unwrap();

        let found = kg.get_entity("test_entity").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Entity");
    }
}
