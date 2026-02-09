use criterion::{criterion_group, criterion_main, Criterion};
use mv_core::{KnowledgeNode, NodeKind, QueryFilters};
use mv_engine::config::EngineConfig;
use mv_engine::engine::MindVaultEngine;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn create_engine(rt: &Runtime) -> (MindVaultEngine, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = EngineConfig {
        data_dir: temp_dir.path().to_string_lossy().to_string(),
        ..Default::default()
    };
    let engine = rt.block_on(async { MindVaultEngine::init(config).await.unwrap() });
    (engine, temp_dir)
}

fn bench_engine_store_node(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (engine, _tmp) = create_engine(&rt);

    c.bench_function("engine_store_node", |b| {
        b.iter(|| {
            let node = KnowledgeNode::new(NodeKind::Fact, "Benchmark content".into());
            rt.block_on(async { engine.store_node(node).await.unwrap() });
        });
    });
}

fn bench_engine_get_node(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (engine, _tmp) = create_engine(&rt);

    let node = KnowledgeNode::new(NodeKind::Fact, "Test content".into());
    let stored = rt.block_on(async { engine.store_node(node).await.unwrap() });
    let id = stored.id;

    c.bench_function("engine_get_node", |b| {
        b.iter(|| {
            rt.block_on(async { engine.get_node(id).await.unwrap() });
        });
    });
}

fn bench_engine_list_nodes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (engine, _tmp) = create_engine(&rt);

    // Pre-populate with 100 nodes
    rt.block_on(async {
        for i in 0..100 {
            let node = KnowledgeNode::new(NodeKind::Fact, format!("Content {i}"));
            engine.store_node(node).await.unwrap();
        }
    });

    c.bench_function("engine_list_100_nodes", |b| {
        let filters = QueryFilters::default();
        b.iter(|| {
            rt.block_on(async { engine.list_nodes(&filters, 100, 0).await.unwrap() });
        });
    });
}

fn bench_engine_update_node(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (engine, _tmp) = create_engine(&rt);

    let node = KnowledgeNode::new(NodeKind::Fact, "Original content".into());
    let stored = rt.block_on(async { engine.store_node(node).await.unwrap() });

    c.bench_function("engine_update_node", |b| {
        let mut version = 0u32;
        b.iter(|| {
            version += 1;
            let mut updated = stored.clone();
            updated.content = format!("Updated content v{version}");
            rt.block_on(async { engine.update_node(updated).await.unwrap() });
        });
    });
}

fn bench_engine_store_1000(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (engine, _tmp) = create_engine(&rt);

    let mut group = c.benchmark_group("engine_batch_store");
    group.sample_size(10);
    group.bench_function("1000_nodes", |b| {
        b.iter(|| {
            rt.block_on(async {
                for i in 0..1000 {
                    let node = KnowledgeNode::new(NodeKind::Fact, format!("Batch content {i}"));
                    engine.store_node(node).await.unwrap();
                }
            });
        });
    });
    group.finish();
}

fn bench_engine_list_in_1000(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (engine, _tmp) = create_engine(&rt);

    // Pre-populate with 1000 nodes of mixed kinds
    rt.block_on(async {
        for i in 0..1000 {
            let kind = if i % 3 == 0 {
                NodeKind::Task
            } else {
                NodeKind::Fact
            };
            let node = KnowledgeNode::new(kind, format!("Content {i}"));
            engine.store_node(node).await.unwrap();
        }
    });

    c.bench_function("engine_list_in_1000_nodes", |b| {
        let filters = QueryFilters::default();
        b.iter(|| {
            rt.block_on(async { engine.list_nodes(&filters, 100, 0).await.unwrap() });
        });
    });
}

criterion_group!(
    benches,
    bench_engine_store_node,
    bench_engine_get_node,
    bench_engine_list_nodes,
    bench_engine_update_node,
    bench_engine_store_1000,
    bench_engine_list_in_1000
);
criterion_main!(benches);
