use crate::benchmark::{
    evaluate_case, evaluate_stages, load_cases, BenchmarkCase, BenchmarkComparison,
    BenchmarkResult, CaseCategory, StageResult,
};
use crate::commands::load_config;
use crate::error::Result;
use crate::palace::{Drawer, DrawerMetadata, IngestMode};
use crate::storage::ChromaStorage;
use std::fs;
use std::path::Path;

const DEFAULT_BENCHMARK_FIXTURE: &str = "fixtures/benchmark_cases.json";

// ---------------------------------------------------------------------------
// Baseline evaluation: plain BM25 (no room filtering, no MemPalace structure)
// ---------------------------------------------------------------------------

fn evaluate_baseline(
    storage: &ChromaStorage,
    cases: &[BenchmarkCase],
    k: usize,
) -> Vec<BenchmarkResult> {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        // Baseline: no wing/room structure -- pure BM25 over all drawers
        let hits = storage.search(&case.query, None, None, k);
        results.push(evaluate_case(case, &hits, k));
    }
    results
}

fn print_quality_section(label: &str, results: &[BenchmarkResult], effective_limit: usize) {
    if results.is_empty() {
        return;
    }
    let n = results.len() as f32;
    let avg_recall: f32 = results.iter().map(|r| r.recall_at_k).sum::<f32>() / n;
    let hit_count = results.iter().filter(|r| r.hit_at_k).count();
    let avg_hit_rate = hit_count as f32 / n;
    let top1_count = results.iter().filter(|r| r.top1_hit).count();
    let avg_top1 = top1_count as f32 / n;
    let avg_mrr: f32 = results.iter().map(|r| r.mrr_at_k).sum::<f32>() / n;

    println!("── {} ({} cases) ──", label, results.len());
    println!("  Recall@{}: {:.3}", effective_limit, avg_recall);
    println!(
        "  Hit@{}:    {:.3} ({}/{})",
        effective_limit,
        avg_hit_rate,
        hit_count,
        results.len()
    );
    println!(
        "  Top-1:    {:.3} ({}/{})",
        avg_top1,
        top1_count,
        results.len()
    );
    println!("  MRR@{}:   {:.3}", effective_limit, avg_mrr);
}

fn print_stage_breakdown(stage_results: &[StageResult], effective_limit: usize) {
    if stage_results.is_empty() {
        return;
    }
    let n = stage_results.len() as f32;

    let avg_retrieval_recall: f32 = stage_results
        .iter()
        .map(|s| s.retrieval.recall_at_k)
        .sum::<f32>()
        / n;
    let avg_retrieval_hit: f32 = stage_results
        .iter()
        .filter(|s| s.retrieval.hit_at_k)
        .count() as f32
        / n;
    let avg_retrieval_mrr: f32 = stage_results
        .iter()
        .map(|s| s.retrieval.mrr_at_k)
        .sum::<f32>()
        / n;

    let avg_ranking_top1: f32 =
        stage_results.iter().filter(|s| s.ranking.top1_hit).count() as f32 / n;
    let avg_ranking_mrr: f32 = stage_results
        .iter()
        .map(|s| s.ranking.mrr_at_k)
        .sum::<f32>()
        / n;

    println!("── Stage Breakdown ──");
    println!(
        "  Retrieval: recall@{}={:.3} hit@{}={:.3} mrr={:.3}",
        effective_limit,
        avg_retrieval_recall,
        effective_limit,
        avg_retrieval_hit,
        avg_retrieval_mrr
    );
    println!(
        "  Ranking:   top1={:.3} mrr={:.3}",
        avg_ranking_top1, avg_ranking_mrr
    );

    // Per-case: highlight cases where retrieval found the answer but ranking lost it
    let retrieval_found_not_ranked: Vec<_> = stage_results
        .iter()
        .filter(|s| s.retrieval.hit_at_k && !s.ranking.top1_hit)
        .collect();
    if !retrieval_found_not_ranked.is_empty() {
        println!();
        println!(
            "  Cases retrieved but not top-1 ({} cases):",
            retrieval_found_not_ranked.len()
        );
        for s in &retrieval_found_not_ranked {
            println!(
                "    - {} (retrieved={}, not top-1; top={:?})",
                s.retrieval.case_id, s.retrieval.recall_at_k, s.ranking.top_id,
            );
        }
    }
}

