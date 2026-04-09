//! Deterministic retrieval benchmark primitives.
//!
//! This module provides the quality-evaluation path for retrieval benchmarks.
//! It computes correctness metrics like recall, hit-rate, top-1 accuracy, and
//! mean reciprocal rank. It does NOT measure latency; that belongs in Criterion
//! benches under `benches/retrieval_benchmark.rs`.

use crate::error::Result;
use crate::layers::SearchHit;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Case category for grouped reporting in the evaluation output.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseCategory {
    #[default]
    Smoke,
    Adversarial,
    Baseline,
}

/// Bench-seeded document: an ID plus the exact text to store.
///
/// When present in a fixture, each entry is seeded verbatim instead of
/// synthesising a drawer body from the case query. This makes the corpus
/// adversarial: expected docs can differ semantically from the query, and
/// distractors can sit nearby in BM25 space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeededDoc {
    pub id: String,
    pub content: String,
}

/// Fixed retrieval benchmark case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkCase {
    pub id: String,
    pub query: String,
    #[serde(default)]
    pub expected_ids: Vec<String>,
    #[serde(default)]
    pub wing: Option<String>,
    #[serde(default)]
    pub room: Option<String>,
    /// Optional category for grouped reporting. Defaults to `Smoke` when omitted.
    #[serde(default)]
    pub category: CaseCategory,
    /// Exact document content to seed for each expected/benchmark doc.
    ///
    /// When populated the CLI uses these strings verbatim instead of
    /// synthesising text from the query. This removes the trivial
    /// query-in-document shortcut that previously made both MemPalace
    /// and baseline look perfect.
    #[serde(default)]
    pub docs: Vec<SeededDoc>,
}

/// Retrieval benchmark result enriched with quality metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub case_id: String,
    pub query: String,
    pub recall_at_k: f32,
    pub hit_at_k: bool,
    pub top1_hit: bool,
    pub mrr_at_k: f32,
    pub matched_ids: Vec<String>,
    pub missed_ids: Vec<String>,
    pub top_result_ids: Vec<String>,
    /// Expected IDs from the fixture (carried to the result for MISS diagnostics).
    pub expected_ids: Vec<String>,
}

fn collect_doc_ids_up_to(results: &[SearchHit], k: usize) -> Vec<&str> {
    results
        .iter()
        .take(k)
        .filter_map(|hit| hit.document_id.as_deref())
        .collect()
}

#[must_use]
pub fn recall_at_k(results: &[SearchHit], expected_ids: &[String], k: usize) -> f32 {
    if expected_ids.is_empty() {
        return 0.0;
    }

    let top_result_ids = collect_doc_ids_up_to(results, k);
    let matched = expected_ids
        .iter()
        .filter(|expected| top_result_ids.iter().any(|id| id == &expected.as_str()))
        .count();

    matched as f32 / expected_ids.len() as f32
}

fn hit_at_k(results: &[SearchHit], expected_ids: &[String], k: usize) -> bool {
    if expected_ids.is_empty() {
        return false;
    }

    let top_result_ids = collect_doc_ids_up_to(results, k);
    expected_ids
        .iter()
        .any(|expected| top_result_ids.iter().any(|id| id == &expected.as_str()))
}

fn top1_hit(results: &[SearchHit], expected_ids: &[String]) -> bool {
    if expected_ids.is_empty() {
        return false;
    }

    results
        .first()
        .and_then(|hit| hit.document_id.as_deref())
        .is_some_and(|id| expected_ids.iter().any(|expected| expected.as_str() == id))
}

fn mrr_at_k(results: &[SearchHit], expected_ids: &[String], k: usize) -> f32 {
    if expected_ids.is_empty() {
        return 0.0;
    }

    let expected_set: HashSet<&str> = expected_ids.iter().map(|s| s.as_str()).collect();

    for (i, hit) in results.iter().take(k).enumerate() {
        if let Some(doc_id) = hit.document_id.as_deref() {
            if expected_set.contains(doc_id) {
                return 1.0 / (i + 1) as f32;
            }
        }
    }

    0.0
}

#[must_use]
pub fn evaluate_case(case: &BenchmarkCase, results: &[SearchHit], k: usize) -> BenchmarkResult {
    let top_result_ids: Vec<String> = results
        .iter()
        .take(k)
        .filter_map(|hit| hit.document_id.clone())
        .collect();

    let matched_ids: Vec<String> = case
        .expected_ids
        .iter()
        .filter(|expected| top_result_ids.iter().any(|id| id == *expected))
        .cloned()
        .collect();

    let missed_ids: Vec<String> = case
        .expected_ids
        .iter()
        .filter(|expected| !matched_ids.iter().any(|m| m == *expected))
        .cloned()
        .collect();

    BenchmarkResult {
        case_id: case.id.clone(),
        query: case.query.clone(),
        recall_at_k: recall_at_k(results, &case.expected_ids, k),
        hit_at_k: hit_at_k(results, &case.expected_ids, k),
        top1_hit: top1_hit(results, &case.expected_ids),
        mrr_at_k: mrr_at_k(results, &case.expected_ids, k),
        matched_ids,
        missed_ids,
        top_result_ids,
        expected_ids: case.expected_ids.clone(),
    }
}

