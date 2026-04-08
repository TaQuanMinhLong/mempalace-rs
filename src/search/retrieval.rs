//! Retrieval - layer-based retrieval (L0-L3)
//!
//! Port from Python retrieval logic. Provides context retrieval from the
//! 4-layer memory stack:
//!   L0: Identity (~100 tokens, always loaded)
//!   L1: Essential story (~500-800 tokens, always loaded)
//!   L2: Wing/room filtered (~200-500 tokens, on demand)
//!   L3: Full semantic search (unlimited, on demand)

use crate::error::Result;
use crate::layers::{MemoryLayer, MemoryStack};

/// Retriever for memory layers
#[derive(Debug, Clone)]
pub struct Retriever;

impl Retriever {
    /// Create a new retriever
    pub fn new() -> Self {
        Self
    }

    /// Retrieve context from memory stack using wake-up + recall.
    ///
    /// Combines layers based on query:
    /// - L0 (identity) is always included via wake_up
    /// - L1 (essential) is included via wake_up
    /// - L2 (filtered) can be included via recall
    pub fn retrieve(
        &self,
        stack: &MemoryStack,
        storage: &crate::storage::ChromaStorage,
        _query: Option<&str>,
    ) -> Result<String> {
        let mut stack_mut = stack.clone();
        let context = stack_mut.wake_up(storage, None);
        Ok(context)
    }

    /// Retrieve from a specific layer only
    pub fn retrieve_layer(
        &self,
        stack: &MemoryStack,
        storage: &crate::storage::ChromaStorage,
        layer: MemoryLayer,
    ) -> Result<String> {
        match layer {
            MemoryLayer::L0 => {
                let mut l0 = stack.l0.clone();
                Ok(l0.render())
            }
            MemoryLayer::L1 => {
                let l1 = stack.l1.generate(storage);
                Ok(l1)
            }
            MemoryLayer::L2 => {
                let context = stack.l2.retrieve(storage, None, None, 10);
                Ok(context)
            }
            MemoryLayer::L3 => {
                // L3 is semantic search - handled separately via SemanticSearcher
                Ok(String::new())
            }
        }
    }

    /// Get a summary of what's in each layer
    pub fn layer_summary(
        &self,
        stack: &MemoryStack,
        storage: &crate::storage::ChromaStorage,
    ) -> LayerSummary {
        let mut l0 = stack.l0.clone();
        LayerSummary {
            l0_chars: l0.render().len(),
            l0_preview: l0.render().chars().take(100).collect(),
            l1_count: storage.count().unwrap_or(0),
            l2_count: storage.count().unwrap_or(0),
        }
    }
}

impl Default for Retriever {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of memory layers
#[derive(Debug, Clone)]
pub struct LayerSummary {
    pub l0_chars: usize,
    pub l0_preview: String,
    pub l1_count: usize,
    pub l2_count: usize,
}

#[cfg(test)]
#[path = "../tests/search_retrieval.rs"]
mod tests;
