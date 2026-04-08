//! MCP server - Model Context Protocol server

pub mod server;
pub(crate) mod tools;

pub use server::{serve, McpServer};