pub fn load_cases(path: &Path) -> Result<Vec<BenchmarkCase>> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

// ---------------------------------------------------------------------------
// Structured comparison for machine-readable artifacts
// ---------------------------------------------------------------------------

/// Aggregate metrics for a set of benchmark results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkAggregates {
    pub cases: usize,
    pub avg_recall: f32,
    pub avg_hit: f32,
    pub avg_top1: f32,
    pub avg_mrr: f32,
}

impl BenchmarkAggregates {
    pub fn compute(results: &[BenchmarkResult]) -> Self {
        let n = results.len();
        if n == 0 {
            return Self {
                cases: 0,
                avg_recall: 0.0,
                avg_hit: 0.0,
                avg_top1: 0.0,
                avg_mrr: 0.0,
            };
        }
        let n = n as f32;
        Self {
            cases: results.len(),
            avg_recall: results.iter().map(|r| r.recall_at_k).sum::<f32>() / n,
            avg_hit: results.iter().filter(|r| r.hit_at_k).count() as f32 / n,
            avg_top1: results.iter().filter(|r| r.top1_hit).count() as f32 / n,
            avg_mrr: results.iter().map(|r| r.mrr_at_k).sum::<f32>() / n,
        }
    }
}

/// Per-case delta between MemPalace and baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerCaseDelta {
    pub case_id: String,
    pub mp_recall: f32,
    pub bl_recall: f32,
    pub mp_hit: bool,
    pub bl_hit: bool,
    pub mp_mrr: f32,
    pub bl_mrr: f32,
}

/// Full structured comparison between MemPalace and a baseline path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub mp_summary: BenchmarkAggregates,
    pub bl_summary: BenchmarkAggregates,
    pub per_case: Vec<PerCaseDelta>,
}

impl BenchmarkComparison {
    pub fn build(results: &[BenchmarkResult], baseline_results: &[BenchmarkResult]) -> Self {
        let mp_summary = BenchmarkAggregates::compute(results);
        let bl_summary = BenchmarkAggregates::compute(baseline_results);

        // If we cannot pair results, return an empty comparison.
        if results.is_empty() || baseline_results.is_empty() {
            return Self {
                mp_summary,
                bl_summary,
                per_case: Vec::new(),
            };
        }

        let paired_len = results.len().min(baseline_results.len());

        let per_case = results
            .iter()
            .zip(baseline_results.iter())
            .take(paired_len)
            .map(|(mp, bl)| PerCaseDelta {
                case_id: mp.case_id.clone(),
                mp_recall: mp.recall_at_k,
                bl_recall: bl.recall_at_k,
                mp_hit: mp.hit_at_k,
                bl_hit: bl.hit_at_k,
                mp_mrr: mp.mrr_at_k,
                bl_mrr: bl.mrr_at_k,
            })
            .collect();

        Self {
            mp_summary,
            bl_summary,
            per_case,
        }
    }
}

/// Candidate retrieval metrics measured *before* any relevance reranking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateRetrievalMetrics {
    pub case_id: String,
    pub total_candidates: usize,
    pub recall_at_k: f32,
    pub hit_at_k: bool,
    pub mrr_at_k: f32,
}

/// Top-k ranking metrics measured *after* reranking the candidate set.
///
/// These share the same candidate pool as `CandidateRetrievalMetrics` but
/// evaluate whether the expected documents end up in the *right order*.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankingMetrics {
    pub case_id: String,
    pub top_id: Option<String>,
    pub top1_hit: bool,
    pub mrr_at_k: f32,
}

/// Combined stage-level result for a single benchmark case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageResult {
    pub retrieval: CandidateRetrievalMetrics,
    pub ranking: RankingMetrics,
}

/// Evaluate a single stage-level case: retrieves candidates, then scores
/// ranking quality separately so we know whether failures come from recall
/// or from ranking order.
#[must_use]
pub fn evaluate_stages(case: &BenchmarkCase, hits: &[SearchHit], k: usize) -> StageResult {
    // Candidate retrieval metrics (same hits list, measuring recall into k).
    let retrieval = CandidateRetrievalMetrics {
        case_id: case.id.clone(),
        total_candidates: hits.len(),
        recall_at_k: recall_at_k(hits, &case.expected_ids, k),
        hit_at_k: hit_at_k(hits, &case.expected_ids, k),
        mrr_at_k: mrr_at_k(hits, &case.expected_ids, k),
    };

    // Ranking metrics (is top-1 correct? how early does an expected doc appear?).
    let top_id = hits.first().and_then(|h| h.document_id.clone());
    let ranking = RankingMetrics {
        case_id: case.id.clone(),
        top_id,
        top1_hit: top1_hit(hits, &case.expected_ids),
        mrr_at_k: mrr_at_k(hits, &case.expected_ids, k),
    };

    StageResult { retrieval, ranking }
}

#[cfg(test)]
#[cfg(feature = "bench")]
#[path = "./tests/benchmark.rs"]
mod tests;
