use super::*;
use crate::palace::{Drawer, DrawerMetadata, IngestMode};
use tempfile::tempdir;

fn make_drawer(id: &str, doc: &str, wing: &str, room: &str) -> Drawer {
    Drawer::new(
        id,
        doc,
        DrawerMetadata::new(wing, room, "test.rs", 0, "test", IngestMode::Projects),
    )
}

#[test]
fn test_add_and_search() {
    let dir = tempdir().unwrap();
    let mut storage = ChromaStorage::new(dir.path(), "test").unwrap();

    storage.add_drawer(&make_drawer("d1", "Rust async programming tutorial", "wing_code", "rust")).unwrap();
    storage.add_drawer(&make_drawer("d2", "Python machine learning guide", "wing_code", "ml")).unwrap();
    storage.add_drawer(&make_drawer("d3", "The Rust borrow checker is strict", "wing_code", "rust")).unwrap();

    let results = storage.search("rust", None, None, 10);
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.text.to_lowercase().contains("rust")));

    let results = storage.search("machine learning", None, None, 10);
    assert!(!results.is_empty());
}

#[test]
fn test_search_with_wing_filter() {
    let dir = tempdir().unwrap();
    let mut storage = ChromaStorage::new(dir.path(), "test").unwrap();

    storage.add_drawer(&make_drawer("d1", "Rust tutorial", "wing_code", "rust")).unwrap();
    storage.add_drawer(&make_drawer("d2", "Personal journal entry", "wing_kai", "thoughts")).unwrap();

    let results = storage.search("tutorial", Some("wing_code"), None, 10);
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.wing == "wing_code"));
}

#[test]
fn test_search_with_room_filter() {
    let dir = tempdir().unwrap();
    let mut storage = ChromaStorage::new(dir.path(), "test").unwrap();

    storage.add_drawer(&make_drawer("d1", "Rust tutorial", "wing_code", "rust")).unwrap();
    storage.add_drawer(&make_drawer("d2", "Rust book notes", "wing_code", "reading")).unwrap();

    let results = storage.search("rust", None, Some("rust"), 10);
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.room == "rust"));
}

#[test]
fn test_delete_drawer() {
    let dir = tempdir().unwrap();
    let mut storage = ChromaStorage::new(dir.path(), "test").unwrap();

    storage.add_drawer(&make_drawer("d1", "To be deleted", "wing_code", "rust")).unwrap();
    assert_eq!(storage.count().unwrap(), 1);

    storage.delete_drawer("d1").unwrap();
    assert_eq!(storage.count().unwrap(), 0);
}

#[test]
fn test_get_drawers_by_filter() {
    let dir = tempdir().unwrap();
    let mut storage = ChromaStorage::new(dir.path(), "test").unwrap();

    storage.add_drawer(&make_drawer("d1", "Rust tutorial", "wing_code", "rust")).unwrap();
    storage.add_drawer(&make_drawer("d2", "ML tutorial", "wing_code", "ml")).unwrap();
    storage.add_drawer(&make_drawer("d3", "Personal notes", "wing_kai", "thoughts")).unwrap();

    let results = storage.get_drawers_by_filter(Some("wing_code"), None, 10);
    assert_eq!(results.len(), 2);
}

#[test]
fn test_count() {
    let dir = tempdir().unwrap();
    let mut storage = ChromaStorage::new(dir.path(), "test").unwrap();

    assert_eq!(storage.count().unwrap(), 0);
    storage.add_drawer(&make_drawer("d1", "doc1", "w", "r")).unwrap();
    storage.add_drawer(&make_drawer("d2", "doc2", "w", "r")).unwrap();
    assert_eq!(storage.count().unwrap(), 2);
}
