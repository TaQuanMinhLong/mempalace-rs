//! mempalace - A local-first memory palace system
//!
//! Ported from Python to Rust. Stores semantic memories in ChromaDB and
//! temporal knowledge graphs in SQLite.

pub mod config;
pub mod dialect;
pub mod error;
pub mod extract;
pub mod graph;
pub mod layers;
pub mod mcp;
pub mod miner;
pub mod normalize;
pub mod palace;
pub mod registry;
pub mod search;
pub mod storage;

pub use error::{MempalaceError, Result};
