use super::*;
use crate::layers::SearchHit;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::Mutex;

#[test]
fn test_search_result_creation() {
    let hit = SearchHit {
        document_id: Some("doc_1".to_string()),
        text: "Test document content".to_string(),
        wing: "wing_test".to_string(),
        room: "room_test".to_string(),
        source_file: "/path/to/file.py".to_string(),
        similarity: 0.95,
        distance: Some(0.05),
    };
    let result = SearchResult { hit };

    assert_eq!(result.hit.similarity, 0.95);
    assert_eq!(result.hit.text, "Test document content");
}

#[tokio::test]
async fn test_search_empty_query() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let searcher = SemanticSearcher::new(Arc::new(Mutex::new(storage)));

    let result = searcher.search("", None, None, 5).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_search_with_filters() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let searcher = SemanticSearcher::new(Arc::new(Mutex::new(storage)));

    let results = searcher
        .search("test query", Some("wing_code"), Some("auth"), 5)
        .await;
    assert!(results.is_ok());
    assert!(results.expect("search should succeed").is_empty());
}
