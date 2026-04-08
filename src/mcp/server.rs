//! MCP server - Model Context Protocol server
//!
//! Port from Python mcp_server.py (19 tools)
//! Uses JSON-RPC protocol over stdio

use crate::config::Config;
use crate::error::Result;
use crate::graph::palace_graph::PalaceGraph;
use crate::palace::{Drawer, DrawerMetadata, IngestMode};
use crate::search::SemanticSearcher;
use crate::storage::{ChromaStorage, KnowledgeGraph};
use chrono::Utc;
// use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::rc::Rc;
use tracing::{error, info};

// Note: Python uses plain dict error responses instead of typed errors.
/// MCP server for MemPalace
#[derive(Debug)]
pub struct McpServer {
    config: Config,
    storage: Rc<RefCell<ChromaStorage>>,
    knowledge_graph: Option<Rc<KnowledgeGraph>>,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;
        let knowledge_graph = if config.knowledge_graph_path.exists() {
            KnowledgeGraph::new(&config.knowledge_graph_path)
                .ok()
                .map(Rc::new)
        } else {
            None
        };
        Ok(Self {
            config,
            storage: Rc::new(RefCell::new(storage)),
            knowledge_graph,
        })
    }

    /// Start the MCP server - read JSON-RPC from stdin, write to stdout
    pub fn start(&self) -> Result<()> {
        info!("MemPalace MCP Server starting...");

        let mut buffer = String::new();
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();

        loop {
            // Read a line from stdin
            buffer.clear();
            match handle.read_line(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    error!("Read error: {}", e);
                    break;
                }
            }

            let line = buffer.trim();
            if line.is_empty() {
                continue;
            }

            // Parse JSON-RPC request
            let request: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    error!("JSON parse error: {}", e);
                    continue;
                }
            };

            // Handle the request
            let response = self.handle_request(&request);

            // Send response
            if let Some(resp) = response {
                let output = serde_json::to_string(&resp).unwrap_or_else(|_| {
                    serde_json::json!({"jsonrpc": "2.0", "error": {"code": -32000, "message": "Serialization error"}}).to_string()
                });
                println!("{}", output);
                std::io::stdout().flush().ok();
            }
        }

        Ok(())
    }

    /// Handle a JSON-RPC request
    fn handle_request(&self, request: &Value) -> Option<Value> {
        let method = request.get("method")?.as_str()?;
        let id = request.get("id").cloned();

        let response = match method {
            "initialize" => self.tool_initialize(&id),
            "notifications/initialized" => serde_json::Value::Null, // Acknowledge and don't respond
            "tools/list" => self.tool_list(&id),
            "tools/call" => self.tool_call(&id, request.get("params")),
            _ => self.error_response(&id, -32601, &format!("Unknown method: {}", method)),
        };

        Some(response)
    }

    /// Initialize response
    fn tool_initialize(&self, id: &Option<Value>) -> Value {
        self.response(
            id,
            &serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {
                    "name": "mempalace",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    /// List all tools
    fn tool_list(&self, id: &Option<Value>) -> Value {
        let tools = serde_json::json!([
            {
                "name": "mempalace_status",
                "description": "Palace overview — total drawers, wing and room counts",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "mempalace_list_wings",
                "description": "List all wings with drawer counts",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "mempalace_list_rooms",
                "description": "List rooms within a wing (or all rooms if no wing given)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "wing": {"type": "string", "description": "Wing to list rooms for (optional)"}
                    }
                }
            },
            {
                "name": "mempalace_get_taxonomy",
                "description": "Full taxonomy: wing → room → drawer count",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "mempalace_get_aaak_spec",
                "description": "Get the AAAK dialect specification",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "mempalace_search",
                "description": "Semantic search. Returns verbatim drawer content with similarity scores.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "What to search for"},
                        "limit": {"type": "integer", "description": "Max results (default 5)"},
                        "wing": {"type": "string", "description": "Filter by wing (optional)"},
                        "room": {"type": "string", "description": "Filter by room (optional)"}
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "mempalace_check_duplicate",
                "description": "Check if content already exists in the palace before filing",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": {"type": "string", "description": "Content to check"},
                        "threshold": {"type": "number", "description": "Similarity threshold 0-1 (default 0.9)"}
                    },
                    "required": ["content"]
                }
            },
            {
                "name": "mempalace_add_drawer",
                "description": "File verbatim content into the palace. Checks for duplicates first.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "wing": {"type": "string", "description": "Wing (project name)"},
                        "room": {"type": "string", "description": "Room (aspect: backend, decisions, meetings...)"},
                        "content": {"type": "string", "description": "Verbatim content to store"},
                        "source_file": {"type": "string", "description": "Where this came from (optional)"},
                        "added_by": {"type": "string", "description": "Who is filing this (default: mcp)"}
                    },
                    "required": ["wing", "room", "content"]
                }
            },
            {
                "name": "mempalace_delete_drawer",
                "description": "Delete a drawer by ID. Irreversible.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "drawer_id": {"type": "string", "description": "ID of the drawer to delete"}
                    },
                    "required": ["drawer_id"]
                }
            },
            {
                "name": "mempalace_kg_query",
                "description": "Query the knowledge graph for an entity's relationships",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity": {"type": "string", "description": "Entity to query"},
                        "as_of": {"type": "string", "description": "Date filter (YYYY-MM-DD, optional)"},
                        "direction": {"type": "string", "description": "outgoing, incoming, or both (default: both)"}
                    },
                    "required": ["entity"]
                }
            },
            {
                "name": "mempalace_kg_add",
                "description": "Add a fact to the knowledge graph",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "subject": {"type": "string", "description": "The entity doing/being something"},
                        "predicate": {"type": "string", "description": "The relationship type"},
                        "object": {"type": "string", "description": "The entity being connected to"},
                        "valid_from": {"type": "string", "description": "When this became true (YYYY-MM-DD, optional)"},
                        "source_closet": {"type": "string", "description": "Closet ID (optional)"}
                    },
                    "required": ["subject", "predicate", "object"]
                }
            },
            {
                "name": "mempalace_kg_invalidate",
                "description": "Mark a fact as no longer true",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "subject": {"type": "string", "description": "Entity"},
                        "predicate": {"type": "string", "description": "Relationship"},
                        "object": {"type": "string", "description": "Connected entity"},
                        "ended": {"type": "string", "description": "When it stopped being true (YYYY-MM-DD, default: today)"}
                    },
                    "required": ["subject", "predicate", "object"]
                }
            },
            {
                "name": "mempalace_kg_timeline",
                "description": "Chronological timeline of facts",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity": {"type": "string", "description": "Entity to get timeline for (optional)"}
                    }
                }
            },
            {
                "name": "mempalace_kg_stats",
                "description": "Knowledge graph overview",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "mempalace_traverse",
                "description": "Walk the palace graph from a room",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "start_room": {"type": "string", "description": "Room to start from"},
                        "max_hops": {"type": "integer", "description": "How many connections to follow (default: 2)"}
                    },
                    "required": ["start_room"]
                }
            },
            {
                "name": "mempalace_find_tunnels",
                "description": "Find rooms that bridge two wings",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "wing_a": {"type": "string", "description": "First wing (optional)"},
                        "wing_b": {"type": "string", "description": "Second wing (optional)"}
                    }
                }
            },
            {
                "name": "mempalace_graph_stats",
                "description": "Palace graph overview",
                "inputSchema": {"type": "object", "properties": {}}
            },
            {
                "name": "mempalace_diary_write",
                "description": "Write to your personal agent diary",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "agent_name": {"type": "string", "description": "Your name"},
                        "entry": {"type": "string", "description": "Your diary entry in AAAK format"},
                        "topic": {"type": "string", "description": "Topic tag (optional, default: general)"}
                    },
                    "required": ["agent_name", "entry"]
                }
            },
            {
                "name": "mempalace_diary_read",
                "description": "Read your recent diary entries",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "agent_name": {"type": "string", "description": "Your name"},
                        "last_n": {"type": "integer", "description": "Number of recent entries (default: 10)"}
                    },
                    "required": ["agent_name"]
                }
            }
        ]);

        self.response(id, &serde_json::json!({ "tools": tools }))
    }

    /// Call a specific tool
    fn tool_call(&self, id: &Option<Value>, params: Option<&Value>) -> Value {
        let params = params
            .and_then(|v| v.get("arguments"))
            .and_then(|v| v.as_object());

        let tool_name = params.and_then(|p| p.get("name")).and_then(|v| v.as_str());

        let tool_args = params
            .and_then(|p| p.get("arguments"))
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let result: Value = match tool_name {
            Some("mempalace_status") => self.mcp_status(),
            Some("mempalace_list_wings") => self.mcp_list_wings(),
            Some("mempalace_list_rooms") => {
                self.mcp_list_rooms(tool_args.get("wing").and_then(|v| v.as_str()))
            }
            Some("mempalace_get_taxonomy") => self.mcp_get_taxonomy(),
            Some("mempalace_get_aaak_spec") => self.mcp_get_aaak_spec(),
            Some("mempalace_search") => self.mcp_search(&tool_args),
            Some("mempalace_check_duplicate") => self.mcp_check_duplicate(&tool_args),
            Some("mempalace_add_drawer") => self.mcp_add_drawer(&tool_args),
            Some("mempalace_delete_drawer") => {
                self.mcp_delete_drawer(tool_args.get("drawer_id").and_then(|v| v.as_str()))
            }
            Some("mempalace_kg_query") => self.mcp_kg_query(&tool_args),
            Some("mempalace_kg_add") => self.mcp_kg_add(&tool_args),
            Some("mempalace_kg_invalidate") => self.mcp_kg_invalidate(&tool_args),
            Some("mempalace_kg_timeline") => {
                self.mcp_kg_timeline(tool_args.get("entity").and_then(|v| v.as_str()))
            }
            Some("mempalace_kg_stats") => self.mcp_kg_stats(),
            Some("mempalace_traverse") => self.mcp_traverse(&tool_args),
            Some("mempalace_find_tunnels") => self.mcp_find_tunnels(
                tool_args.get("wing_a").and_then(|v| v.as_str()),
                tool_args.get("wing_b").and_then(|v| v.as_str()),
            ),
            Some("mempalace_graph_stats") => self.mcp_graph_stats(),
            Some("mempalace_diary_write") => self.mcp_diary_write(&tool_args),
            Some("mempalace_diary_read") => self.mcp_diary_read(&tool_args),
            Some(name) => serde_json::json!({"error": format!("Unknown tool: {}", name)}),
            None => serde_json::json!({"error": "Missing tool name"}),
        };

        self.response(
            id,
            &serde_json::json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}]
            }),
        )
    }

    // ==================== Tool Implementations ====================

    fn mcp_status(&self) -> Value {
        let drawer_count = self.storage.borrow().count().unwrap_or(0);
        let all_drawers = self.storage.borrow().get_all_drawers();

        // Build wing/room counts
        let mut wings: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut rooms: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for drawer in &all_drawers {
            *wings.entry(drawer.metadata.wing.clone()).or_insert(0) += 1;
            *rooms.entry(drawer.metadata.room.clone()).or_insert(0) += 1;
        }

        // KG stats
        let (entity_count, triple_count) = if let Some(ref kg) = self.knowledge_graph {
            (
                kg.get_entity_count().unwrap_or(0),
                kg.get_triple_count().unwrap_or(0),
            )
        } else {
            (0, 0)
        };

        serde_json::json!({
            "total_drawers": drawer_count,
            "total_entities": entity_count,
            "total_triples": triple_count,
            "wings": wings,
            "rooms": rooms,
            "palace_path": self.config.palace_path.to_string_lossy(),
            "kg_path": self.config.knowledge_graph_path.to_string_lossy(),
            "protocol": PALACE_PROTOCOL,
            "aaak_dialect": AAAK_SPEC
        })
    }

    fn mcp_list_wings(&self) -> Value {
        let all_drawers = self.storage.borrow().get_all_drawers();
        let mut wing_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for drawer in &all_drawers {
            *wing_counts.entry(drawer.metadata.wing.clone()).or_insert(0) += 1;
        }

        let wings: Vec<_> = wing_counts
            .into_iter()
            .map(|(name, count)| serde_json::json!({"name": name, "drawers": count}))
            .collect();

        serde_json::json!({"wings": wings})
    }

    fn mcp_list_rooms(&self, wing: Option<&str>) -> Value {
        let all_drawers = self.storage.borrow().get_all_drawers();
        let mut room_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for drawer in &all_drawers {
            if let Some(w) = wing {
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

        serde_json::json!({"wing": wing.unwrap_or("all"), "rooms": rooms})
    }

    fn mcp_get_taxonomy(&self) -> Value {
        let all_drawers = self.storage.borrow().get_all_drawers();
        let mut taxonomy: std::collections::HashMap<
            String,
            std::collections::HashMap<String, usize>,
        > = std::collections::HashMap::new();

        for drawer in &all_drawers {
            let wing = &drawer.metadata.wing;
            let room = &drawer.metadata.room;
            let rooms = taxonomy.entry(wing.clone()).or_default();
            *rooms.entry(room.clone()).or_insert(0) += 1;
        }

        serde_json::json!({"taxonomy": taxonomy})
    }

    fn mcp_get_aaak_spec(&self) -> Value {
        serde_json::json!({"aaak_spec": AAAK_SPEC})
    }

    fn mcp_search(&self, args: &Map<String, Value>) -> Value {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return serde_json::json!({"error": "Missing query"}),
        };
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(5) as usize;
        let wing = args.get("wing").and_then(|v| v.as_str());
        let room = args.get("room").and_then(|v| v.as_str());

        let searcher = SemanticSearcher::new(Rc::clone(&self.storage));
        match searcher.search(query, wing, room, limit) {
            Ok(results) => {
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
                serde_json::json!({
                    "results": hits,
                    "count": hits.len(),
                    "query": query,
                    "limit": limit,
                    "wing": wing,
                    "room": room
                })
            }
            Err(e) => serde_json::json!({"error": format!("Search failed: {}", e)}),
        }
    }

    fn mcp_check_duplicate(&self, args: &Map<String, Value>) -> Value {
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return serde_json::json!({"error": "Missing content"}),
        };
        let threshold = args
            .get("threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.9);

        // Search for similar content
        let searcher = SemanticSearcher::new(Rc::clone(&self.storage));
        match searcher.search(content, None, None, 5) {
            Ok(results) => {
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

                serde_json::json!({
                    "is_duplicate": !matches.is_empty(),
                    "matches": matches,
                    "content_preview": if content.chars().count() > 100 {
                        format!("{}...", content.chars().take(100).collect::<String>())
                    } else {
                        content.to_string()
                    },
                    "threshold": threshold
                })
            }
            Err(_) => serde_json::json!({
                "is_duplicate": false,
                "matches": [],
                "content_preview": if content.chars().count() > 100 {
                    format!("{}...", content.chars().take(100).collect::<String>())
                } else {
                    content.to_string()
                },
                "threshold": threshold
            }),
        }
    }

    fn mcp_add_drawer(&self, args: &Map<String, Value>) -> Value {
        let wing = match args.get("wing").and_then(|v| v.as_str()) {
            Some(w) => w,
            None => return serde_json::json!({"success": false, "error": "Missing wing"}),
        };
        let room = match args.get("room").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return serde_json::json!({"success": false, "error": "Missing room"}),
        };
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return serde_json::json!({"success": false, "error": "Missing content"}),
        };
        let source_file = args
            .get("source_file")
            .and_then(|v| v.as_str())
            .unwrap_or("mcp");
        let added_by = args
            .get("added_by")
            .and_then(|v| v.as_str())
            .unwrap_or("mcp");

        // Generate drawer ID
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
        let result = hasher.finalize();
        let drawer_id = format!("drawer_{}_{}", wing, hex::encode(&result[..8]));

        // Create metadata
        let metadata =
            DrawerMetadata::new(wing, room, source_file, 0, added_by, IngestMode::Projects);

        // Create and store the drawer
        let drawer = Drawer::new(drawer_id.clone(), content, metadata);

        let storage = Rc::clone(&self.storage);
        if let Err(e) = storage.borrow_mut().add_drawer(&drawer) {
            return serde_json::json!({"success": false, "error": format!("Failed to add drawer: {}", e)});
        }

        info!("Filed drawer: {} -> {}/{}", drawer_id, wing, room);

        serde_json::json!({
            "success": true,
            "drawer_id": drawer_id,
            "wing": wing,
            "room": room,
            "source_file": source_file,
            "added_by": added_by
        })
    }

    fn mcp_delete_drawer(&self, drawer_id: Option<&str>) -> Value {
        let drawer_id = match drawer_id {
            Some(id) => id,
            None => return serde_json::json!({"success": false, "error": "Missing drawer_id"}),
        };

        let storage = Rc::clone(&self.storage);
        if let Err(e) = storage.borrow_mut().delete_drawer(drawer_id) {
            return serde_json::json!({"success": false, "error": format!("Failed to delete: {}", e)});
        }

        info!("Deleted drawer: {}", drawer_id);

        serde_json::json!({
            "success": true,
            "drawer_id": drawer_id
        })
    }

    fn mcp_kg_query(&self, args: &Map<String, Value>) -> Value {
        let entity = match args.get("entity").and_then(|v| v.as_str()) {
            Some(e) => e,
            None => return serde_json::json!({"error": "Missing entity"}),
        };

        let kg = match &self.knowledge_graph {
            Some(kg) => kg,
            None => return serde_json::json!({"error": "Knowledge graph not available"}),
        };

        match kg.get_triples_for_entity(entity) {
            Ok(triples) => {
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

                serde_json::json!({
                    "entity": entity,
                    "as_of": args.get("as_of").and_then(|v| v.as_str()),
                    "direction": args.get("direction").and_then(|v| v.as_str()).unwrap_or("both"),
                    "facts": facts,
                    "count": facts.len()
                })
            }
            Err(e) => serde_json::json!({"error": format!("KG query failed: {}", e)}),
        }
    }

    fn mcp_kg_add(&self, args: &Map<String, Value>) -> Value {
        let subject = match args.get("subject").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return serde_json::json!({"error": "Missing subject"}),
        };
        let predicate = match args.get("predicate").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return serde_json::json!({"error": "Missing predicate"}),
        };
        let object = match args.get("object").and_then(|v| v.as_str()) {
            Some(o) => o,
            None => return serde_json::json!({"error": "Missing object"}),
        };

        let kg = match &self.knowledge_graph {
            Some(kg) => kg,
            None => return serde_json::json!({"error": "Knowledge graph not available"}),
        };

        let triple_id = format!("{}_{}_{}", subject, predicate, object);
        let triple = crate::storage::Triple {
            id: triple_id.clone(),
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            valid_from: args
                .get("valid_from")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
            valid_to: None,
            confidence: 1.0,
            source_closet: args
                .get("source_closet")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            source_file: "mcp".to_string(),
            extracted_at: Utc::now(),
        };

        match kg.upsert_triple(&triple) {
            Ok(()) => {
                info!("Added triple: {} -> {} -> {}", subject, predicate, object);
                serde_json::json!({
                    "success": true,
                    "triple_id": triple_id,
                    "fact": format!("{} -> {} -> {}", subject, predicate, object),
                    "valid_from": args.get("valid_from").and_then(|v| v.as_str())
                })
            }
            Err(e) => serde_json::json!({"error": format!("Failed to add triple: {}", e)}),
        }
    }

    fn mcp_kg_invalidate(&self, args: &Map<String, Value>) -> Value {
        let subject = match args.get("subject").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return serde_json::json!({"error": "Missing subject"}),
        };
        let predicate = match args.get("predicate").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return serde_json::json!({"error": "Missing predicate"}),
        };
        let object = match args.get("object").and_then(|v| v.as_str()) {
            Some(o) => o,
            None => return serde_json::json!({"error": "Missing object"}),
        };

        let kg = match &self.knowledge_graph {
            Some(kg) => kg,
            None => return serde_json::json!({"error": "Knowledge graph not available"}),
        };

        let ended_binding = Utc::now().format("%Y-%m-%d").to_string();
        let ended = args
            .get("ended")
            .and_then(|v| v.as_str())
            .unwrap_or(&ended_binding);

        // Find and update the triple with valid_to
        match kg.get_triples_for_entity(subject) {
            Ok(triples) => {
                for mut triple in triples {
                    if triple.subject == subject
                        && triple.predicate == predicate
                        && triple.object == object
                    {
                        triple.valid_to = chrono::NaiveDate::parse_from_str(ended, "%Y-%m-%d").ok();
                        if let Ok(()) = kg.upsert_triple(&triple) {
                            return serde_json::json!({
                                "success": true,
                                "fact": format!("{} -> {} -> {}", subject, predicate, object),
                                "ended": ended
                            });
                        }
                    }
                }
                serde_json::json!({"error": "Triple not found"})
            }
            Err(e) => serde_json::json!({"error": format!("Failed to query KG: {}", e)}),
        }
    }

    fn mcp_kg_timeline(&self, entity: Option<&str>) -> Value {
        let kg = match &self.knowledge_graph {
            Some(kg) => kg,
            None => return serde_json::json!({"error": "Knowledge graph not available"}),
        };

        let timeline: Vec<_> = if let Some(e) = entity {
            match kg.get_triples_for_entity(e) {
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
            // Get all triples, sorted by date
            vec![]
        };

        serde_json::json!({
            "entity": entity.unwrap_or("all"),
            "timeline": timeline,
            "count": timeline.len()
        })
    }

    fn mcp_kg_stats(&self) -> Value {
        let kg = match &self.knowledge_graph {
            Some(kg) => kg,
            None => return serde_json::json!({"error": "Knowledge graph not available"}),
        };

        let entities = kg.get_entity_count().unwrap_or(0);
        let triples = kg.get_triple_count().unwrap_or(0);

        serde_json::json!({
            "entities": entities,
            "triples": triples,
            "current_facts": triples,
            "expired_facts": 0,
            "relationship_types": []
        })
    }

    fn mcp_traverse(&self, args: &Map<String, Value>) -> Value {
        let start_room = match args.get("start_room").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return serde_json::json!({"error": "Missing start_room"}),
        };
        let max_hops = args.get("max_hops").and_then(|v| v.as_i64()).unwrap_or(2) as usize;

        let palace_graph = PalaceGraph::new(Rc::clone(&self.storage), self.config.clone());
        let direction = crate::graph::palace_graph::Direction::Forward;

        match palace_graph.navigate(start_room, direction) {
            Ok(rooms) => {
                let visited = std::collections::HashSet::from([start_room.to_string()]);
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

                serde_json::json!({
                    "start_room": start_room,
                    "max_hops": max_hops,
                    "connections": connections,
                    "visited": visited.into_iter().collect::<Vec<_>>()
                })
            }
            Err(e) => serde_json::json!({
                "start_room": start_room,
                "max_hops": max_hops,
                "connections": [],
                "visited": [start_room],
                "error": format!("{}", e)
            }),
        }
    }

    fn mcp_find_tunnels(&self, wing_a: Option<&str>, wing_b: Option<&str>) -> Value {
        let palace_graph = PalaceGraph::new(Rc::clone(&self.storage), self.config.clone());

        match palace_graph.find_all_tunnels(wing_a, wing_b) {
            Ok(tunnels) => {
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

                serde_json::json!({
                    "wing_a": wing_a.unwrap_or("all"),
                    "wing_b": wing_b.unwrap_or("all"),
                    "tunnels": tunnel_list
                })
            }
            Err(e) => serde_json::json!({
                "wing_a": wing_a.unwrap_or("all"),
                "wing_b": wing_b.unwrap_or("all"),
                "tunnels": [],
                "error": format!("{}", e)
            }),
        }
    }

    fn mcp_graph_stats(&self) -> Value {
        let palace_graph = PalaceGraph::new(Rc::clone(&self.storage), self.config.clone());

        match palace_graph.graph_stats() {
            Ok(stats) => {
                serde_json::json!({
                    "total_rooms": stats.total_rooms,
                    "total_connections": stats.total_edges,
                    "tunnel_rooms": stats.tunnel_rooms,
                    "rooms_per_wing": stats.rooms_per_wing
                })
            }
            Err(e) => serde_json::json!({
                "total_rooms": 0,
                "total_connections": 0,
                "error": format!("{}", e)
            }),
        }
    }

    fn mcp_diary_write(&self, args: &Map<String, Value>) -> Value {
        let agent_name = match args.get("agent_name").and_then(|v| v.as_str()) {
            Some(a) => a,
            None => return serde_json::json!({"success": false, "error": "Missing agent_name"}),
        };
        let entry = match args.get("entry").and_then(|v| v.as_str()) {
            Some(e) => e,
            None => return serde_json::json!({"success": false, "error": "Missing entry"}),
        };
        let topic = args
            .get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        let wing = format!("wing_{}", agent_name.to_lowercase().replace(' ', "_"));
        let room = format!("diary/{}", topic);
        let now = Utc::now();
        let entry_id = format!("diary_{}_{}", wing, now.format("%Y%m%d_%H%M%S"));

        // Generate drawer ID
        let mut hasher = Sha256::new();
        hasher.update(format!("{}_{}_{}_{}", wing, room, entry, now.to_rfc3339()).as_bytes());
        let result = hasher.finalize();
        let drawer_id = format!("{}_{}", entry_id, hex::encode(&result[..8]));

        // Create metadata
        let metadata = DrawerMetadata::new(
            wing.clone(),
            room.clone(),
            "diary",
            0,
            agent_name,
            IngestMode::Convos,
        );

        // Create and store the drawer
        let drawer = Drawer::new(drawer_id.clone(), entry, metadata);

        match self.storage.borrow_mut().add_drawer(&drawer) {
            Ok(_) => {
                info!("Wrote diary entry: {} -> {}/{}", entry_id, wing, topic);
                serde_json::json!({
                    "success": true,
                    "entry_id": entry_id,
                    "agent": agent_name,
                    "topic": topic,
                    "timestamp": now.to_rfc3339(),
                    "wing": wing,
                    "room": room
                })
            }
            Err(e) => {
                serde_json::json!({"success": false, "error": format!("Failed to write diary: {}", e)})
            }
        }
    }

    fn mcp_diary_read(&self, args: &Map<String, Value>) -> Value {
        let agent_name = match args.get("agent_name").and_then(|v| v.as_str()) {
            Some(a) => a,
            None => return serde_json::json!({"error": "Missing agent_name"}),
        };
        let last_n = args.get("last_n").and_then(|v| v.as_i64()).unwrap_or(10) as usize;

        let wing = format!("wing_{}", agent_name.to_lowercase().replace(' ', "_"));

        // Get all drawers for this wing, filter for diary entries
        let all_drawers = self.storage.borrow().get_all_drawers();
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

        serde_json::json!({
            "agent": agent_name,
            "wing": wing,
            "entries": entries,
            "total": total,
            "showing": total.min(last_n)
        })
    }

    // ==================== Helpers ====================

    fn response(&self, id: &Option<Value>, result: &Value) -> Value {
        let id = id.clone().unwrap_or(serde_json::Value::Null);
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })
    }

    fn error_response(&self, id: &Option<Value>, code: i32, message: &str) -> Value {
        let id = id.clone().unwrap_or(serde_json::Value::Null);
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        })
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new().expect("Failed to create MCP server")
    }
}

