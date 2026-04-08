pub(crate) use crate::mcp::server::McpServer;
pub(crate) use rmcp::model::{CallToolResult, Content};
pub(crate) use rmcp::schemars::JsonSchema;
pub(crate) use rmcp::ErrorData as McpError;
pub(crate) use serde::Deserialize;

pub mod catalog;
pub mod diary;
pub mod drawers;
pub mod graph;
pub mod knowledge_graph;
pub mod protocol;
pub mod search;
