use super::*;
use crate::graph::palace_graph::{Direction, PalaceGraph};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct TraverseArgs {
    #[schemars(description = "Room to start from")]
    pub start_room: String,
    #[schemars(description = "How many connections to follow (default: 2)")]
    pub max_hops: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FindTunnelsArgs {
    #[schemars(description = "First wing (optional)")]
    pub wing_a: Option<String>,
    #[schemars(description = "Second wing (optional)")]
    pub wing_b: Option<String>,
}

pub(crate) async fn traverse(
    server: &McpServer,
    args: &TraverseArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let max_hops = args.max_hops.unwrap_or(2);
    let start_room = args.start_room.clone();
    let storage = server.storage.clone();
    let config = server.config.clone();

    let palace_graph = PalaceGraph::new(storage, config);
    let rooms = palace_graph
        .navigate(&start_room, Direction::Forward)
        .await
        .map_err(|e| McpError::invalid_request(e.to_string(), None))?;

    let visited = std::collections::HashSet::from([start_room.clone()]);
    let connections: Vec<_> = rooms
        .into_iter()
        .take(max_hops * 3)
        .map(|r| {
            serde_json::json!({
                "room": r.name,
                "wings": r.wing,
                "halls": r.keywords
            })
        })
        .collect();

    let result = serde_json::json!({
        "start_room": start_room,
        "max_hops": max_hops,
        "connections": connections,
        "visited": visited.into_iter().collect::<Vec<_>>()
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn find_tunnels(
    server: &McpServer,
    args: &FindTunnelsArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let wing_a = args.wing_a.clone();
    let wing_b = args.wing_b.clone();
    let storage = server.storage.clone();
    let config = server.config.clone();

    let palace_graph = PalaceGraph::new(storage, config);
    let tunnels = palace_graph
        .find_all_tunnels(wing_a.as_deref(), wing_b.as_deref())
        .await
        .map_err(|e| McpError::invalid_request(e.to_string(), None))?;

    let tunnel_list: Vec<_> = tunnels
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "room": t.room,
                "wings": t.wings,
                "halls": t.halls,
                "count": t.count
            })
        })
        .collect();

    let result = serde_json::json!({
        "wing_a": wing_a.unwrap_or_else(|| "all".to_string()),
        "wing_b": wing_b.unwrap_or_else(|| "all".to_string()),
        "tunnels": tunnel_list
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn graph_stats(
    server: &McpServer,
) -> std::result::Result<CallToolResult, McpError> {
    let storage = server.storage.clone();
    let config = server.config.clone();

    let palace_graph = PalaceGraph::new(storage, config);
    let stats = palace_graph
        .graph_stats()
        .await
        .map_err(|e| McpError::invalid_request(e.to_string(), None))?;

    let result = serde_json::json!({
        "total_rooms": stats.total_rooms,
        "total_connections": stats.total_edges,
        "tunnel_rooms": stats.tunnel_rooms,
        "rooms_per_wing": stats.rooms_per_wing
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}