fn print_adversarial_section(
    cases: &[BenchmarkCase],
    results: &[BenchmarkResult],
    effective_limit: usize,
) {
    let adv_cases: Vec<_> = cases
        .iter()
        .filter(|c| c.category == CaseCategory::Adversarial)
        .collect();
    if adv_cases.is_empty() {
        return;
    }
    let case_ids: Vec<_> = adv_cases.iter().map(|c| c.id.as_str()).collect();
    let adv_results: Vec<_> = results
        .iter()
        .filter(|r| case_ids.contains(&r.case_id.as_str()))
        .collect();

    let pass_count = adv_results
        .iter()
        .filter(|r| r.missed_ids.is_empty())
        .count();
    let fail_count = adv_results.len() - pass_count;

    println!();
    println!("── Adversarial Cases ──");
    println!("  {} PASS / {} FAIL", pass_count, fail_count);
    for result in &adv_results {
        let status = if result.missed_ids.is_empty() {
            "PASS"
        } else {
            "FAIL"
        };
        println!(
            "  [{}] {} (query=\"{}\") recall@{}={:.3} top1={} mrr={:.3}",
            status,
            result.case_id,
            result.query,
            effective_limit,
            result.recall_at_k,
            result.top1_hit,
            result.mrr_at_k,
        );
        if !result.missed_ids.is_empty() {
            println!(
                "          expected={:?} top={:?}",
                adv_cases
                    .iter()
                    .find(|c| c.id == result.case_id)
                    .map(|c| &c.expected_ids)
                    .unwrap_or(&Vec::new()),
                result.top_result_ids,
            );
        }
    }
}

fn print_side_by_side_comparison(
    results: &[BenchmarkResult],
    baseline_results: &[BenchmarkResult],
    effective_limit: usize,
) {
    if results.len() != baseline_results.len() {
        return;
    }
    let mp_recall: f32 = results.iter().map(|r| r.recall_at_k).sum::<f32>() / results.len() as f32;
    let bl_recall: f32 =
        baseline_results.iter().map(|r| r.recall_at_k).sum::<f32>() / baseline_results.len() as f32;
    let mp_hit: f32 = results.iter().filter(|r| r.hit_at_k).count() as f32 / results.len() as f32;
    let bl_hit: f32 = baseline_results.iter().filter(|r| r.hit_at_k).count() as f32
        / baseline_results.len() as f32;
    let mp_mrr: f32 = results.iter().map(|r| r.mrr_at_k).sum::<f32>() / results.len() as f32;
    let bl_mrr: f32 =
        baseline_results.iter().map(|r| r.mrr_at_k).sum::<f32>() / baseline_results.len() as f32;

    let mp_top1: f32 = results.iter().filter(|r| r.top1_hit).count() as f32 / results.len() as f32;
    let bl_top1: f32 = baseline_results.iter().filter(|r| r.top1_hit).count() as f32
        / baseline_results.len() as f32;

    fn fmt_delta(delta: f32) -> String {
        if delta >= 0.0 {
            format!("+{:.3}", delta)
        } else {
            format!("-{:.3}", delta.abs())
        }
    }

    println!("── MemPalace vs Baseline (Aggregates) ──");
    println!("  Metric            MemPalace   Baseline    Delta");
    println!(
        "  Recall@{}         {:.3}      {:.3}      {}",
        effective_limit,
        mp_recall,
        bl_recall,
        fmt_delta(mp_recall - bl_recall)
    );
    println!(
        "  Hit@{}            {:.3}      {:.3}      {}",
        effective_limit,
        mp_hit,
        bl_hit,
        fmt_delta(mp_hit - bl_hit)
    );
    println!(
        "  Top-1             {:.3}      {:.3}      {}",
        mp_top1,
        bl_top1,
        fmt_delta(mp_top1 - bl_top1)
    );
    println!(
        "  MRR@{}            {:.3}      {:.3}      {}\n",
        effective_limit,
        mp_mrr,
        bl_mrr,
        fmt_delta(mp_mrr - bl_mrr)
    );

    println!("── Per-Case Comparison ──");
    for (mp, bl) in results.iter().zip(baseline_results.iter()) {
        let rec_delta = fmt_delta(mp.recall_at_k - bl.recall_at_k);
        let hit_delta = if mp.hit_at_k == bl.hit_at_k {
            "=".to_string()
        } else if mp.hit_at_k {
            "+WIN".to_string()
        } else {
            "-LOSS".to_string()
        };
        println!(
            "  {}  recall {} hit {} mrr (mp={:.2} bl={:.2})",
            mp.case_id, rec_delta, hit_delta, mp.mrr_at_k, bl.mrr_at_k,
        );
    }
    println!();
}

