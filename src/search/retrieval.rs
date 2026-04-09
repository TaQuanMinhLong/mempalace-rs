//! Retrieval - layer-based retrieval (L0-L3)
//!
//! Port from Python retrieval logic. Provides context retrieval from the
//! 4-layer memory stack:
//!   L0: Identity (~100 tokens, always loaded)
//!   L1: Essential story (~500-800 tokens, always loaded)
//!   L2: Wing/room filtered (~200-500 tokens, on demand)
//!   L3: Full semantic search (unlimited, on demand)

use crate::error::{MempalaceError, Result};
use crate::layers::{MemoryLayer, MemoryStack, SearchHit};
use crate::storage::ChromaStorage;

/// Retrieval mode used for a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    WakeUpOnly,
    LayeredSearch,
}

/// Query-aware retrieval options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrieveOptions {
    pub query: Option<String>,
    pub wing: Option<String>,
    pub room: Option<String>,
    pub limit: usize,
}

impl RetrieveOptions {
    #[must_use]
    pub fn new(query: Option<&str>) -> Self {
        Self {
            query: query.map(str::to_string),
            wing: None,
            room: None,
            limit: 5,
        }
    }

    #[must_use]
    pub fn with_wing(mut self, wing: Option<&str>) -> Self {
        self.wing = wing.map(str::to_string);
        self
    }

    #[must_use]
    pub fn with_room(mut self, room: Option<&str>) -> Self {
        self.room = room.map(str::to_string);
        self
    }

    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }
}

/// Explanation for how a query was processed.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalExplanation {
    pub steps: Vec<String>,
    pub filters_applied: Vec<String>,
    pub ranking_reason: String,
}

/// Internal retrieval plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalPlan {
    pub mode: RetrievalMode,
    pub query: Option<String>,
    pub wing: Option<String>,
    pub room: Option<String>,
    pub limit: usize,
}

/// Structured retrieval result.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalResult {
    pub mode: RetrievalMode,
    pub context: String,
    pub hits: Vec<SearchHit>,
    pub explanation: RetrievalExplanation,
}

/// Retriever for memory layers.
#[derive(Debug, Clone)]
pub struct Retriever;

impl Retriever {
    /// Create a new retriever.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Build a retrieval plan from user options.
    #[must_use]
    pub fn plan(&self, options: &RetrieveOptions) -> RetrievalPlan {
        let query = options
            .query
            .as_ref()
            .map(|q| q.trim())
            .filter(|q| !q.is_empty())
            .map(str::to_string);

        let limit = options.limit.max(1);

        if query.is_some() {
            RetrievalPlan {
                mode: RetrievalMode::LayeredSearch,
                query,
                wing: options.wing.clone(),
                room: options.room.clone(),
                limit,
            }
        } else {
            RetrievalPlan {
                mode: RetrievalMode::WakeUpOnly,
                query: None,
                wing: options.wing.clone(),
                room: options.room.clone(),
                limit,
            }
        }
    }

    /// Retrieve context from memory stack using wake-up + search.
    pub fn retrieve(
        &self,
        stack: &MemoryStack,
        storage: &ChromaStorage,
        query: Option<&str>,
    ) -> Result<String> {
        let options = RetrieveOptions::new(query).with_limit(5);
        self.retrieve_with_options(stack, storage, &options)
            .map(|result| result.context)
    }

    /// Query-aware retrieval with explicit pipeline output.
    pub fn retrieve_with_options(
        &self,
        stack: &MemoryStack,
        storage: &ChromaStorage,
        options: &RetrieveOptions,
    ) -> Result<RetrievalResult> {
        let plan = self.plan(options);
        match plan.mode {
            RetrievalMode::WakeUpOnly => Ok(self.retrieve_wake_up(stack, storage, &plan)),
            RetrievalMode::LayeredSearch => self.retrieve_query(stack, storage, &plan),
        }
    }

