//! Semantic search - ChromaDB query integration
//!
//! Port from Python searcher.py. Provides semantic search against the palace
//! with optional wing/room filtering.
//!
//! SemanticSearcher is a wrapper around ChromaStorage that provides:
//! - A stable interface isolating callers from storage implementation details
//! - Future hook for query transformation (e.g., query expansion, synonyms)
//! - Future hook for result caching
//! - Consistent error handling via Result<Vec<SearchResult>>

use crate::error::{MempalaceError, Result};
use crate::layers::SearchHit;
use crate::storage::ChromaStorage;
use std::cell::RefCell;
use std::rc::Rc;

/// Search result with similarity score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub hit: SearchHit,
}

/// Semantic searcher for the memory palace
pub struct SemanticSearcher {
    storage: Rc<RefCell<ChromaStorage>>,
}

impl SemanticSearcher {
    /// Create a new semantic searcher
    pub fn new(storage: Rc<RefCell<ChromaStorage>>) -> Self {
        Self { storage }
    }

    /// Search the palace for drawers matching the query.
    ///
    /// Optionally filter by wing (project) and/or room (aspect).
    /// Returns results sorted by similarity score (descending).
    pub fn search(
        &self,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        if query.is_empty() {
            return Err(MempalaceError::Search("Query cannot be empty".into()));
        }

        let limit = if limit == 0 { 5 } else { limit };

        // Delegate to storage layer which will query ChromaDB
        let hits = self.storage.borrow().search(query, wing, room, limit);

        // Convert to SearchResult
        let results = hits.into_iter().map(|hit| SearchResult { hit }).collect();

        Ok(results)
    }

    /// Search and return results as a structured response
    pub fn search_with_context(
        &self,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Result<SearchResponse> {
        let results = self.search(query, wing, room, limit)?;

        Ok(SearchResponse {
            query: query.to_string(),
            filters: SearchFilters {
                wing: wing.map(String::from),
                room: room.map(String::from),
            },
            results: results.into_iter().map(|r| r.hit).collect(),
        })
    }
}

/// Search response for programmatic access
#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub query: String,
    pub filters: SearchFilters,
    pub results: Vec<SearchHit>,
}

/// Search filters
#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub wing: Option<String>,
    pub room: Option<String>,
}

#[cfg(test)]
#[path = "../tests/search_semantic.rs"]
mod tests;
