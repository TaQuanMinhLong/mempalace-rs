//! Knowledge graph operations - wraps SQLite KG
//!
//! Port from Python knowledge_graph.py. Provides temporal entity-relationship
//! graph with:
//!   - Entity nodes (people, projects, tools, concepts)
//!   - Typed relationship edges (daughter_of, does, loves, works_on, etc.)
//!   - Temporal validity (valid_from → valid_to — knows WHEN facts are true)
//!   - Closet references (links back to the verbatim memory)

// Re-exports from storage
pub use crate::storage::{Entity, EntityType, KnowledgeGraph, Triple};

use crate::error::Result;
use chrono::NaiveDate;

impl KnowledgeGraph {
    /// Search for entities by name (partial match)
    pub fn search_entities(&self, query: &str) -> Result<Vec<Entity>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, properties, created_at
             FROM entities
             WHERE LOWER(name) LIKE ?
             ORDER BY name",
        )?;

        let entities = stmt
            .query_map([&pattern], |row| {
                let properties_str: String = row.get(3)?;
                let created_str: String = row.get(4)?;
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: EntityType::from(&row.get::<_, String>(2)?),
                    properties: serde_json::from_str(&properties_str)
                        .unwrap_or(serde_json::Value::Object(Default::default())),
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entities)
    }

    /// Get triples that were valid at a specific point in time.
    ///
    /// A triple is valid at `as_of` if:
    /// - `valid_from` is null or <= `as_of`, AND
    /// - `valid_to` is null or >= `as_of`
    pub fn get_active_triples(&self, as_of: NaiveDate) -> Result<Vec<Triple>> {
        let as_of_str = as_of.to_string();
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, subject, predicate, object, valid_from, valid_to,
                   confidence, source_closet, source_file, extracted_at
            FROM triples
            WHERE (valid_from IS NULL OR valid_from <= ?1)
              AND (valid_to IS NULL OR valid_to >= ?1)
            ORDER BY valid_from ASC NULLS LAST
            "#,
        )?;

        let triples = stmt
            .query_map([&as_of_str], |row| {
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
                    extracted_at: chrono::DateTime::parse_from_rfc3339(&extracted_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(triples)
    }

    /// Get all current (non-expired) triples
    pub fn get_current_triples(&self) -> Result<Vec<Triple>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, subject, predicate, object, valid_from, valid_to,
                   confidence, source_closet, source_file, extracted_at
            FROM triples
            WHERE valid_to IS NULL
            ORDER BY extracted_at DESC
            "#,
        )?;

        let triples = stmt
            .query_map([], |row| {
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
                    extracted_at: chrono::DateTime::parse_from_rfc3339(&extracted_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(triples)
    }

    /// Get graph statistics
    pub fn stats(&self) -> Result<GraphStats> {
        let entities: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))?;
        let triples: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM triples", [], |row| row.get(0))?;
        let current: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM triples WHERE valid_to IS NULL",
            [],
            |row| row.get(0),
        )?;

        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT predicate FROM triples ORDER BY predicate")?;
        let predicates: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(GraphStats {
            entities: entities as usize,
            triples: triples as usize,
            current_facts: current as usize,
            expired_facts: (triples - current) as usize,
            relationship_types: predicates,
        })
    }
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub entities: usize,
    pub triples: usize,
    pub current_facts: usize,
    pub expired_facts: usize,
    pub relationship_types: Vec<String>,
}

impl GraphStats {
    #[inline]
    pub fn new(
        entities: usize,
        triples: usize,
        current_facts: usize,
        expired_facts: usize,
        relationship_types: Vec<String>,
    ) -> Self {
        Self {
            entities,
            triples,
            current_facts,
            expired_facts,
            relationship_types,
        }
    }
}

#[cfg(test)]
#[path = "../tests/graph_knowledge.rs"]
mod tests;
