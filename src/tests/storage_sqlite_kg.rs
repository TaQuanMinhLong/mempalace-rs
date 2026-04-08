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
