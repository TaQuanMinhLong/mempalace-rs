//! 4-layer memory stack
//!
//! L0: Identity (~100 tokens, always loaded)
//! L1: Essential story (~500-800 tokens, always loaded)
//! L2: Wing/room filtered (~200-500 tokens, on demand)
//! L3: Full semantic search (unlimited, on demand)
//!
//! Wake-up cost: ~600-900 tokens (L0+L1). Leaves 95%+ of context free.

use crate::palace::{Drawer, DrawerMetadata};
use crate::storage::ChromaStorage;
use std::collections::HashMap;
use std::path::PathBuf;

/// Memory layer levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLayer {
    L0,
    L1,
    L2,
    L3,
}

impl MemoryLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryLayer::L0 => "L0",
            MemoryLayer::L1 => "L1",
            MemoryLayer::L2 => "L2",
            MemoryLayer::L3 => "L3",
        }
    }
}

// ============================================================================
// Layer 0 — Identity (~100 tokens)
// ============================================================================

/// Layer 0: Identity text from ~/.mempalace/identity.txt
#[derive(Debug, Clone)]
pub struct Layer0 {
    path: PathBuf,
    cached_text: Option<String>,
}

impl Layer0 {
    /// Create a new Layer0 instance
    pub fn new(identity_path: Option<PathBuf>) -> Self {
        let path = identity_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            PathBuf::from(format!("{}/.mempalace/identity.txt", home))
        });
        Self {
            path,
            cached_text: None,
        }
    }

    /// Render the identity text
    pub fn render(&mut self) -> String {
        if let Some(ref text) = self.cached_text {
            return text.clone();
        }

        let text = if self.path.exists() {
            std::fs::read_to_string(&self.path)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| Self::default_text())
        } else {
            Self::default_text()
        };

        self.cached_text = Some(text.clone());
        text
    }

    /// Token estimate (rough: chars / 4)
    pub fn token_estimate(&mut self) -> usize {
        self.render().len() / 4
    }

    /// Check if identity file exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    fn default_text() -> String {
        "## L0 — IDENTITY\nNo identity configured. Create ~/.mempalace/identity.txt".to_string()
    }
}

// ============================================================================
// Layer 1 — Essential Story (~500-800 tokens)
// ============================================================================

const LAYER1_MAX_DRAWERS: usize = 15;
const LAYER1_MAX_CHARS: usize = 3200;

/// Layer 1: Essential story from top palace drawers
#[derive(Debug, Clone)]
pub struct Layer1 {
    // palace_path: PathBuf, // not stored; passed as constructor arg
    wing: Option<String>,
}

