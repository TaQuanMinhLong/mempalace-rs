//! mempalace - A local-first memory palace system
//!
//! Ported from Python to Rust. Stores semantic memories in ChromaDB and
//! temporal knowledge graphs in SQLite.

pub mod commands;
pub mod config;
pub mod dialect;
pub mod error;
pub mod extract;
pub mod graph;
pub mod layers;
pub mod logger;
pub mod mcp;
pub mod miner;
pub mod normalize;
pub mod palace;
pub mod registry;
pub mod search;
pub mod storage;

pub use error::{MempalaceError, Result};

#[cfg(test)]
#[path = "./tests/mcp_tools.rs"]
mod mcp_tools;

#[cfg(test)]
#[path = "./tests/mcp_tool_handlers.rs"]
mod mcp_tool_handlers;
