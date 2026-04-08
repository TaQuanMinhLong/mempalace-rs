//! ChromaDB storage wrapper
//!
//! Note: ChromaDB doesn't have an official Rust client yet.
//! For now, this module provides a stub that can be implemented once
//! an official client is available, or via HTTP API to a ChromaDB server.

use crate::error::Result;
use crate::palace::Drawer;
use std::path::Path;

/// Search result with distance
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub drawer: Drawer,
    pub distance: Option<f64>,
}

/// ChromaDB storage for drawers
#[derive(Debug, Clone)]
pub struct ChromaStorage {
    _palace_path: std::path::PathBuf,
    _collection_name: String,
    // In-memory fallback for testing
    _drawers: Vec<Drawer>,
}

impl ChromaStorage {
    /// Create a new ChromaDB storage
    pub fn new(palace_path: &Path, collection_name: &str) -> Result<Self> {
        Ok(Self {
            _palace_path: palace_path.to_path_buf(),
            _collection_name: collection_name.to_string(),
            _drawers: Vec::new(),
        })
    }

    /// Add a drawer to storage
    pub fn add_drawer(&mut self, drawer: &Drawer) -> Result<()> {
        // For now, store in memory as fallback
        self._drawers.push(drawer.clone());
        Ok(())
    }

    /// Get top drawers by importance/recency
    pub fn get_top_drawers(&self, limit: usize, wing: Option<&str>) -> Vec<Drawer> {
        let mut drawers: Vec<_> = self._drawers.clone();

        // Filter by wing if specified
        if let Some(w) = wing {
            drawers.retain(|d| d.metadata.wing == w);
        }

        // Sort by importance (default 3.0) and take top N
        drawers.sort_by(|a, b| {
            let imp_a = a.metadata.importance.unwrap_or(3.0);
            let imp_b = b.metadata.importance.unwrap_or(3.0);
            imp_b
                .partial_cmp(&imp_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        drawers.into_iter().take(limit).collect()
    }

    /// Get drawers filtered by wing and/or room
    pub fn get_drawers_by_filter(
        &self,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Vec<Drawer> {
        let mut drawers: Vec<_> = self._drawers.clone();

        if let Some(w) = wing {
            drawers.retain(|d| d.metadata.wing == w);
        }

        if let Some(r) = room {
            drawers.retain(|d| d.metadata.room == r);
        }

        drawers.into_iter().take(limit).collect()
    }

    /// Search for drawers (semantic search - uses keyword fallback)
    pub fn search(
        &self,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Vec<crate::layers::SearchHit> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<_> = self._drawers.clone();

        // Filter by wing/room
        if let Some(w) = wing {
            results.retain(|d| d.metadata.wing == w);
        }
        if let Some(r) = room {
            results.retain(|d| d.metadata.room == r);
        }

        // Simple keyword matching as fallback
        results.retain(|d| d.document.to_lowercase().contains(&query_lower));

        results
            .into_iter()
            .take(limit)
            .map(|d| crate::layers::SearchHit::from_drawer(&d, Some(0.5)))
            .collect()
    }

    /// Delete a drawer
    pub fn delete_drawer(&mut self, id: &str) -> Result<()> {
        self._drawers.retain(|d| d.id != id);
        Ok(())
    }

    /// Count total drawers
    pub fn count(&self) -> Result<usize> {
        Ok(self._drawers.len())
    }

    /// Get all drawers
    pub fn get_all_drawers(&self) -> Vec<Drawer> {
        self._drawers.clone()
    }
}