// ==================== AAAK Protocol Constants ====================

const PALACE_PROTOCOL: &str = r#"IMPORTANT — MemPalace Memory Protocol:
1. ON WAKE-UP: Call mempalace_status to load palace overview + AAAK spec.
2. BEFORE RESPONDING about any person, project, or past event: call mempalace_kg_query or mempalace_search FIRST. Never guess — verify.
3. IF UNSURE about a fact (name, gender, age, relationship): say "let me check" and query the palace. Wrong is worse than slow.
4. AFTER EACH SESSION: call mempalace_diary_write to record what happened, what you learned, what matters.
5. WHEN FACTS CHANGE: call mempalace_kg_invalidate on the old fact, mempalace_kg_add for the new one.

This protocol ensures the AI KNOWS before it speaks. Storage is not memory — but storage + this protocol = memory."#;

const AAAK_SPEC: &str = r#"AAAK is a compressed memory dialect that MemPalace uses for efficient storage.
It is designed to be readable by both humans and LLMs without decoding.

FORMAT:
  ENTITIES: 3-letter uppercase codes. ALC=Alice, JOR=Jordan, RIL=Riley, MAX=Max, BEN=Ben.
  EMOTIONS: *action markers* before/during text. *warm*=joy, *fierce*=determined, *raw*=vulnerable, *bloom*=tenderness.
  STRUCTURE: Pipe-separated fields. FAM: family | PROJ: projects | WARNING: warnings/reminders.
  DATES: ISO format (2026-03-31). COUNTS: Nx = N mentions (e.g., 570x).
  IMPORTANCE: STAR to STARSTARSTARSTARSTAR (1-5 scale).
  HALLS: hall_facts, hall_events, hall_discoveries, hall_preferences, hall_advice.
  WINGS: wing_user, wing_agent, wing_team, wing_code, wing_myproject, wing_hardware, wing_ue5, wing_ai_research.
  ROOMS: Hyphenated slugs representing named ideas (e.g., chromadb-setup, gpu-pricing).

EXAMPLE:
  FAM: ALC→HEART Jordan | 2D(kids): RIL(18,sports) MAX(11,chess+swimming) | BEN(contributor)

Read AAAK naturally — expand codes mentally, treat *markers* as emotional context.
When WRITING AAAK: use entity codes, mark emotions, keep structure tight."#;
