use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mempalace::benchmark::{evaluate_case, load_cases, BenchmarkCase};
use mempalace::dialect::aaak::AaakDialect;
use mempalace::palace::{Drawer, DrawerMetadata, IngestMode};
use mempalace::storage::ChromaStorage;
use mempalace::tokenizer::{LocalTokenizer, Tokenizer};
use std::hint::black_box;
use std::path::Path;
use std::time::Duration;
use tempfile::tempdir;

const FIXTURE_PATH: &str = "fixtures/benchmark_cases.json";
const BENCH_LIMIT: usize = 5;

fn make_drawer(id: &str, doc: &str, wing: &str, room: &str) -> Drawer {
    Drawer::new(
        id,
        doc,
        DrawerMetadata::new(wing, room, "bench.rs", 0, "bench", IngestMode::Projects),
    )
}

fn benchmark_doc(case: &BenchmarkCase, expected_id: &str) -> String {
    format!(
        "fixture={} query={} expected={} wing={} room={}",
        case.id,
        case.query,
        expected_id,
        case.wing.as_deref().unwrap_or("wing_benchmark"),
        case.room.as_deref().unwrap_or("general")
    )
}

fn seed_storage(cases: &[BenchmarkCase]) -> ChromaStorage {
    let dir = tempdir().unwrap();
    let mut storage = ChromaStorage::new(dir.path(), "bench").unwrap();
    for case in cases {
        let wing = case.wing.as_deref().unwrap_or("wing_benchmark");
        let room = case.room.as_deref().unwrap_or("general");
        for expected_id in &case.expected_ids {
            storage
                .add_drawer(&make_drawer(
                    expected_id,
                    &benchmark_doc(case, expected_id),
                    wing,
                    room,
                ))
                .unwrap();
        }
    }
    storage
}

fn seed_storage_with_size(n: usize) -> ChromaStorage {
    let dir = tempdir().unwrap();
    let mut storage = ChromaStorage::new(dir.path(), "bench").unwrap();

    let rooms = ["auth", "deploy", "general", "ci", "database"];
    let queries = [
        "user bought laptop",
        "deploy failed because env var missing",
        "oauth callback mismatch bug",
        "api rate limit exceeded",
        "database timeout on query",
    ];

    for i in 0..n {
        let idx = i % 5;
        let wing = format!("wing_{:04}", i / 10 + 1);
        let room = rooms[i % rooms.len()];
        storage
            .add_drawer(&make_drawer(
                &format!("scaled_{:05}", i),
                queries[idx],
                &wing,
                room,
            ))
            .unwrap();
    }

    storage
}

fn load_fixture_cases() -> Vec<BenchmarkCase> {
    load_cases(Path::new(FIXTURE_PATH)).expect("benchmark fixture should deserialize")
}

fn benchmark_config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(30)
}

fn bench_retrieval_from_fixtures(c: &mut Criterion) {
    let cases = load_fixture_cases();
    let storage = seed_storage(&cases);
    let mut group = c.benchmark_group("search_latency");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(1));

    for case in &cases {
        group.bench_with_input(
            BenchmarkId::new("search_and_score", &case.id),
            case,
            |b, case| {
                b.iter(|| {
                    let hits = storage.search(
                        black_box(&case.query),
                        case.wing.as_deref(),
                        case.room.as_deref(),
                        BENCH_LIMIT,
                    );
                    black_box(evaluate_case(case, &hits, BENCH_LIMIT));
                })
            },
        );
    }

    group.finish();
}

fn bench_search_latency_by_size(c: &mut Criterion) {
    let cases = load_fixture_cases();
    let sizes = [100, 500, 1000];
    let mut group = c.benchmark_group("search_latency_scaled");
    group.sample_size(15);
    group.warm_up_time(Duration::from_secs(1));

    for size in &sizes {
        let storage = seed_storage_with_size(*size);
        let warmup_case = cases.first().unwrap();

        group.bench_with_input(
            BenchmarkId::new("search", format!("n={}", size)),
            warmup_case,
            |b, case| {
                b.iter(|| {
                    let hits = storage.search(
                        black_box(&case.query),
                        case.wing.as_deref(),
                        case.room.as_deref(),
                        BENCH_LIMIT,
                    );
                    black_box(hits);
                })
            },
        );
    }

    group.finish();
}

fn bench_local_tokenizer(c: &mut Criterion) {
    let tokenizer = LocalTokenizer::new();
    let text =
        "deploy failed because env var missing in production auth callback mismatch oauth bug";
    let mut group = c.benchmark_group("tokenizer");
    group.sample_size(50);
    group.warm_up_time(Duration::from_secs(1));
    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("local_count", |b| {
        b.iter(|| black_box(tokenizer.count(black_box(text))))
    });
    group.finish();
}

fn bench_aaak_compress(c: &mut Criterion) {
    let dialect = AaakDialect::new();
    let text = "Alice and Bob discussed the deploy failure, traced it to a missing env var, and decided to fix the oauth callback mismatch in the next release.";
    let mut group = c.benchmark_group("aaak");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(1));
    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("compress", |b| {
        b.iter(|| black_box(dialect.compress(black_box(text)).unwrap()))
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = benchmark_config();
    targets = bench_retrieval_from_fixtures, bench_search_latency_by_size, bench_local_tokenizer, bench_aaak_compress
}
criterion_main!(benches);
