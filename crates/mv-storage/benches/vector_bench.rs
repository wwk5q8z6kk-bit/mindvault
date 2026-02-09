use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use mv_core::*;
use std::path::Path;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Helper to create a temporary LanceDB store for benchmarks.
async fn create_temp_store(
    dir: &Path,
    dimensions: usize,
) -> Result<mv_storage::vector::LanceVectorStore, mv_core::MvError> {
    mv_storage::vector::LanceVectorStore::open(dir, dimensions).await
}

fn bench_vector_upsert(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let dimensions = 384;
    let store = rt.block_on(create_temp_store(dir.path(), dimensions)).unwrap();

    c.bench_function("vector_upsert_single", |b| {
        b.iter(|| {
            let id = Uuid::now_v7();
            let embedding: Vec<f32> = (0..dimensions).map(|i| (i as f32) * 0.001).collect();
            rt.block_on(async {
                store
                    .upsert(id, embedding, "benchmark content", None)
                    .await
                    .unwrap();
            });
        });
    });
}

fn bench_vector_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let dimensions = 384;
    let store = rt.block_on(create_temp_store(dir.path(), dimensions)).unwrap();

    // Pre-populate with 100 vectors
    rt.block_on(async {
        for i in 0..100 {
            let id = Uuid::now_v7();
            let embedding: Vec<f32> = (0..dimensions).map(|j| ((i * dimensions + j) as f32) * 0.001).collect();
            store
                .upsert(id, embedding, &format!("content {i}"), None)
                .await
                .unwrap();
        }
    });

    c.bench_function("vector_search_100", |b| {
        let query_vec: Vec<f32> = (0..dimensions).map(|i| (i as f32) * 0.002).collect();
        b.iter(|| {
            rt.block_on(async {
                store
                    .search(query_vec.clone(), 10, 0.0, None)
                    .await
                    .unwrap();
            });
        });
    });
}

fn bench_vector_batch_upsert(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_batch_upsert");
    for size in [10, 50, 200] {
        group.sample_size(if size >= 200 { 10 } else { 100 });
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let rt = Runtime::new().unwrap();
            let dir = tempfile::tempdir().unwrap();
            let dimensions = 384;
            let store = rt.block_on(create_temp_store(dir.path(), dimensions)).unwrap();

            b.iter(|| {
                rt.block_on(async {
                    for i in 0..size {
                        let id = Uuid::now_v7();
                        let embedding: Vec<f32> =
                            (0..dimensions).map(|j| ((i * dimensions + j) as f32) * 0.001).collect();
                        store
                            .upsert(id, embedding, &format!("batch content {i}"), None)
                            .await
                            .unwrap();
                    }
                });
            });
        });
    }
    group.finish();
}

fn bench_vector_search_1000(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let dimensions = 384;
    let store = rt.block_on(create_temp_store(dir.path(), dimensions)).unwrap();

    // Pre-populate with 1000 vectors
    rt.block_on(async {
        for i in 0..1000 {
            let id = Uuid::now_v7();
            let embedding: Vec<f32> =
                (0..dimensions).map(|j| ((i * dimensions + j) as f32) * 0.001).collect();
            store
                .upsert(id, embedding, &format!("content {i}"), None)
                .await
                .unwrap();
        }
    });

    let mut group = c.benchmark_group("vector_search_large");
    group.sample_size(10);

    let query_vec: Vec<f32> = (0..dimensions).map(|i| (i as f32) * 0.002).collect();

    group.bench_function("search_top10_in_1000", |b| {
        b.iter(|| {
            rt.block_on(async {
                store
                    .search(query_vec.clone(), 10, 0.0, None)
                    .await
                    .unwrap();
            });
        });
    });

    group.bench_function("search_top50_in_1000", |b| {
        b.iter(|| {
            rt.block_on(async {
                store
                    .search(query_vec.clone(), 50, 0.0, None)
                    .await
                    .unwrap();
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_vector_upsert,
    bench_vector_search,
    bench_vector_batch_upsert,
    bench_vector_search_1000
);
criterion_main!(benches);
