use super::*;
use std::cell::RefCell;
use std::rc::Rc;
use tempfile::tempdir;

#[test]
fn test_palace_graph_creation() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let config = Config::default();
    let graph = PalaceGraph::new(Rc::new(RefCell::new(storage)), config);

    let (nodes, edges) = graph.build_graph().unwrap();
    assert!(nodes.is_empty());
    assert!(edges.is_empty());
}

#[test]
fn test_find_tunnel_empty_graph() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let config = Config::default();
    let graph = PalaceGraph::new(Rc::new(RefCell::new(storage)), config);

    let tunnel = graph.find_tunnel("wing_a", "wing_b").unwrap();
    assert!(tunnel.is_none());
}

#[test]
fn test_get_hall_empty_graph() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let config = Config::default();
    let graph = PalaceGraph::new(Rc::new(RefCell::new(storage)), config);

    let result = graph.get_hall("technical");
    assert!(result.is_err());
}

#[test]
fn test_graph_stats() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let config = Config::default();
    let graph = PalaceGraph::new(Rc::new(RefCell::new(storage)), config);

    let stats = graph.graph_stats().unwrap();
    assert_eq!(stats.total_rooms, 0);
    assert_eq!(stats.tunnel_rooms, 0);
}
