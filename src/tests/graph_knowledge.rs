use super::*;
use chrono::NaiveDate;
use tempfile::tempdir;

#[test]
fn test_search_entities() {
    let dir = tempdir().unwrap();
    let kg_path = dir.path().join("test_kg.sqlite3");
    let kg = KnowledgeGraph::new(&kg_path).unwrap();

    kg.upsert_entity(&Entity {
        id: "alice".to_string(),
        name: "Alice".to_string(),
        entity_type: EntityType::Person,
        properties: serde_json::json!({}),
        created_at: chrono::Utc::now(),
    })
    .unwrap();

    kg.upsert_entity(&Entity {
        id: "bob".to_string(),
        name: "Bob".to_string(),
        entity_type: EntityType::Person,
        properties: serde_json::json!({}),
        created_at: chrono::Utc::now(),
    })
    .unwrap();

    let results = kg.search_entities("ali").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");

    let results = kg.search_entities("").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_get_active_triples() {
    let dir = tempdir().unwrap();
    let kg_path = dir.path().join("test_kg.sqlite3");
    let kg = KnowledgeGraph::new(&kg_path).unwrap();

    let now = chrono::Utc::now();
    let past = NaiveDate::parse_from_str("2020-01-01", "%Y-%m-%d").unwrap();

    kg.upsert_triple(&Triple {
        id: "t1".to_string(),
        subject: "alice".to_string(),
        predicate: "child_of".to_string(),
        object: "bob".to_string(),
        valid_from: Some(past),
        valid_to: None,
        confidence: 1.0,
        source_closet: "test".to_string(),
        source_file: "test.rs".to_string(),
        extracted_at: now,
    })
    .unwrap();

    let as_of = NaiveDate::parse_from_str("2025-06-15", "%Y-%m-%d").unwrap();
    let active = kg.get_active_triples(as_of).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].subject, "alice");
}

#[test]
fn test_graph_stats() {
    let dir = tempdir().unwrap();
    let kg_path = dir.path().join("test_kg.sqlite3");
    let kg = KnowledgeGraph::new(&kg_path).unwrap();

    kg.upsert_entity(&Entity {
        id: "test".to_string(),
        name: "Test".to_string(),
        entity_type: EntityType::Concept,
        properties: serde_json::json!({}),
        created_at: chrono::Utc::now(),
    })
    .unwrap();

    let stats = kg.stats().unwrap();
    assert_eq!(stats.entities, 1);
    assert_eq!(stats.triples, 0);
}