impl Layer1 {
    /// Create a new Layer1 instance
    pub fn new(palace_path: Option<PathBuf>, wing: Option<String>) -> Self {
        let _palace_path = palace_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            PathBuf::from(format!("{}/.mempalace/palace", home))
        });
        Self { wing }
    }

    /// Set wing filter
    pub fn with_wing(mut self, wing: String) -> Self {
        self.wing = Some(wing);
        self
    }

    /// Generate L1 text from top drawers
    pub fn generate(&self, storage: &ChromaStorage) -> String {
        let drawers = storage.get_top_drawers(LAYER1_MAX_DRAWERS, self.wing.as_deref());

        if drawers.is_empty() {
            return "## L1 — No memories yet.".to_string();
        }

        // Score and sort by importance
        let mut scored: Vec<(f64, &Drawer)> = drawers
            .iter()
            .map(|d| {
                let importance = d
                    .metadata
                    .emotional_weight
                    .or(d.metadata.importance)
                    .unwrap_or(3.0);
                (importance, d)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let top: Vec<_> = scored.into_iter().take(LAYER1_MAX_DRAWERS).collect();

        // Group by room
        let mut by_room: HashMap<String, Vec<(f64, &Drawer)>> = HashMap::new();
        for (importance, drawer) in &top {
            by_room
                .entry(drawer.metadata.room.clone())
                .or_default()
                .push((*importance, *drawer));
        }

        // Build text
        let mut lines = vec!["## L1 — ESSENTIAL STORY".to_string()];
        let mut total_len = lines[0].len();

        let mut rooms: Vec<_> = by_room.keys().collect();
        rooms.sort();

        for room in rooms {
            let room_line = format!("\n[{}]", room);
            if total_len + room_line.len() > LAYER1_MAX_CHARS {
                lines.push("  ... (more in L3 search)".to_string());
                return lines.join("\n");
            }
            lines.push(room_line.clone());
            total_len += room_line.len();

            for (_, drawer) in &by_room[room] {
                let source = std::path::Path::new(&drawer.metadata.source_file)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                let snippet = drawer
                    .document
                    .trim()
                    .replace('\n', " ")
                    .chars()
                    .take(200)
                    .collect::<String>();

                let snippet = if drawer.document.len() > 200 {
                    format!("{}...", snippet)
                } else {
                    snippet
                };

                let mut entry = format!("  - {}", snippet);
                if !source.is_empty() {
                    entry.push_str(&format!("  ({})", source));
                }

                if total_len + entry.len() > LAYER1_MAX_CHARS {
                    lines.push("  ... (more in L3 search)".to_string());
                    return lines.join("\n");
                }

                lines.push(entry.clone());
                total_len += entry.len();
            }
        }

        lines.join("\n")
    }
}

// ============================================================================
// Layer 2 — On-Demand (~200-500 tokens per retrieval)
// ============================================================================

/// Layer 2: Wing/room filtered retrieval
#[derive(Debug, Clone)]
pub struct Layer2 {
    // palace_path: PathBuf, // not stored; passed as constructor arg
}

impl Layer2 {
    /// Create a new Layer2 instance
    pub fn new(palace_path: Option<PathBuf>) -> Self {
        let _palace_path = palace_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            PathBuf::from(format!("{}/.mempalace/palace", home))
        });
        Self {}
    }

    /// Retrieve drawers filtered by wing/room
    pub fn retrieve(
        &self,
        storage: &ChromaStorage,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> String {
        let drawers = storage.get_drawers_by_filter(wing, room, n_results);

        if drawers.is_empty() {
            let mut label = String::new();
            if let Some(w) = wing {
                label.push_str(&format!("wing={}", w));
            }
            if let Some(r) = room {
                if !label.is_empty() {
                    label.push(' ');
                }
                label.push_str(&format!("room={}", r));
            }
            return format!("No drawers found for {}.", label);
        }

        let mut lines = vec![format!("## L2 — ON-DEMAND ({} drawers)", drawers.len())];

        for drawer in drawers.iter().take(n_results) {
            let room_name = &drawer.metadata.room;
            let source = std::path::Path::new(&drawer.metadata.source_file)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let snippet = drawer
                .document
                .trim()
                .replace('\n', " ")
                .chars()
                .take(300)
                .collect::<String>();

            let snippet = if drawer.document.len() > 300 {
                format!("{}...", snippet)
            } else {
                snippet
            };

            let mut entry = format!("  [{}] {}", room_name, snippet);
            if !source.is_empty() {
                entry.push_str(&format!("  ({})", source));
            }

            lines.push(entry);
        }

        lines.join("\n")
    }
}

// ============================================================================
// Layer 3 — Deep Search (full semantic search)
// ============================================================================

/// Layer 3: Deep semantic search
#[derive(Debug, Clone)]
pub struct Layer3 {
    // palace_path: PathBuf, // not stored; passed as constructor arg
}

impl Layer3 {
    /// Create a new Layer3 instance
    pub fn new(palace_path: Option<PathBuf>) -> Self {
        let _palace_path = palace_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            PathBuf::from(format!("{}/.mempalace/palace", home))
        });
        Self {}
    }

    /// Semantic search, returns compact result text
    pub fn search(
        &self,
        storage: &ChromaStorage,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> String {
        let results = storage.search(query, wing, room, n_results);

        if results.is_empty() {
            return "No results found.".to_string();
        }

        let mut lines = vec![format!("## L3 — SEARCH RESULTS for \"{}\"", query)];

        for (i, result) in results.iter().enumerate().take(n_results) {
            let similarity = (1.0 - result.distance.unwrap_or(1.0)).round() / 1000.0;
            let wing_name = &result.wing;
            let room_name = &result.room;
            let source = std::path::Path::new(&result.source_file)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let snippet = result
                .text
                .trim()
                .replace('\n', " ")
                .chars()
                .take(300)
                .collect::<String>();

            let snippet = if result.text.len() > 300 {
                format!("{}...", snippet)
            } else {
                snippet
            };

            lines.push(format!(
                "  [{}] {}/{} (sim={:.3})",
                i + 1,
                wing_name,
                room_name,
                similarity
            ));
            lines.push(format!("      {}", snippet));
            if !source.is_empty() {
                lines.push(format!("      src: {}", source));
            }
        }

        lines.join("\n")
    }

    /// Return raw search results
    pub fn search_raw(
        &self,
        storage: &ChromaStorage,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> Vec<SearchHit> {
        storage.search(query, wing, room, n_results)
    }
}

/// Search hit with metadata
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub text: String,
    pub wing: String,
    pub room: String,
    pub source_file: String,
    pub similarity: f64,
    pub distance: Option<f64>,
}

