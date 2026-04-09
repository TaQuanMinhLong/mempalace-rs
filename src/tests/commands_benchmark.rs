#![cfg(feature = "bench")]

use crate::benchmark::BenchmarkCase;

#[test]
fn test_benchmark_fixture_shape_deserializes() {
    let raw = r#"[
      {
        "id": "purchase_lookup",
        "query": "user bought laptop",
        "expected_ids": ["event_1"],
        "wing": "wing_benchmark",
        "room": "purchases"
      }
    ]"#;

    let cases: Vec<BenchmarkCase> = serde_json::from_str(raw).unwrap();
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].expected_ids, vec!["event_1".to_string()]);
    assert_eq!(cases[0].category, crate::benchmark::CaseCategory::Smoke);
}

#[test]
fn test_benchmark_fixture_adversarial_category_deserializes() {
    let raw = r#"[
      {
        "id": "temporal_conflict",
        "query": "user current job",
        "expected_ids": ["event_recent_job"],
        "wing": "wing_benchmark",
        "room": "careers",
        "category": "adversarial"
      }
    ]"#;

    let cases: Vec<BenchmarkCase> = serde_json::from_str(raw).unwrap();
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].id, "temporal_conflict");
    assert_eq!(
        cases[0].category,
        crate::benchmark::CaseCategory::Adversarial
    );
}
