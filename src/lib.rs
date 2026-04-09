//! mempalace - A local-first memory palace system
//!
//! Ported from Python to Rust. Stores semantic memories in ChromaDB and
//! temporal knowledge graphs in SQLite.

#[cfg(feature = "bench")]
pub mod benchmark;
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
pub mod tokenizer;

pub use error::{MempalaceError, Result};

#[cfg(test)]
#[path = "./tests/commands_benchmark.rs"]
mod commands_benchmark;

#[cfg(test)]
#[path = "./tests/mcp_tools.rs"]
mod mcp_tools;

#[cfg(test)]
#[path = "./tests/mcp_tool_handlers.rs"]
mod mcp_tool_handlers;

#[cfg(test)]
#[path = "./tests/tokenizer.rs"]
mod tokenizer_tests;