impl SearchHit {
    pub fn from_drawer(drawer: &Drawer, distance: Option<f64>) -> Self {
        let similarity = distance.map(|d| (1.0 - d).round() / 1000.0).unwrap_or(1.0);
        Self {
            text: drawer.document.clone(),
            wing: drawer.metadata.wing.clone(),
            room: drawer.metadata.room.clone(),
            source_file: drawer.metadata.source_file.clone(),
            similarity,
            distance,
        }
    }
}

// ============================================================================
// MemoryStack — unified interface
// ============================================================================

/// The full 4-layer memory stack
#[derive(Debug, Clone)]
pub struct MemoryStack {
    pub palace_path: PathBuf,
    pub identity_path: PathBuf,
    pub l0: Layer0,
    pub l1: Layer1,
    pub l2: Layer2,
    pub l3: Layer3,
}

impl MemoryStack {
    /// Create a new memory stack
    pub fn new(palace_path: Option<PathBuf>, identity_path: Option<PathBuf>) -> Self {
        let palace_path = palace_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            PathBuf::from(format!("{}/.mempalace/palace", home))
        });

        let identity_path = identity_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            PathBuf::from(format!("{}/.mempalace/identity.txt", home))
        });

        Self {
            palace_path: palace_path.clone(),
            identity_path: identity_path.clone(),
            l0: Layer0::new(Some(identity_path)),
            l1: Layer1::new(Some(palace_path.clone()), None),
            l2: Layer2::new(Some(palace_path.clone())),
            l3: Layer3::new(Some(palace_path)),
        }
    }

    /// Generate wake-up text: L0 (identity) + L1 (essential story)
    pub fn wake_up(&mut self, storage: &ChromaStorage, wing: Option<&str>) -> String {
        let mut parts = Vec::new();

        // L0: Identity
        parts.push(self.l0.render());
        parts.push(String::new());

        // L1: Essential Story
        let l1 = if let Some(w) = wing {
            self.l1.clone().with_wing(w.to_string())
        } else {
            self.l1.clone()
        };
        parts.push(l1.generate(storage));

        parts.join("\n")
    }

    /// On-demand L2 retrieval filtered by wing/room
    pub fn recall(
        &self,
        storage: &ChromaStorage,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> String {
        self.l2.retrieve(storage, wing, room, n_results)
    }

    /// Deep L3 semantic search
    pub fn search(
        &self,
        storage: &ChromaStorage,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> String {
        self.l3.search(storage, query, wing, room, n_results)
    }

    /// Status of all layers
    pub fn status(&mut self, storage: &ChromaStorage) -> MemoryStackStatus {
        MemoryStackStatus {
            palace_path: self.palace_path.clone(),
            identity_path: self.identity_path.clone(),
            identity_exists: self.l0.exists(),
            identity_tokens: self.l0.token_estimate(),
            total_drawers: storage.count().unwrap_or(0),
        }
    }
}

/// Memory stack status
#[derive(Debug, Clone)]
pub struct MemoryStackStatus {
    pub palace_path: PathBuf,
    pub identity_path: PathBuf,
    pub identity_exists: bool,
    pub identity_tokens: usize,
    pub total_drawers: usize,
}

// ============================================================================
// Extended DrawerMetadata for importance weights
// ============================================================================

impl DrawerMetadata {
    /// Get emotional weight (for L1 scoring)
    pub fn emotional_weight(&self) -> Option<f64> {
        // Try to get from custom fields if available
        None
    }

    /// Get importance score (for L1 scoring)
    pub fn importance(&self) -> Option<f64> {
        // Try to get from custom fields if available
        None
    }
}

// ============================================================================
// Tests
// ============================================================================

/// Safely truncate a string to at most `max_chars` characters, respecting UTF-8 boundaries
pub fn truncate_safe(s: &str, max_chars: usize) -> &str {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        return s;
    }
    // nth(max_chars) gives the character at position max_chars (the first NOT included)
    // So we take from 0 to that position
    let byte_end = s
        .char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    &s[..byte_end]
}

#[cfg(test)]
#[path = "./tests/layers.rs"]
mod tests;