fn persist_comparison_artifact(comparison: &BenchmarkComparison, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(comparison)?;
    fs::write(output_path, &json)?;
    eprintln!("Comparison artifact written to {}", output_path.display());
    Ok(())
}

pub fn run(fixture: Option<&str>, limit: usize) -> Result<()> {
    let fixture_path = fixture.unwrap_or(DEFAULT_BENCHMARK_FIXTURE);
    let cases = load_cases(Path::new(fixture_path))?;

    if cases.is_empty() {
        println!("No benchmark cases found in {}", fixture_path);
        return Ok(());
    }

    let config = load_config()?;
    let mut storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;
    seed_cases(&mut storage, &cases)?;

    let effective_limit = limit.max(1);

    // --------------- quality evaluation (MemPalace path) ---------------
    let mut results = Vec::with_capacity(cases.len());
    let mut stage_results = Vec::with_capacity(cases.len());
    for case in &cases {
        let hits = storage.search(
            &case.query,
            case.wing.as_deref(),
            case.room.as_deref(),
            effective_limit,
        );
        results.push(evaluate_case(case, &hits, effective_limit));
        stage_results.push(evaluate_stages(case, &hits, effective_limit));
    }

    // --------------- baseline evaluation (pure BM25, no structure) ----
    let baseline_results = evaluate_baseline(&storage, &cases, effective_limit);

    // --------------- grouped reporting ---------------------------------
    let smoke_results: Vec<_> = results
        .iter()
        .filter(|r| {
            cases
                .iter()
                .any(|c| c.id == r.case_id && c.category == CaseCategory::Smoke)
        })
        .cloned()
        .collect();
    let adversarial_results: Vec<_> = results
        .iter()
        .filter(|r| {
            cases
                .iter()
                .any(|c| c.id == r.case_id && c.category == CaseCategory::Adversarial)
        })
        .cloned()
        .collect();

    // --------------- print quality-first output ------------------------
    println!("Benchmark fixture: {}", fixture_path);
    println!(
        "Cases: {} ({} smoke, {} adversarial)\n",
        results.len(),
        smoke_results.len(),
        adversarial_results.len()
    );

    // Lead with product-facing quality metrics
    println!("── Quality Summary ──");
    print_quality_section("MemPalace", &results, effective_limit);
    println!();

    // Adversarial PASS/FAIL first (high-value signal)
    print_adversarial_section(&cases, &results, effective_limit);
    println!();

    // Stage breakdown: can we tell if misses are recall vs ranking failures?
    print_stage_breakdown(&stage_results, effective_limit);
    println!();

    // Baseline comparison
    print_quality_section("Baseline (BM25 only)", &baseline_results, effective_limit);
    println!();

    // Side-by-side aggregate + per-case comparison
    print_side_by_side_comparison(&results, &baseline_results, effective_limit);

    // Structured comparison artifact
    let comparison = BenchmarkComparison::build(&results, &baseline_results);
    let artifact_path = Path::new("benchmarks/latest_comparison.json");
    if let Err(e) = persist_comparison_artifact(&comparison, artifact_path) {
        eprintln!("Failed to write benchmark artifact: {}", e);
    }

    println!("JSON: {}", serde_json::to_string_pretty(&comparison)?);

    Ok(())
}

fn seed_cases(storage: &mut ChromaStorage, cases: &[BenchmarkCase]) -> Result<()> {
    for case in cases {
        let wing = case.wing.as_deref().unwrap_or("wing_benchmark");
        let room = case.room.as_deref().unwrap_or("general");

        if case.docs.is_empty() {
            // Fallback: legacy cases without explicit doc content. Seeded from
            // the query text — this is intentionally weaker but keeps tests
            // that do not yet define rich corpus fixtures working.
            for expected_id in &case.expected_ids {
                let metadata = DrawerMetadata::new(
                    wing,
                    room,
                    format!("benchmark://{}", case.id),
                    0,
                    "benchmark",
                    IngestMode::Projects,
                );
                let drawer = Drawer::new(expected_id.clone(), case.query.clone(), metadata);
                storage.add_drawer(&drawer)?;
            }
        } else {
            // Rich corpus mode: seed exact document content from the fixture.
            for doc in &case.docs {
                let metadata = DrawerMetadata::new(
                    wing,
                    room,
                    format!("benchmark://{}", case.id),
                    0,
                    "benchmark",
                    IngestMode::Projects,
                );
                let drawer = Drawer::new(doc.id.clone(), doc.content.clone(), metadata);
                storage.add_drawer(&drawer)?;
            }
        }
    }

    Ok(())
}
