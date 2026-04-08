use super::*;
use crate::search::SemanticSearcher;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SearchArgs {
    #[schemars(description = "What to search for")]
    pub query: String,
    #[schemars(description = "Max results (default 5)")]
    pub limit: Option<usize>,
    #[schemars(description = "Filter by wing (optional)")]
    pub wing: Option<String>,
    #[schemars(description = "Filter by room (optional)")]
    pub room: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CheckDuplicateArgs {
    #[schemars(description = "Content to check")]
    pub content: String,
    #[schemars(description = "Similarity threshold 0-1 (default 0.9)")]
    pub threshold: Option<f64>,
}

pub(crate) async fn search(
    server: &McpServer,
    args: &SearchArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let limit = args.limit.unwrap_or(5);
    let storage = server.storage.clone();
    let query = args.query.clone();
    let wing = args.wing.clone();
    let room = args.room.clone();

    let searcher = SemanticSearcher::new(storage);
    let results = searcher
        .search(&query, wing.as_deref(), room.as_deref(), limit)
        .await
        .map_err(|e| McpError::invalid_request(e.to_string(), None))?;

    let hits: Vec<_> = results
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "text": r.hit.text,
                "wing": r.hit.wing,
                "room": r.hit.room,
                "source_file": r.hit.source_file,
                "similarity": r.hit.similarity,
                "distance": r.hit.distance
            })
        })
        .collect();

    let result = serde_json::json!({
        "results": hits,
        "count": hits.len(),
        "query": query,
        "limit": limit,
        "wing": wing,
        "room": room
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn check_duplicate(
    server: &McpServer,
    args: &CheckDuplicateArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let threshold = args.threshold.unwrap_or(0.9);
    let storage = server.storage.clone();
    let content = args.content.clone();

    let searcher = SemanticSearcher::new(storage);
    let results = searcher
        .search(&content, None, None, 5)
        .await
        .map_err(|e| McpError::invalid_request(e.to_string(), None))?;

    let matches: Vec<_> = results
        .into_iter()
        .filter(|r| r.hit.similarity >= threshold)
        .map(|r| {
            serde_json::json!({
                "text": r.hit.text,
                "wing": r.hit.wing,
                "room": r.hit.room,
                "similarity": r.hit.similarity
            })
        })
        .collect();

    let result = serde_json::json!({
        "is_duplicate": !matches.is_empty(),
        "matches": matches,
        "content_preview": if content.chars().count() > 100 {
            format!("{}...", content.chars().take(100).collect::<String>())
        } else {
            content.clone()
        },
        "threshold": threshold
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}
