//! Chat parser - multi-format chat parser
//!
//! Supports 6 formats:
//! 1. Plain text with > markers (pass through)
//! 2. Claude.ai JSON export
//! 3. ChatGPT conversations.json
//! 4. Claude Code JSONL
//! 5. OpenAI Codex CLI JSONL
//! 6. Slack JSON export

use crate::error::{MempalaceError, Result};
use chrono::{DateTime, Utc};

/// Chat format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatFormat {
    PlainText,
    ClaudeAi,
    ChatGPT,
    ClaudeCode,
    Codex,
    Slack,
}

impl ChatFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatFormat::PlainText => "plaintext",
            ChatFormat::ClaudeAi => "claudeai",
            ChatFormat::ChatGPT => "chatgpt",
            ChatFormat::ClaudeCode => "claudecode",
            ChatFormat::Codex => "codex",
            ChatFormat::Slack => "slack",
        }
    }
}

/// Exchange in a conversation
#[derive(Debug, Clone)]
pub struct Exchange {
    pub role: String,
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
}

/// Chat parser
#[derive(Debug, Clone)]
pub struct ChatParser;

impl ChatParser {
    pub fn new() -> Self {
        Self
    }

    /// Detect format from content
    pub fn detect_format(&self, content: &str) -> ChatFormat {
        let trimmed = content.trim();

        // Check for plain text with > markers
        let quote_lines = trimmed
            .lines()
            .filter(|line| line.trim().starts_with('>'))
            .count();
        if quote_lines >= 3 {
            return ChatFormat::PlainText;
        }

        // Check for JSON-based formats
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            // Try to parse and detect specific JSON format
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                // ChatGPT conversations.json has a "mapping" field
                if json.get("mapping").is_some() {
                    return ChatFormat::ChatGPT;
                }

                // Claude.ai JSON export - check for messages array or privacy export
                if json.get("messages").is_some()
                    || json.get("chat_messages").is_some()
                    || (json.is_array()
                        && json
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|v| v.get("chat_messages"))
                            .is_some())
                {
                    return ChatFormat::ClaudeAi;
                }
            }

            // Try JSONL format for Claude Code or Codex
            if trimmed.contains('\n') {
                let first_line = trimmed.lines().next().unwrap_or("");
                if let Ok(entry) = serde_json::from_str::<serde_json::Value>(first_line) {
                    let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    // Claude Code JSONL has "type" field with "human" or "assistant"
                    if entry_type == "human" || entry_type == "user" || entry_type == "assistant" {
                        return ChatFormat::ClaudeCode;
                    }

                    // Codex JSONL has "type": "event_msg" with payload.type
                    if entry_type == "event_msg" || entry_type == "session_meta" {
                        return ChatFormat::Codex;
                    }
                }
            }
        }

        // Slack JSON export is an array of message objects
        if trimmed.starts_with('[') {
            if let Ok(arr) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if arr.is_array() {
                    if let Some(first) = arr.as_array().and_then(|a| a.first()) {
                        if first.get("type").is_some() && first.get("text").is_some() {
                            return ChatFormat::Slack;
                        }
                    }
                }
            }
        }

        ChatFormat::PlainText
    }

    /// Normalize content to exchanges
    pub fn normalize(&self, content: &str, format: ChatFormat) -> Result<Vec<Exchange>> {
        match format {
            ChatFormat::PlainText => self.normalize_plain_text(content),
            ChatFormat::ClaudeAi => self.normalize_claude_ai(content),
            ChatFormat::ChatGPT => self.normalize_chatgpt(content),
            ChatFormat::ClaudeCode => self.normalize_claude_code(content),
            ChatFormat::Codex => self.normalize_codex(content),
            ChatFormat::Slack => self.normalize_slack(content),
        }
    }

    /// Normalize a file by detecting format and converting to transcript
    pub fn normalize_file(&self, content: &str) -> Result<Vec<Exchange>> {
        let format = self.detect_format(content);
        self.normalize(content, format)
    }

    /// Convert exchanges to transcript format with > markers
    pub fn to_transcript(&self, exchanges: &[Exchange]) -> String {
        let mut lines = Vec::new();
        let mut i = 0;

        while i < exchanges.len() {
            let exchange = &exchanges[i];

            if exchange.role == "user" {
                lines.push(format!("> {}", exchange.content));

                // If next is assistant, include it directly
                if i + 1 < exchanges.len() && exchanges[i + 1].role == "assistant" {
                    lines.push(exchanges[i + 1].content.clone());
                    i += 2;
                } else {
                    i += 1;
                }
            } else {
                lines.push(exchange.content.clone());
                i += 1;
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }

    fn normalize_plain_text(&self, content: &str) -> Result<Vec<Exchange>> {
        let mut exchanges = Vec::new();
        let mut current_role: Option<&str> = None;
        let mut current_content = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with('>') {
                // Save previous exchange
                if let Some(role) = current_role {
                    if !current_content.trim().is_empty() {
                        exchanges.push(Exchange {
                            role: role.to_string(),
                            content: current_content.trim().to_string(),
                            timestamp: None,
                        });
                    }
                }

                current_role = Some("user");
                current_content = trimmed.trim_start_matches("> ").to_string();
            } else if trimmed.starts_with("---") {
                // Skip separator lines
                continue;
            } else if !trimmed.is_empty() {
                // Accumulate assistant content
                if current_role.is_none() {
                    current_role = Some("assistant");
                }

                if current_role == Some("assistant") || exchanges.is_empty() {
                    if !current_content.is_empty() {
                        current_content.push(' ');
                    }
                    current_content.push_str(trimmed);
                }
            } else if !current_content.is_empty() && current_role == Some("assistant") {
                // Empty line after content - save the exchange
                exchanges.push(Exchange {
                    role: "assistant".to_string(),
                    content: current_content.trim().to_string(),
                    timestamp: None,
                });
                current_content.clear();
                current_role = None;
            }
        }

        // Don't forget the last exchange
        if let Some(role) = current_role {
            if !current_content.trim().is_empty() {
                exchanges.push(Exchange {
                    role: role.to_string(),
                    content: current_content.trim().to_string(),
                    timestamp: None,
                });
            }
        }

        Ok(exchanges)
    }

    fn normalize_claude_ai(&self, content: &str) -> Result<Vec<Exchange>> {
        let data: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| MempalaceError::Normalization(format!("Invalid JSON: {}", e)))?;

        let messages = if data.is_array() {
            // Privacy export: array of conversation objects with chat_messages inside
            if data.get(0).and_then(|v| v.get("chat_messages")).is_some() {
                let mut all_messages = Vec::new();
                for convo in data.as_array().unwrap() {
                    if let Some(chat_messages) =
                        convo.get("chat_messages").and_then(|v| v.as_array())
                    {
                        for item in chat_messages {
                            if let Some(msg) = self.extract_claude_message(item) {
                                all_messages.push(msg);
                            }
                        }
                    }
                }
                all_messages
            } else {
                // Flat messages list
                data.as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|item| self.extract_claude_message(item))
                    .collect()
            }
        } else if let Some(msgs) = data.get("messages").and_then(|v| v.as_array()) {
            msgs.iter()
                .filter_map(|item| self.extract_claude_message(item))
                .collect()
        } else if let Some(msgs) = data.get("chat_messages").and_then(|v| v.as_array()) {
            msgs.iter()
                .filter_map(|item| self.extract_claude_message(item))
                .collect()
        } else {
            return Ok(Vec::new());
        };

        if messages.len() >= 2 {
            Ok(messages)
        } else {
            Ok(Vec::new())
        }
    }

    fn extract_claude_message(&self, item: &serde_json::Value) -> Option<Exchange> {
        let role = item.get("role")?.as_str()?;
        let text = self.extract_content(item.get("content")?.clone());

        match role {
            "user" | "human" if !text.is_empty() => Some(Exchange {
                role: "user".to_string(),
                content: text,
                timestamp: None,
            }),
            "assistant" | "ai" if !text.is_empty() => Some(Exchange {
                role: "assistant".to_string(),
                content: text,
                timestamp: None,
            }),
            _ => None,
        }
    }

    fn normalize_chatgpt(&self, content: &str) -> Result<Vec<Exchange>> {
        let data: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| MempalaceError::Normalization(format!("Invalid JSON: {}", e)))?;

        let mapping = data
            .get("mapping")
            .and_then(|v| v.as_object())
            .ok_or_else(|| MempalaceError::Normalization("Missing 'mapping' field".to_string()))?;

        // Find root node (parent=None and no message)
        let mut root_id: Option<&String> = None;
        let mut fallback_root: Option<&String> = None;

        for (node_id, node) in mapping {
            if node.get("parent").is_none() {
                if node.get("message").is_none() {
                    root_id = Some(node_id);
                    break;
                } else if fallback_root.is_none() {
                    fallback_root = Some(node_id);
                }
            }
        }

        let root_id = root_id
            .or(fallback_root)
            .ok_or_else(|| MempalaceError::Normalization("Could not find root node".to_string()))?;

        // Build adjacency list: node_id -> list of child ids (owned strings)
        let children: std::collections::HashMap<String, Vec<String>> = mapping
            .iter()
            .map(|(id, node)| {
                let child_ids: Vec<String> = node
                    .get("children")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                (id.clone(), child_ids)
            })
            .collect();

        // Topological sort via DFS post-order
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut stack: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut order: Vec<String> = Vec::new();

        fn dfs(
            node_id: &str,
            children: &std::collections::HashMap<String, Vec<String>>,
            visited: &mut std::collections::HashSet<String>,
            stack: &mut std::collections::HashSet<String>,
            order: &mut Vec<String>,
        ) {
            if visited.contains(node_id) {
                return;
            }
            if stack.contains(node_id) {
                // Cycle detected — skip
                return;
            }
            visited.insert(node_id.to_string());
            stack.insert(node_id.to_string());
            if let Some(child_ids) = children.get(node_id) {
                for child_id in child_ids {
                    dfs(child_id, children, visited, stack, order);
                }
            }
            stack.remove(node_id);
            order.push(node_id.to_string());
        }

        dfs(root_id, &children, &mut visited, &mut stack, &mut order);

        // Also visit any nodes not reachable from root (orphans)
        for node_id in mapping.keys() {
            dfs(node_id, &children, &mut visited, &mut stack, &mut order);
        }

        // Collect exchanges in topological order
        let mut messages = Vec::new();
        for node_id in order {
            let node = mapping.get(node_id.as_str()).unwrap();
            let msg = node.get("message");

            if let Some(msg_obj) = msg.and_then(|v| v.as_object()) {
                let role = msg_obj
                    .get("author")
                    .and_then(|v| v.get("role"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let content = msg_obj.get("content").and_then(|v| v.as_object());

                let text = if let Some(parts) = content
                    .and_then(|c| c.get("parts"))
                    .and_then(|v| v.as_array())
                {
                    parts
                        .iter()
                        .filter_map(|p| p.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string()
                } else {
                    String::new()
                };

                if !text.is_empty() {
                    match role {
                        "user" => messages.push(Exchange {
                            role: "user".to_string(),
                            content: text,
                            timestamp: None,
                        }),
                        "assistant" => messages.push(Exchange {
                            role: "assistant".to_string(),
                            content: text,
                            timestamp: None,
                        }),
                        _ => {}
                    }
                }
            }
        }

        if messages.len() >= 2 {
            Ok(messages)
        } else {
            Ok(Vec::new())
        }
    }

    fn normalize_claude_code(&self, content: &str) -> Result<Vec<Exchange>> {
        let mut messages = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let entry: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if !entry.is_object() {
                continue;
            }

            let msg_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let message = entry.get("message").ok_or_else(|| {
                MempalaceError::Normalization("Missing 'message' field".to_string())
            })?;

            if msg_type == "human" || msg_type == "user" {
                let text = self.extract_content(message.clone());
                if !text.is_empty() {
                    messages.push(Exchange {
                        role: "user".to_string(),
                        content: text,
                        timestamp: None,
                    });
                }
            } else if msg_type == "assistant" {
                let text = self.extract_content(message.clone());
                if !text.is_empty() {
                    messages.push(Exchange {
                        role: "assistant".to_string(),
                        content: text,
                        timestamp: None,
                    });
                }
            }
        }

        if messages.len() >= 2 {
            Ok(messages)
        } else {
            Ok(Vec::new())
        }
    }

    fn normalize_codex(&self, content: &str) -> Result<Vec<Exchange>> {
        let mut messages = Vec::new();
        let mut has_session_meta = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let entry: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if !entry.is_object() {
                continue;
            }

            let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");

            if entry_type == "session_meta" {
                has_session_meta = true;
                continue;
            }

            if entry_type != "event_msg" {
                continue;
            }

            let payload = entry.get("payload").and_then(|v| v.as_object());
            let payload_type = payload
                .and_then(|p| p.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let msg = payload
                .and_then(|p| p.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if msg.is_empty() {
                continue;
            }

            if payload_type == "user_message" {
                messages.push(Exchange {
                    role: "user".to_string(),
                    content: msg.trim().to_string(),
                    timestamp: None,
                });
            } else if payload_type == "agent_message" {
                messages.push(Exchange {
                    role: "assistant".to_string(),
                    content: msg.trim().to_string(),
                    timestamp: None,
                });
            }
        }

        if messages.len() >= 2 && has_session_meta {
            Ok(messages)
        } else {
            Ok(Vec::new())
        }
    }

    fn normalize_slack(&self, content: &str) -> Result<Vec<Exchange>> {
        let data: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| MempalaceError::Normalization(format!("Invalid JSON: {}", e)))?;

        let items = data.as_array().ok_or_else(|| {
            MempalaceError::Normalization("Expected array of messages".to_string())
        })?;

        let mut messages = Vec::new();
        let mut seen_users = std::collections::HashMap::new();
        let mut last_role: Option<&'static str> = None;

        for item in items {
            if !item.is_object() {
                continue;
            }

            if item.get("type").and_then(|v| v.as_str()) != Some("message") {
                continue;
            }

            let user = item
                .get("user")
                .or_else(|| item.get("username"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let text = item
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();

            if text.is_empty() || user.is_empty() {
                continue;
            }

            let role = if seen_users.is_empty() {
                seen_users.insert(user.to_string(), "user".to_string());
                "user"
            } else if last_role.is_some_and(|r| r == "user") {
                seen_users.insert(user.to_string(), "assistant".to_string());
                "assistant"
            } else {
                seen_users.insert(user.to_string(), "user".to_string());
                "user"
            };

            let role_str = role.to_string();
            last_role = Some(role);
            messages.push(Exchange {
                role: role_str,
                content: text.to_string(),
                timestamp: None,
            });
        }

        if messages.len() >= 2 {
            Ok(messages)
        } else {
            Ok(Vec::new())
        }
    }

    /// Extract text content from various content formats
    fn extract_content(&self, content: serde_json::Value) -> String {
        if let Some(s) = content.as_str() {
            return s.trim().to_string();
        }

        if let Some(arr) = content.as_array() {
            let parts: Vec<String> = arr
                .iter()
                .filter_map(|item| {
                    if let Some(s) = item.as_str() {
                        Some(s.to_string())
                    } else if item.is_object()
                        && item.get("type") == Some(&serde_json::Value::String("text".to_string()))
                    {
                        item.get("text")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            return parts.join(" ").trim().to_string();
        }

        if let Some(obj) = content.as_object() {
            // Handle Claude Code format: {"content": [{"type": "text", "text": "..."}]}
            if let Some(inner) = obj.get("content") {
                return self.extract_content(inner.clone());
            }
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                return text.trim().to_string();
            }
        }

        String::new()
    }
}

impl Default for ChatParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../tests/normalize_parser.rs"]
mod tests;
