use super::*;
use chrono::Utc;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KgQueryArgs {
    #[schemars(description = "Entity to query")]
    pub entity: String,
    #[schemars(description = "Date filter (YYYY-MM-DD, optional)")]
    pub as_of: Option<String>,
    #[schemars(description = "outgoing, incoming, or both (default: both)")]
    pub direction: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KgAddArgs {
    #[schemars(description = "The entity doing/being something")]
    pub subject: String,
    #[schemars(description = "The relationship type")]
    pub predicate: String,
    #[schemars(description = "The entity being connected to")]
    pub object: String,
    #[schemars(description = "When this became true (YYYY-MM-DD, optional)")]
    pub valid_from: Option<String>,
    #[schemars(description = "Closet ID (optional)")]
    pub source_closet: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KgInvalidateArgs {
    #[schemars(description = "Entity")]
    pub subject: String,
    #[schemars(description = "Relationship")]
    pub predicate: String,
    #[schemars(description = "Connected entity")]
    pub object: String,
    #[schemars(description = "When it stopped being true (YYYY-MM-DD, default: today)")]
    pub ended: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KgTimelineArgs {
    #[schemars(description = "Entity to get timeline for (optional)")]
    pub entity: Option<String>,
}

pub(crate) async fn kg_query(
    server: &McpServer,
    args: &KgQueryArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let kg = server
        .knowledge_graph
        .as_ref()
        .ok_or_else(|| McpError::invalid_request("Knowledge graph not available", None))?;

    let entity = args.entity.clone();
    let as_of = args.as_of.clone();
    let direction = args.direction.clone().unwrap_or_else(|| "both".to_string());

    let triples = {
        let kg = kg.lock().await;
        kg.get_triples_for_entity(&entity)
            .map_err(|e| McpError::invalid_request(e.to_string(), None))?
    };

    let facts: Vec<_> = triples
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "subject": t.subject,
                "predicate": t.predicate,
                "object": t.object,
                "valid_from": t.valid_from.map(|d| d.to_string()),
                "valid_to": t.valid_to.map(|d| d.to_string()),
                "confidence": t.confidence
            })
        })
        .collect();

    let result = serde_json::json!({
        "entity": entity,
        "as_of": as_of,
        "direction": direction,
        "facts": facts,
        "count": facts.len()
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn kg_add(
    server: &McpServer,
    args: &KgAddArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let kg = server
        .knowledge_graph
        .as_ref()
        .ok_or_else(|| McpError::invalid_request("Knowledge graph not available", None))?;

    let subject = args.subject.clone();
    let predicate = args.predicate.clone();
    let object = args.object.clone();
    let valid_from = args.valid_from.clone();
    let source_closet = args.source_closet.clone();

    let triple_id = format!("{}_{}_{}", subject, predicate, object);
    let triple = crate::storage::Triple {
        id: triple_id.clone(),
        subject: subject.clone(),
        predicate: predicate.clone(),
        object: object.clone(),
        valid_from: valid_from
            .as_ref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        valid_to: None,
        confidence: 1.0,
        source_closet: source_closet.unwrap_or_default(),
        source_file: "mcp".to_string(),
        extracted_at: Utc::now(),
    };

    {
        let kg = kg.lock().await;
        kg.upsert_triple(&triple)
            .map_err(|e| McpError::invalid_request(e.to_string(), None))?;
    }

    tracing::info!("Added triple: {} -> {} -> {}", subject, predicate, object);

    let result = serde_json::json!({
        "success": true,
        "triple_id": triple_id,
        "fact": format!("{} -> {} -> {}", subject, predicate, object),
        "valid_from": valid_from
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn kg_invalidate(
    server: &McpServer,
    args: &KgInvalidateArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let kg = server
        .knowledge_graph
        .as_ref()
        .ok_or_else(|| McpError::invalid_request("Knowledge graph not available", None))?;

    let subject = args.subject.clone();
    let predicate = args.predicate.clone();
    let object = args.object.clone();
    let ended_binding = Utc::now().format("%Y-%m-%d").to_string();
    let ended = args.ended.as_ref().unwrap_or(&ended_binding);

    {
        let kg = kg.lock().await;
        match kg.get_triples_for_entity(&subject) {
            Ok(triples) => {
                for mut triple in triples {
                    if triple.subject == subject
                        && triple.predicate == predicate
                        && triple.object == object
                    {
                        triple.valid_to = chrono::NaiveDate::parse_from_str(ended, "%Y-%m-%d").ok();
                        if kg.upsert_triple(&triple).is_ok() {
                            let result = serde_json::json!({
                                "success": true,
                                "fact": format!("{} -> {} -> {}", subject, predicate, object),
                                "ended": ended
                            });
                            return Ok(CallToolResult::success(vec![Content::text(
                                serde_json::to_string_pretty(&result).unwrap_or_default(),
                            )]));
                        }
                    }
                }
                Err(McpError::invalid_request("Triple not found", None))
            }
            Err(e) => Err(McpError::invalid_request(e.to_string(), None)),
        }
    }
}

pub(crate) async fn kg_timeline(
    server: &McpServer,
    args: &KgTimelineArgs,
) -> std::result::Result<CallToolResult, McpError> {
    let kg = server
        .knowledge_graph
        .as_ref()
        .ok_or_else(|| McpError::invalid_request("Knowledge graph not available", None))?;

    let entity_opt = args.entity.clone();

    let timeline: Vec<_> = if let Some(ref entity) = entity_opt {
        let kg = kg.lock().await;
        match kg.get_triples_for_entity(entity) {
            Ok(triples) => triples
                .into_iter()
                .map(|t| {
                    serde_json::json!({
                        "subject": t.subject,
                        "predicate": t.predicate,
                        "object": t.object,
                        "valid_from": t.valid_from.map(|d| d.to_string()),
                        "valid_to": t.valid_to.map(|d| d.to_string()),
                        "extracted_at": t.extracted_at.to_rfc3339()
                    })
                })
                .collect(),
            Err(_) => vec![],
        }
    } else {
        vec![]
    };

    let result = serde_json::json!({
        "entity": entity_opt.unwrap_or_else(|| "all".to_string()),
        "timeline": timeline,
        "count": timeline.len()
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub(crate) async fn kg_stats(server: &McpServer) -> std::result::Result<CallToolResult, McpError> {
    let kg = server
        .knowledge_graph
        .as_ref()
        .ok_or_else(|| McpError::invalid_request("Knowledge graph not available", None))?;

    let (entities, triples) = {
        let kg = kg.lock().await;
        (
            kg.get_entity_count().unwrap_or(0),
            kg.get_triple_count().unwrap_or(0),
        )
    };

    let result = serde_json::json!({
        "entities": entities,
        "triples": triples,
        "current_facts": triples,
        "expired_facts": 0,
        "relationship_types": []
    });

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}