    fn retrieve_wake_up(
        &self,
        stack: &MemoryStack,
        storage: &ChromaStorage,
        plan: &RetrievalPlan,
    ) -> RetrievalResult {
        let mut stack_mut = stack.clone();
        let context = stack_mut.wake_up(storage, plan.wing.as_deref());
        RetrievalResult {
            mode: RetrievalMode::WakeUpOnly,
            context,
            hits: Vec::new(),
            explanation: RetrievalExplanation {
                steps: vec![
                    "retrieve: load wake-up context".to_string(),
                    "filter: wing applied only to L1 when provided".to_string(),
                    "rank: no query supplied, skip search ranking".to_string(),
                    "explain: return L0 + L1 context".to_string(),
                ],
                filters_applied: plan
                    .wing
                    .as_ref()
                    .map(|wing| vec![format!("wing={wing}")])
                    .unwrap_or_default(),
                ranking_reason:
                    "wake-up mode uses identity and essential story without query ranking"
                        .to_string(),
            },
        }
    }

    fn retrieve_query(
        &self,
        stack: &MemoryStack,
        storage: &ChromaStorage,
        plan: &RetrievalPlan,
    ) -> Result<RetrievalResult> {
        let query = plan
            .query
            .as_deref()
            .ok_or_else(|| MempalaceError::Search("missing retrieval query".to_string()))?;

        let hits = self.retrieve_hits(
            storage,
            query,
            plan.wing.as_deref(),
            plan.room.as_deref(),
            plan.limit,
        );
        let context = self.render_context(stack, storage, query, &hits, plan);

        Ok(RetrievalResult {
            mode: RetrievalMode::LayeredSearch,
            context,
            hits,
            explanation: RetrievalExplanation {
                steps: vec![
                    "retrieve: query SQLite FTS index".to_string(),
                    "filter: apply optional wing/room filters".to_string(),
                    "rank: order by BM25-backed search score".to_string(),
                    "explain: render wake-up context plus ranked hits".to_string(),
                ],
                filters_applied: filters_for(plan),
                ranking_reason: "ranked by query-conditioned SQLite FTS/BM25 results".to_string(),
            },
        })
    }

    fn retrieve_hits(
        &self,
        storage: &ChromaStorage,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Vec<SearchHit> {
        storage.search(query, wing, room, limit.max(1))
    }

    fn render_context(
        &self,
        stack: &MemoryStack,
        storage: &ChromaStorage,
        query: &str,
        hits: &[SearchHit],
        plan: &RetrievalPlan,
    ) -> String {
        let mut stack_mut = stack.clone();
        let wake_up = stack_mut.wake_up(storage, plan.wing.as_deref());
        let mut lines = vec![
            wake_up,
            String::new(),
            format!("## RETRIEVAL — QUERY: {query}"),
        ];

        if hits.is_empty() {
            lines.push("No ranked hits found.".to_string());
            return lines.join("\n");
        }

        for (index, hit) in hits.iter().enumerate() {
            let document_id = hit.document_id.as_deref().unwrap_or("unknown");
            lines.push(format!(
                "{}. [{}] {}/{} id={} score={:.3}",
                index + 1,
                hit.source_file,
                hit.wing,
                hit.room,
                document_id,
                hit.similarity
            ));
            lines.push(format!("   {}", compact_preview(&hit.text, 220)));
        }

        lines.join("\n")
    }

    /// Retrieve from a specific layer only.
    pub fn retrieve_layer(
        &self,
        stack: &MemoryStack,
        storage: &ChromaStorage,
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
                // L3 is semantic search - handled separately via SemanticSearcher.
                Ok(String::new())
            }
        }
    }

    /// Get a summary of what's in each layer.
    #[must_use]
    pub fn layer_summary(&self, stack: &MemoryStack, storage: &ChromaStorage) -> LayerSummary {
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
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

fn filters_for(plan: &RetrievalPlan) -> Vec<String> {
    [
        plan.wing.as_ref().map(|wing| format!("wing={wing}")),
        plan.room.as_ref().map(|room| format!("room={room}")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn compact_preview(text: &str, max_chars: usize) -> String {
    let preview = text.trim().replace('\n', " ");
    let char_count = preview.chars().count();
    if char_count <= max_chars {
        return preview;
    }

    let end = preview
        .char_indices()
        .nth(max_chars)
        .map(|(idx, _)| idx)
        .unwrap_or(preview.len());
    format!("{}...", &preview[..end])
}

/// Summary of memory layers.
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
