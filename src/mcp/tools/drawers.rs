use super::*;
use crate::palace::{Drawer, DrawerMetadata, IngestMode};
use chrono::Utc;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AddDrawerArgs {
    #[schemars(description = "Wing (project name)")]
    pub wing: String,
    #[schemars(description = "Room (aspect: backend, decisions, meetings...)")]
    pub room: String,
    #[schemars(description = "Verbatim content to store")]
    pub content: String,
    #[schemars(description = "Where this came from (optional)")]
    pub source_file: Option<String>,
    #[schemars(description = "Who is filing this (default: mcp)")]
    pub added_by: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DeleteDrawerArgs {
    #[schemars(description = "ID of the drawer to delete")]
    pub drawer_id: String,
}

pub(crate) async fn add_drawer(
    server: &McpServer,
    args: &AddDrawerArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let source_file = args
        .source_file
        .clone()
        .unwrap_or_else(|| "mcp".to_string());
    let added_by = args.added_by.clone().unwrap_or_else(|| "mcp".to_string());
    let wing = args.wing.clone();
    let room = args.room.clone();
    let content = args.content.clone();

    let mut hasher = Sha256::new();
    hasher.update(
        format!(
            "{}_{}_{}_{}",
            wing,
            room,
            &content[..content.len().min(100)],
            Utc::now().to_rfc3339()
        )
        .as_bytes(),
    );
    let hash_result = hasher.finalize();
    let drawer_id = format!("drawer_{}_{}", wing, hex::encode(&hash_result[..8]));

    let metadata = DrawerMetadata::new(
        &wing,
        &room,
        &source_file,
        0,
        &added_by,
        IngestMode::Projects,
    );

    let drawer = Drawer::new(drawer_id.clone(), &content, metadata);

    let storage = server.storage.clone();
    storage
        .lock()
        .await
        .add_drawer(&drawer)
        .map_err(|e| McpError::invalid_request(e.to_string(), None))?;

    tracing::info!("Filed drawer: {} -> {}/{}", drawer_id, wing, room);

    let result = serde_json::json!({
        "success": true,
        "drawer_id": drawer_id,
        "wing": wing,
        "room": room,
        "source_file": source_file,
        "added_by": added_by
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn delete_drawer(
    server: &McpServer,
    args: &DeleteDrawerArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let drawer_id = args.drawer_id.clone();
    let storage = server.storage.clone();
    storage
        .lock()
        .await
        .delete_drawer(&drawer_id)
        .map_err(|e| McpError::invalid_request(e.to_string(), None))?;

    tracing::info!("Deleted drawer: {}", drawer_id);

    let result = serde_json::json!({
        "success": true,
        "drawer_id": drawer_id
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}
