use crate::benchmark::{
    evaluate_case, evaluate_stages, recall_at_k, BenchmarkAggregates, BenchmarkCase,
    BenchmarkComparison, CaseCategory, SeededDoc,
};
use crate::layers::SearchHit;

fn make_hit(id: &str, text: &str) -> SearchHit {
    SearchHit {
        document_id: Some(id.to_string()),
        text: text.to_string(),
        wing: "wing_x".to_string(),
        room: "room_y".to_string(),
        source_file: "fixtures/test.json".to_string(),
        similarity: 0.9,
        distance: Some(0.1),
    }
}

#[test]
fn test_recall_at_k_counts_matching_ids() {
    let results = vec![
        make_hit("event_123", "user bought laptop"),
        make_hit("event_999", "user bought monitor"),
    ];

    let expected = vec!["event_123".to_string()];
    assert_eq!(recall_at_k(&results, &expected, 1), 1.0);
    assert_eq!(recall_at_k(&results, &expected, 2), 1.0);
}

#[test]
fn test_recall_at_k_partial_match() {
    let results = vec![make_hit("a", "doc a"), make_hit("b", "doc b")];
    let expected = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    assert_eq!(recall_at_k(&results, &expected, 2), 2.0 / 3.0);
}

#[test]
fn test_evaluate_case_reports_matches_and_quality_metrics() {
    let case = BenchmarkCase {
        id: "purchase_lookup".to_string(),
        query: "user bought laptop".to_string(),
        expected_ids: vec!["event_123".to_string()],
        wing: Some("wing_code".to_string()),
        room: Some("purchases".to_string()),
        category: CaseCategory::Smoke,
        docs: vec![],
    };

    let results = vec![
        make_hit("event_123", "user bought laptop"),
        make_hit("event_999", "user bought monitor"),
    ];

    let result = evaluate_case(&case, &results, 2);
    assert_eq!(result.case_id, "purchase_lookup");
    assert_eq!(result.recall_at_k, 1.0);
    assert!(result.hit_at_k);
    assert!(result.top1_hit);
    assert_eq!(result.mrr_at_k, 1.0);
    assert_eq!(result.matched_ids, vec!["event_123".to_string()]);
    assert!(result.missed_ids.is_empty());
    assert_eq!(result.expected_ids, vec!["event_123".to_string()]);
}

#[test]
fn test_evaluate_case_reports_missed_ids() {
    let case = BenchmarkCase {
        id: "miss_case".to_string(),
        query: "nothing found".to_string(),
        expected_ids: vec!["expected_a".to_string(), "expected_b".to_string()],
        wing: Some("wing_x".to_string()),
        room: Some("room_y".to_string()),
        category: CaseCategory::Adversarial,
        docs: vec![],
    };

    let results = vec![make_hit("wrong_result", "unrelated document")];

    let result = evaluate_case(&case, &results, 1);
    assert_eq!(result.recall_at_k, 0.0);
    assert!(!result.hit_at_k);
    assert!(!result.top1_hit);
    assert_eq!(result.mrr_at_k, 0.0);
    assert!(result.matched_ids.is_empty());
    assert_eq!(result.missed_ids.len(), 2);
    assert_eq!(
        result.expected_ids,
        vec!["expected_a".to_string(), "expected_b".to_string()]
    );
}

#[test]
fn test_case_category_defaults_to_smoke() {
    let json = r#"{"id":"t","query":"q","expected_ids":[]}"#;
    let case: BenchmarkCase = serde_json::from_str(json).expect("must deserialize");
    assert_eq!(case.category, CaseCategory::Smoke);
}

#[test]
fn test_case_category_deserializes_adversarial() {
    let json = r#"{"id":"t","query":"q","expected_ids":[],"category":"adversarial"}"#;
    let case: BenchmarkCase = serde_json::from_str(json).expect("must deserialize");
    assert_eq!(case.category, CaseCategory::Adversarial);
}

