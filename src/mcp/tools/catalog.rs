use super::protocol::{AAAK_SPEC, PALACE_PROTOCOL};
use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ListRoomsArgs {
    #[schemars(description = "Wing to list rooms for (optional)")]
    pub wing: Option<String>,
}

pub(crate) async fn status(server: &McpServer) -> std::result::Result<CallToolResult, McpError> {
    let (drawer_count, all_drawers) = {
        let storage = server.storage.lock().await;
        (storage.count().unwrap_or(0), storage.get_all_drawers())
    };

    let mut wings: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut rooms: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for drawer in &all_drawers {
        *wings.entry(drawer.metadata.wing.clone()).or_insert(0) += 1;
        *rooms.entry(drawer.metadata.room.clone()).or_insert(0) += 1;
    }

    let (entity_count, triple_count) = if let Some(ref kg) = server.knowledge_graph {
        let kg = kg.lock().await;
        (
            kg.get_entity_count().unwrap_or(0),
            kg.get_triple_count().unwrap_or(0),
        )
    } else {
        (0, 0)
    };

    let result = serde_json::json!({
        "total_drawers": drawer_count,
        "total_entities": entity_count,
        "total_triples": triple_count,
        "wings": wings,
        "rooms": rooms,
        "palace_path": server.config.palace_path.to_string_lossy(),
        "kg_path": server.config.knowledge_graph_path.to_string_lossy(),
        "protocol": PALACE_PROTOCOL,
        "aaak_dialect": AAAK_SPEC
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn list_wings(
    server: &McpServer,
) -> std::result::Result<CallToolResult, McpError> {
    let all_drawers = {
        let storage = server.storage.lock().await;
        storage.get_all_drawers()
    };
    let mut wing_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for drawer in &all_drawers {
        *wing_counts.entry(drawer.metadata.wing.clone()).or_insert(0) += 1;
    }

    let wings: Vec<_> = wing_counts
        .into_iter()
        .map(|(name, count)| serde_json::json!({"name": name, "drawers": count}))
        .collect();

    let result = serde_json::json!({"wings": wings});
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn list_rooms(
    server: &McpServer,
    args: &ListRoomsArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let wing_filter = args.wing.as_deref();
    let all_drawers = {
        let storage = server.storage.lock().await;
        storage.get_all_drawers()
    };
    let mut room_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for drawer in &all_drawers {
        if let Some(w) = wing_filter {
            if drawer.metadata.wing != w {
                continue;
            }
        }
        *room_counts.entry(drawer.metadata.room.clone()).or_insert(0) += 1;
    }

    let rooms: Vec<_> = room_counts
        .into_iter()
        .map(|(name, count)| serde_json::json!({"name": name, "drawers": count}))
        .collect();

    let result = serde_json::json!({"wing": wing_filter.unwrap_or("all"), "rooms": rooms});
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn get_taxonomy(
    server: &McpServer,
) -> std::result::Result<CallToolResult, McpError> {
    let all_drawers = {
        let storage = server.storage.lock().await;
        storage.get_all_drawers()
    };
    let mut taxonomy: std::collections::HashMap<String, std::collections::HashMap<String, usize>> =
        std::collections::HashMap::new();

    for drawer in &all_drawers {
        let wing = &drawer.metadata.wing;
        let room = &drawer.metadata.room;
        let rooms = taxonomy.entry(wing.clone()).or_default();
        *rooms.entry(room.clone()).or_insert(0) += 1;
    }

    let result = serde_json::json!({"taxonomy": taxonomy});
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) fn get_aaak_spec() -> std::result::Result<CallToolResult, McpError> {
    let result = serde_json::json!({"aaak_spec": AAAK_SPEC});
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}
