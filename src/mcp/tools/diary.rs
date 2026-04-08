use super::*;
use crate::palace::{Drawer, DrawerMetadata, IngestMode};
use chrono::Utc;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DiaryWriteArgs {
    #[schemars(description = "Your name")]
    pub agent_name: String,
    #[schemars(description = "Your diary entry in AAAK format")]
    pub entry: String,
    #[schemars(description = "Topic tag (optional, default: general)")]
    pub topic: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DiaryReadArgs {
    #[schemars(description = "Your name")]
    pub agent_name: String,
    #[schemars(description = "Number of recent entries (default: 10)")]
    pub last_n: Option<usize>,
}

pub(crate) async fn diary_write(
    server: &McpServer,
    args: &DiaryWriteArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let agent_name = args.agent_name.clone();
    let entry = args.entry.clone();
    let topic = args.topic.clone().unwrap_or_else(|| "general".to_string());

    let wing = format!("wing_{}", agent_name.to_lowercase().replace(' ', "_"));
    let room = format!("diary/{}", topic);
    let now = Utc::now();
    let entry_id = format!("diary_{}_{}", wing, now.format("%Y%m%d_%H%M%S"));

    let mut hasher = Sha256::new();
    hasher.update(format!("{}_{}_{}_{}", wing, room, entry, now.to_rfc3339()).as_bytes());
    let hash_result = hasher.finalize();
    let drawer_id = format!("{}_{}", entry_id, hex::encode(&hash_result[..8]));

    let metadata = DrawerMetadata::new(&wing, &room, "diary", 0, &agent_name, IngestMode::Convos);
    let drawer = Drawer::new(drawer_id.clone(), &entry, metadata);

    let storage = server.storage.clone();
    storage
        .lock()
        .await
        .add_drawer(&drawer)
        .map_err(|e| McpError::invalid_request(e.to_string(), None))?;

    tracing::info!("Wrote diary entry: {} -> {}/{}", entry_id, wing, topic);

    let result = serde_json::json!({
        "success": true,
        "entry_id": entry_id,
        "agent": agent_name,
        "topic": topic,
        "timestamp": now.to_rfc3339(),
        "wing": wing,
        "room": room
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn diary_read(
    server: &McpServer,
    args: &DiaryReadArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let agent_name = args.agent_name.clone();
    let last_n = args.last_n.unwrap_or(10);
    let wing = format!("wing_{}", agent_name.to_lowercase().replace(' ', "_"));

    let all_drawers = {
        let storage = server.storage.lock().await;
        storage.get_all_drawers()
    };
    let entries: Vec<_> = all_drawers
        .into_iter()
        .filter(|d| d.metadata.wing == wing && d.metadata.room.starts_with("diary/"))
        .rev()
        .take(last_n)
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "content": d.document,
                "room": d.metadata.room,
                "source_file": d.metadata.source_file,
                "added_by": d.metadata.added_by
            })
        })
        .collect();
    let total = entries.len();

    let result = serde_json::json!({
        "agent": agent_name,
        "wing": wing,
        "entries": entries,
        "total": total,
        "showing": total.min(last_n)
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}
