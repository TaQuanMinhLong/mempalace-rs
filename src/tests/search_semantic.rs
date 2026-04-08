use super::*;
use tempfile::tempdir;

#[test]
fn test_search_result_creation() {
    let hit = SearchHit {
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

#[test]
fn test_search_empty_query() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let searcher = SemanticSearcher::new(Rc::new(RefCell::new(storage)));

    let result = searcher.search("", None, None, 5);
    assert!(result.is_err());
}

#[test]
fn test_search_with_filters() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let searcher = SemanticSearcher::new(Rc::new(RefCell::new(storage)));

    let results = searcher.search("test query", Some("wing_code"), Some("auth"), 5);
    assert!(results.is_ok());
    assert!(results.unwrap().is_empty());
}
