use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use mv_core::*;
use mv_storage::sqlite::SqliteNodeStore;
use tokio::runtime::Runtime;

fn bench_insert_single(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = SqliteNodeStore::open_in_memory().unwrap();

    c.bench_function("insert_single_node", |b| {
        b.iter(|| {
            let node = KnowledgeNode::new(NodeKind::Fact, "Benchmark content".into());
            rt.block_on(async { store.insert(&node).await.unwrap() });
        });
    });
}

fn bench_get_by_id(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = SqliteNodeStore::open_in_memory().unwrap();

    // Pre-populate
    let node = KnowledgeNode::new(NodeKind::Fact, "Test content".into());
    let id = node.id;
    rt.block_on(async { store.insert(&node).await.unwrap() });

    c.bench_function("get_by_id", |b| {
        b.iter(|| {
            rt.block_on(async { store.get(id).await.unwrap() });
        });
    });
}

fn bench_list_with_filters(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = SqliteNodeStore::open_in_memory().unwrap();

    // Pre-populate with 100 nodes
    rt.block_on(async {
        for i in 0..100 {
            let node = KnowledgeNode::new(NodeKind::Fact, format!("Content {i}"));
            store.insert(&node).await.unwrap();
        }
    });

    c.bench_function("list_100_nodes", |b| {
        let filters = QueryFilters::default();
        b.iter(|| {
            rt.block_on(async { store.list(&filters, 100, 0).await.unwrap() });
        });
    });
}

fn bench_batch_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_insert");
    for size in [100, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let rt = Runtime::new().unwrap();
            b.iter(|| {
                let store = SqliteNodeStore::open_in_memory().unwrap();
                rt.block_on(async {
                    for i in 0..size {
                        let node = KnowledgeNode::new(NodeKind::Fact, format!("Content {i}"));
                        store.insert(&node).await.unwrap();
                    }
                });
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_insert_single,
    bench_get_by_id,
    bench_list_with_filters,
    bench_batch_insert
);
criterion_main!(benches);
