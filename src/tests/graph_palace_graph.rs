use super::*;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_palace_graph_creation() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let config = Config::default();
    let graph = PalaceGraph::new(Arc::new(Mutex::new(storage)), config);

    let (nodes, edges) = graph.build_graph().await.unwrap();
    assert!(nodes.is_empty());
    assert!(edges.is_empty());
}

#[tokio::test]
async fn test_find_tunnel_empty_graph() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let config = Config::default();
    let graph = PalaceGraph::new(Arc::new(Mutex::new(storage)), config);

    let tunnel = graph.find_tunnel("wing_a", "wing_b").await.unwrap();
    assert!(tunnel.is_none());
}

#[tokio::test]
async fn test_get_hall_empty_graph() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let config = Config::default();
    let graph = PalaceGraph::new(Arc::new(Mutex::new(storage)), config);

    let result = graph.get_hall("technical").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_graph_stats() {
    let dir = tempdir().unwrap();
    let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
    let config = Config::default();
    let graph = PalaceGraph::new(Arc::new(Mutex::new(storage)), config);

    let stats = graph.graph_stats().await.unwrap();
    assert_eq!(stats.total_rooms, 0);
    assert_eq!(stats.tunnel_rooms, 0);
}