#[test]
fn test_case_category_serializes_back() {
    let case = BenchmarkCase {
        id: "t".to_string(),
        query: "q".to_string(),
        expected_ids: vec![],
        wing: None,
        room: None,
        category: CaseCategory::Adversarial,
        docs: vec![],
    };
    let json = serde_json::to_string(&case).expect("must serialize");
    assert!(
        json.contains(r#""category":"adversarial""#)
            || json.contains(r#""category": "adversarial"#),
        "Unexpected JSON: {json}"
    );
}

#[test]
fn test_baseline_smoke_category() {
    assert_eq!(CaseCategory::Baseline, CaseCategory::Baseline);
}

#[test]
fn test_result_carries_expected_ids_for_miss_diagnostics() {
    let case = BenchmarkCase {
        id: "diag".to_string(),
        query: "diag query".to_string(),
        expected_ids: vec![
            "exp_1".to_string(),
            "exp_2".to_string(),
            "exp_3".to_string(),
        ],
        wing: None,
        room: None,
        category: CaseCategory::Adversarial,
        docs: vec![],
    };

    let results = vec![make_hit("exp_1", "partial hit"), make_hit("wrong", "wrong")];

    let result = evaluate_case(&case, &results, 2);
    assert_eq!(result.matched_ids.len(), 1);
    assert_eq!(result.missed_ids.len(), 2);
    assert!(result.missed_ids.iter().any(|id| id == "exp_2"));
    assert!(result.missed_ids.iter().any(|id| id == "exp_3"));
    assert_eq!(
        result.expected_ids.len(),
        3,
        "expected_ids must be carried to result"
    );
}

fn make_result(
    case_id: &str,
    recall: f32,
    hit: bool,
    top1: bool,
    mrr: f32,
) -> crate::benchmark::BenchmarkResult {
    crate::benchmark::BenchmarkResult {
        case_id: case_id.to_string(),
        query: "test".to_string(),
        recall_at_k: recall,
        hit_at_k: hit,
        top1_hit: top1,
        mrr_at_k: mrr,
        matched_ids: vec![],
        missed_ids: vec![],
        top_result_ids: vec![],
        expected_ids: vec![],
    }
}

fn make_aggregate(
    cases: usize,
    avg_recall: f32,
    avg_hit: f32,
    avg_top1: f32,
    avg_mrr: f32,
) -> BenchmarkAggregates {
    BenchmarkAggregates {
        cases,
        avg_recall,
        avg_hit,
        avg_top1,
        avg_mrr,
    }
}

#[test]
fn test_benchmark_aggregates_compute_correctly() {
    let result1 = make_result("t1", 1.0, true, true, 1.0);
    let result2 = make_result("t2", 0.0, false, false, 0.0);
    let avg = BenchmarkAggregates::compute(&[result1, result2]);
    let expected = make_aggregate(2, 0.5, 0.5, 0.5, 0.5);
    assert_eq!(avg.cases, expected.cases);
    assert!((avg.avg_recall - expected.avg_recall).abs() < 1e-6);
    assert!((avg.avg_hit - expected.avg_hit).abs() < 1e-6);
    assert!((avg.avg_top1 - expected.avg_top1).abs() < 1e-6);
    assert!((avg.avg_mrr - expected.avg_mrr).abs() < 1e-6);
}

#[test]
fn test_benchmark_aggregates_empty_input() {
    let avg = BenchmarkAggregates::compute(&[]);
    assert_eq!(avg.cases, 0);
    assert_eq!(avg.avg_recall, 0.0);
}

#[test]
fn test_benchmark_comparison_builds_correctly() {
    fn make_result(
        case_id: &str,
        recall: f32,
        hit: bool,
        top1: bool,
        mrr: f32,
    ) -> crate::benchmark::BenchmarkResult {
        crate::benchmark::BenchmarkResult {
            case_id: case_id.to_string(),
            query: "test".to_string(),
            recall_at_k: recall,
            hit_at_k: hit,
            top1_hit: top1,
            mrr_at_k: mrr,
            matched_ids: vec![],
            missed_ids: vec![],
            top_result_ids: vec![],
            expected_ids: vec![],
        }
    }

    let mp_results = vec![
        make_result("test_1", 1.0, true, true, 1.0),
        make_result("test_2", 0.5, false, false, 0.5),
    ];
    let bl_results = vec![
        make_result("test_1", 0.8, true, true, 0.8),
        make_result("test_2", 0.3, false, false, 0.3),
    ];

    let comparison = BenchmarkComparison::build(&mp_results, &bl_results);
    assert_eq!(comparison.mp_summary.cases, 2);
    assert_eq!(comparison.bl_summary.cases, 2);
    assert_eq!(comparison.per_case.len(), 2);

    // Mp should be better than baseline
    assert!(comparison.mp_summary.avg_recall > comparison.bl_summary.avg_recall);
    // Hits are equal in this test data (both true or both false for same cases)
    assert!((comparison.mp_summary.avg_hit - comparison.bl_summary.avg_hit).abs() < 1e-6);
    assert!(comparison.mp_summary.avg_mrr > comparison.bl_summary.avg_mrr);
}

#[test]
fn test_benchmark_comparison_handles_empty_baseline() {
    fn make_result(case_id: &str) -> crate::benchmark::BenchmarkResult {
        crate::benchmark::BenchmarkResult {
            case_id: case_id.to_string(),
            query: "test".to_string(),
            recall_at_k: 0.5,
            hit_at_k: true,
            top1_hit: false,
            mrr_at_k: 0.5,
            matched_ids: vec![],
            missed_ids: vec![],
            top_result_ids: vec![],
            expected_ids: vec![],
        }
    }

    let results = vec![make_result("test_1")];
    let baseline_results = vec![];

    let comparison = BenchmarkComparison::build(&results, &baseline_results);
    assert_eq!(comparison.mp_summary.cases, 1);
    assert_eq!(comparison.bl_summary.cases, 0);
    assert!(
        comparison.per_case.is_empty(),
        "should have no deltas with empty baseline"
    );
}

// ---------------------------------------------------------------------------
// New quality hardening tests
// ---------------------------------------------------------------------------

#[test]
fn test_seeded_doc_deserializes() {
    let json = r#"{"id":"doc_1","content":"some text"}"#;
    let doc: SeededDoc = serde_json::from_str(json).expect("must deserialize");
    assert_eq!(doc.id, "doc_1");
    assert_eq!(doc.content, "some text");
}

#[test]
fn test_seeded_doc_serializes_back() {
    let doc = SeededDoc {
        id: "d".to_string(),
        content: "hello world".to_string(),
    };
    let json = serde_json::to_string(&doc).expect("must serialize");
    assert!(json.contains(r#""id":"d""#) || json.contains(r#""id": "d""#));
    assert!(json.contains("hello world"));
}

#[test]
fn test_case_with_docs_deserializes() {
    let json = r#"{
        "id":"hard_test",
        "query":"user current job",
        "expected_ids":["event_recent"],
        "category":"adversarial",
        "docs":[
            {"id":"event_recent","content":"User works at BigCo since 2024."},
            {"id":"event_old","content":"User worked at SmallCo in 2020."}
        ]
    }"#;
    let case: BenchmarkCase = serde_json::from_str(json).expect("must deserialize");
    assert_eq!(case.id, "hard_test");
    assert_eq!(case.docs.len(), 2);
    assert_eq!(case.docs[0].id, "event_recent");
    assert_eq!(case.category, CaseCategory::Adversarial);
}

#[test]
fn test_case_docs_defaults_to_empty() {
    let json = r#"{"id":"t","query":"q","expected_ids":[]}"#;
    let case: BenchmarkCase = serde_json::from_str(json).expect("must deserialize");
    assert!(case.docs.is_empty());
}

#[test]
fn test_evaluate_stages_reports_retrieval_and_ranking_separately() {
    // Case expects two IDs; retrieval finds one at position 2, not at top.
    let case = BenchmarkCase {
        id: "stage_test".to_string(),
        query: "test query".to_string(),
        expected_ids: vec!["expected_a".to_string(), "expected_b".to_string()],
        wing: None,
        room: None,
        category: CaseCategory::Adversarial,
        docs: vec![
            SeededDoc {
                id: "expected_a".to_string(),
                content: "doc a".to_string(),
            },
            SeededDoc {
                id: "expected_b".to_string(),
                content: "doc b".to_string(),
            },
        ],
    };

    // Hits: wrong doc first, expected_a at position 2, expected_b not in results.
    let hits = vec![
        make_hit("wrong", "wrong document"),
        make_hit("expected_a", "found it"),
    ];

    let stages = evaluate_stages(&case, &hits, 5);

    // Retrieval found expected_a out of 2 expected, so recall = 0.5
    assert_eq!(stages.retrieval.total_candidates, 2);
    assert!((stages.retrieval.recall_at_k - 0.5).abs() < 1e-6);
    assert!(
        stages.retrieval.hit_at_k,
        "at least one expected id was retrieved"
    );
    assert!((stages.retrieval.mrr_at_k - 0.5).abs() < 1e-6); // 1/position(2) = 0.5

    // Ranking: top-1 is wrong
    assert!(!stages.ranking.top1_hit);
    assert_eq!(stages.ranking.top_id, Some("wrong".to_string()));
    assert!((stages.ranking.mrr_at_k - 0.5).abs() < 1e-6);
}

#[test]
fn test_evaluate_stages_full_hit() {
    let case = BenchmarkCase {
        id: "perfect".to_string(),
        query: "test".to_string(),
        expected_ids: vec!["doc_a".to_string()],
        wing: None,
        room: None,
        category: CaseCategory::Smoke,
        docs: vec![SeededDoc {
            id: "doc_a".to_string(),
            content: "the answer".to_string(),
        }],
    };
    let hits = vec![
        make_hit("doc_a", "the answer"),
        make_hit("doc_b", "something else"),
    ];

    let stages = evaluate_stages(&case, &hits, 5);
    assert_eq!(stages.retrieval.recall_at_k, 1.0);
    assert!(stages.retrieval.hit_at_k);
    assert!(stages.ranking.top1_hit);
    assert_eq!(stages.ranking.top_id, Some("doc_a".to_string()));
}

#[test]
fn test_evaluate_stages_retrieval_found_not_ranked() {
    // Simulates: BM25 found the right doc but it wasn't ranked #1.
    let case = BenchmarkCase {
        id: "retrieve_but_miss_rank".to_string(),
        query: "find the thing".to_string(),
        expected_ids: vec!["expected_thing".to_string()],
        wing: None,
        room: None,
        category: CaseCategory::Adversarial,
        docs: vec![SeededDoc {
            id: "expected_thing".to_string(),
            content: "the thing".to_string(),
        }],
    };

    // Retrieval hit it at position 3, but top-1 is a distractor.
    let hits = vec![
        make_hit("distractor_1", "similar sounding"),
        make_hit("distractor_2", "also relevant"),
        make_hit("expected_thing", "the actual answer"),
    ];

    let stages = evaluate_stages(&case, &hits, 5);

    assert!(
        stages.retrieval.hit_at_k,
        "expected doc exists in candidates"
    );
    assert!(!stages.ranking.top1_hit, "top-1 is NOT the expected doc");
    // MRR should be 1/3 ≈ 0.333
    assert!((stages.retrieval.mrr_at_k - 1.0 / 3.0).abs() < 1e-4);
}
