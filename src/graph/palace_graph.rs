//! Palace graph - room navigation and tunnel discovery
//!
//! Port from Python palace_graph.py. Builds a navigable graph from the palace
//! structure:
//!   - Nodes = rooms (named ideas)
//!   - Edges = shared rooms across wings (tunnels)
//!   - Edge types = halls (the corridors)
//!
//! Enables queries like:
//!   "Start at chromadb-setup in wing_code, walk to wing_myproject"
//!   "Find all rooms connected to riley-college-apps"
//!   "What topics bridge wing_hardware and wing_myproject?"

use crate::config::Config;
use crate::error::{MempalaceError, Result};
use crate::palace::Room;
use crate::storage::ChromaStorage;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Direction for navigation
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Forward,
    Backward,
    Tunnel,
}

/// Tunnel between wings (a room that spans multiple wings)
#[derive(Debug, Clone)]
pub struct Tunnel {
    pub room: String,
    pub wings: Vec<String>,
    pub halls: Vec<String>,
    pub count: usize,
}

/// Hall content (rooms in a hall category)
#[derive(Debug, Clone)]
pub struct HallContent {
    pub hall_type: String,
    pub rooms: Vec<String>,
}

/// Room node in the palace graph
#[derive(Debug, Clone)]
pub struct RoomNode {
    pub name: String,
    pub wings: Vec<String>,
    pub halls: Vec<String>,
    pub count: usize,
    pub dates: Vec<String>,
}

/// Palace graph for navigation
#[derive(Debug)]
pub struct PalaceGraph {
    storage: Rc<RefCell<ChromaStorage>>,
    // config: Config, // not stored; Python passes as parameter
}

impl PalaceGraph {
    /// Create a new palace graph
    pub fn new(storage: Rc<RefCell<ChromaStorage>>, _config: Config) -> Self {
        Self { storage }
    }

    /// Build the graph from ChromaDB metadata.
    ///
    /// Returns (nodes, edges) where:
    /// - nodes: map of room name -> RoomNode
    /// - edges: list of tunnel connections
    pub fn build_graph(&self) -> Result<(HashMap<String, RoomNode>, Vec<TunnelEdge>)> {
        let mut room_data: HashMap<String, RoomData> = HashMap::new();

        // Iterate through all drawers in storage
        let drawers = self.storage.borrow().get_all_drawers();

        for drawer in drawers {
            let room_name = &drawer.metadata.room;
            let wing = &drawer.metadata.wing;

            // Skip empty or "general" rooms
            if room_name.is_empty() || room_name == "general" || wing.is_empty() {
                continue;
            }

            let entry = room_data.entry(room_name.clone()).or_default();
            entry.wings.insert(wing.clone());
            entry.count += 1;
        }

        // Build edges from rooms that span multiple wings
        let mut edges: Vec<TunnelEdge> = Vec::new();
        for (room, data) in &room_data {
            let wings: Vec<_> = data.wings.iter().collect();
            if wings.len() >= 2 {
                // Create an edge for each pair of wings
                for (i, wa) in wings.iter().enumerate() {
                    for wb in wings.iter().skip(i + 1) {
                        edges.push(TunnelEdge {
                            room: room.clone(),
                            wing_a: (*wa).clone(),
                            wing_b: (*wb).clone(),
                            hall: String::new(),
                            count: data.count,
                        });
                    }
                }
            }
        }

        // Convert to nodes
        let mut nodes: HashMap<String, RoomNode> = HashMap::new();
        for (room, data) in room_data {
            let mut wings: Vec<_> = data.wings.into_iter().collect();
            wings.sort();
            nodes.insert(
                room.clone(),
                RoomNode {
                    name: room,
                    wings,
                    halls: Vec::new(),
                    count: data.count,
                    dates: Vec::new(),
                },
            );
        }

        Ok((nodes, edges))
    }

    /// Navigate from a room in a direction.
    ///
    /// Forward: move to rooms with shared wings
    /// Backward: move to rooms with overlapping concepts
    /// Tunnel: find tunnel rooms between wings
    pub fn navigate(&self, from_room: &str, direction: Direction) -> Result<Vec<Room>> {
        let (nodes, _edges) = self.build_graph()?;

        if nodes.is_empty() {
            return Err(MempalaceError::NotFound(
                "No palace graph available. Run mempalace mine first.".to_string(),
            ));
        }

        let from_node = nodes
            .get(from_room)
            .ok_or_else(|| MempalaceError::NotFound(format!("Room '{}' not found", from_room)))?;

        match direction {
            Direction::Forward => {
                // Find rooms sharing a wing with the starting room
                let mut results = Vec::new();
                for (room_name, node) in &nodes {
                    if room_name == from_room {
                        continue;
                    }
                    let shared: Vec<_> = from_node
                        .wings
                        .iter()
                        .filter(|w| node.wings.contains(w))
                        .collect();
                    if !shared.is_empty() {
                        results.push(Room::new(
                            room_name.clone(),
                            node.wings.join("/"),
                            node.halls.clone(),
                        ));
                    }
                }
                Ok(results)
            }
            Direction::Backward => {
                // Find rooms that share halls (conceptual connection)
                let mut results = Vec::new();
                for (room_name, node) in &nodes {
                    if room_name == from_room {
                        continue;
                    }
                    let shared: Vec<_> = from_node
                        .halls
                        .iter()
                        .filter(|h| node.halls.contains(h))
                        .collect();
                    if !shared.is_empty() {
                        results.push(Room::new(
                            room_name.clone(),
                            node.wings.join("/"),
                            node.halls.clone(),
                        ));
                    }
                }
                Ok(results)
            }
            Direction::Tunnel => {
                // Tunnels are rooms that span multiple wings
                // This is handled by find_tunnel
                Ok(Vec::new())
            }
        }
    }

