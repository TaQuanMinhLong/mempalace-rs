use super::*;
use crate::storage::ChromaStorage;
use tempfile::tempdir;

#[test]
fn test_retriever_creation() {
    let retriever = Retriever::new();
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test").unwrap();
    let _ = retriever.layer_summary(&MemoryStack::new(None, None), &storage);
}

#[test]
fn test_retrieve_layer_l0() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test").unwrap();
    let stack = MemoryStack::new(None, None);
    let retriever = Retriever::new();

    let context = retriever.retrieve_layer(&stack, &storage, MemoryLayer::L0);
    assert!(context.is_ok());
}

#[test]
fn test_retrieve_layer_l3_returns_empty() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test").unwrap();
    let stack = MemoryStack::new(None, None);
    let retriever = Retriever::new();

    let context = retriever
        .retrieve_layer(&stack, &storage, MemoryLayer::L3)
        .unwrap();
    assert!(context.is_empty());
}

#[test]
fn test_retrieval_plan_uses_query_when_present() {
    let retriever = Retriever::new();
    let options = RetrieveOptions::new(Some("graph migration")).with_limit(3);

    let plan = retriever.plan(&options);

    assert_eq!(plan.mode, RetrievalMode::LayeredSearch);
    assert_eq!(plan.query.as_deref(), Some("graph migration"));
    assert_eq!(plan.limit, 3);
}

#[test]
fn test_retrieval_without_query_falls_back_to_wakeup() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test").unwrap();
    let stack = MemoryStack::new(None, None);
    let retriever = Retriever::new();
    let options = RetrieveOptions::new(None).with_wing(Some("wing_code"));

    let result = retriever
        .retrieve_with_options(&stack, &storage, &options)
        .unwrap();

    assert_eq!(result.mode, RetrievalMode::WakeUpOnly);
    assert!(result.hits.is_empty());
    assert!(result.context.contains("## L0"));
}
