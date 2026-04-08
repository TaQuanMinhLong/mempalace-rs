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