    /// Find tunnel between two wings.
    ///
    /// A tunnel is a room that exists in both wings, acting as a
    /// hallway connecting them.
    pub fn find_tunnel(&self, wing_a: &str, wing_b: &str) -> Result<Option<Tunnel>> {
        let (nodes, _edges) = self.build_graph()?;

        if nodes.is_empty() {
            return Ok(None);
        }

        // Find rooms that span both wings
        let mut candidates: Vec<_> = nodes
            .values()
            .filter(|n| {
                n.wings.contains(&wing_a.to_string()) && n.wings.contains(&wing_b.to_string())
            })
            .collect();

        if candidates.is_empty() {
            return Ok(None);
        }

        // Return the most connected tunnel (most content)
        candidates.sort_by_key(|b| std::cmp::Reverse(b.count));

        let tunnel = candidates.first().unwrap();
        Ok(Some(Tunnel {
            room: tunnel.name.clone(),
            wings: tunnel.wings.clone(),
            halls: tunnel.halls.clone(),
            count: tunnel.count,
        }))
    }

    /// Find all tunnels (rooms spanning multiple wings).
    ///
    /// If wing_a or wing_b is specified, only return tunnels connecting those wings.
    pub fn find_all_tunnels(
        &self,
        wing_a: Option<&str>,
        wing_b: Option<&str>,
    ) -> Result<Vec<Tunnel>> {
        let (nodes, _edges) = self.build_graph()?;

        if nodes.is_empty() {
            return Ok(Vec::new());
        }

        let mut tunnels = Vec::new();

        for node in nodes.values() {
            if node.wings.len() < 2 {
                continue;
            }

            // Filter by wing if specified
            if let Some(wa) = wing_a {
                if !node.wings.contains(&wa.to_string()) {
                    continue;
                }
            }
            if let Some(wb) = wing_b {
                if !node.wings.contains(&wb.to_string()) {
                    continue;
                }
            }

            tunnels.push(Tunnel {
                room: node.name.clone(),
                wings: node.wings.clone(),
                halls: node.halls.clone(),
                count: node.count,
            });
        }

        // Sort by content count descending
        tunnels.sort_by_key(|b| std::cmp::Reverse(b.count));
        tunnels.truncate(50);

        Ok(tunnels)
    }

    /// Get hall content - rooms belonging to a hall category.
    pub fn get_hall(&self, hall_type: &str) -> Result<Vec<String>> {
        let (nodes, _edges) = self.build_graph()?;

        if nodes.is_empty() {
            return Err(MempalaceError::NotFound(
                "No palace graph available. Run mempalace mine first.".to_string(),
            ));
        }

        let hall_lower = hall_type.to_lowercase();
        let mut rooms: Vec<_> = nodes
            .values()
            .filter(|n| n.halls.iter().any(|h| h.to_lowercase() == hall_lower))
            .map(|n| n.name.clone())
            .collect();

        rooms.sort();
        Ok(rooms)
    }

    /// Get graph statistics
    pub fn graph_stats(&self) -> Result<GraphStats> {
        let (nodes, edges) = self.build_graph()?;

        let tunnel_rooms = nodes.values().filter(|n| n.wings.len() >= 2).count();

        let mut rooms_per_wing: HashMap<String, usize> = HashMap::new();
        for node in nodes.values() {
            for wing in &node.wings {
                *rooms_per_wing.entry(wing.clone()).or_insert(0) += 1;
            }
        }

        let top_tunnels: Vec<Tunnel> = nodes
            .values()
            .filter(|n| n.wings.len() >= 2)
            .map(|n| Tunnel {
                room: n.name.clone(),
                wings: n.wings.clone(),
                halls: n.halls.clone(),
                count: n.count,
            })
            .take(10)
            .collect();

        Ok(GraphStats {
            total_rooms: nodes.len(),
            tunnel_rooms,
            total_edges: edges.len(),
            rooms_per_wing,
            top_tunnels,
        })
    }
}

/// Edge between rooms via a shared wing
#[derive(Debug, Clone)]
pub struct TunnelEdge {
    pub room: String,
    pub wing_a: String,
    pub wing_b: String,
    pub hall: String,
    pub count: usize,
}

/// Internal room data during graph building
#[derive(Debug, Clone, Default)]
struct RoomData {
    wings: HashSet<String>,
    // halls: HashSet<String>, // not populated in Rust port
    count: usize,
    // dates: HashSet<String>, // not populated in Rust port
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub total_rooms: usize,
    pub tunnel_rooms: usize,
    pub total_edges: usize,
    pub rooms_per_wing: HashMap<String, usize>,
    pub top_tunnels: Vec<Tunnel>,
}

#[cfg(test)]
mod tests {
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

        // Empty graph since ChromaDB is stub
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

        // Returns None for empty graph
        let tunnel = graph.find_tunnel("wing_a", "wing_b").unwrap();
        assert!(tunnel.is_none());
    }

    #[test]
    fn test_get_hall_empty_graph() {
        let dir = tempdir().unwrap();
        let storage = ChromaStorage::new(dir.path(), "test_collection").unwrap();
        let config = Config::default();
        let graph = PalaceGraph::new(Rc::new(RefCell::new(storage)), config);

        // Returns error for empty graph
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
}
