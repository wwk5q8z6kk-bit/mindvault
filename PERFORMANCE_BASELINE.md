# Performance Baseline

## Status

No defensible performance baseline exists yet. The repository contains Criterion-style or unit
benchmarks for portions of SQLite/vector/engine behavior, but there is no fixed corpus, relevance
judgment set, host profile, cold/warm distinction, peak-memory capture, regression budget, or
end-to-end benchmark for the mandatory workloads.

This audit did not run a release build: the machine had approximately 4.4 GiB free while existing
build and dependency trees consumed tens of gigabytes. Running a partial debug benchmark under
that pressure would create misleading numbers and risk user work.

## Reproducible framework

`benchmarks/phase0/` defines:

- versioned workload and fixture contracts;
- cold/warm run rules;
- command-adapter contracts for exact, FTS, vector, graph, hybrid, indexing, startup, extraction,
  sync, migration, and backup/restore;
- JSON result envelopes with commit, host, tool versions, elapsed time, exit code, and peak RSS;
- comparison gates for current Rust/current implementation versus any proposed C++ path.

All twelve adapters remain explicitly required. Existing Criterion code is retained in the
manifest only as legacy probe metadata because it does not bind the fixed corpus or assert the
required correctness/relevance gates.

## Required metrics

| Workload | Primary metrics | Correctness gate |
|---|---|---|
| Exact lookup | p50/p95/p99 latency, QPS | 100% ID/source match |
| Full-text | latency, MRR/nDCG/Recall@k | fixed relevance judgments |
| Vector | latency, Recall@k, build time/size | brute-force reference |
| Graph | latency by depth/degree, visited nodes | exact fixture traversal |
| Hybrid | latency, nDCG/MRR, attribution | deterministic candidate ledger |
| Indexing | docs/s, bytes/s, write amplification | all versions projected |
| Memory | peak/steady RSS, allocation where available | no correctness loss |
| Startup | cold/warm ready time | health + schema + query |
| Extraction | MB/s, RSS, timeout/error rate | golden extracted spans |
| Sync | ops/s, bytes, conflict precision | deterministic convergence |
| Migration | wall time, peak disk/RSS | integrity/export equivalence |
| Backup/restore | throughput, size, RTO | hashes + query equivalence |

## Dataset tiers

- S: 1,000 documents / 100 attachments;
- M: 100,000 documents / 10,000 attachments;
- L: 1,000,000 documents, executed only on a provisioned benchmark host;
- adversarial: long documents, Unicode, duplicate titles, high-degree graph, binary bombs,
  contradictory statements, repeated sync conflicts, and interrupted migrations.

No real user content may enter the benchmark corpus.

## C++ gate

C++ remains prohibited until an isolated prototype and current Rust implementation use identical
fixtures, semantics, compiler optimization, hardware, cold/warm protocol, and correctness tests.
“Material” means a predeclared threshold—recommended: at least 30% p95 latency or 25% peak-RSS
improvement on a release-blocking workload—after including FFI, packaging, security, and
maintenance cost. If Rust tuning or a library change meets the target, reject C++.
