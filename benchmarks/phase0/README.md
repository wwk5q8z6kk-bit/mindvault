# Phase 0 Benchmark Framework

This is a measurement harness, not a product subsystem. It does not authorize C++ or v2
implementation.

## Goals

The framework requires coverage for exact lookup, full-text retrieval, vector retrieval, graph
retrieval, hybrid ranking, indexing throughput, memory, startup, attachment extraction, sync,
migration, and backup/restore. It records the command, commit, host, versions, elapsed time, peak
RSS sampled from the child process, exit code, and output hashes in JSON.

## Prerequisites

1. Use a clean checkout at the recorded commit.
2. Use synthetic, versioned fixtures—never real user content.
3. Provision enough disk and memory; record the filesystem/device.
4. Build release binaries before timed runs.
5. Stop unrelated high-load processes.
6. Pin CPU/power settings where the host permits.

The current machine lacks enough free disk for a defensible full release run. That is why
`PERFORMANCE_BASELINE.md` reports framework-ready, measurement-blocked rather than invented
numbers.

## Commands

```bash
python3 -B benchmarks/phase0/run_baseline.py --validate
python3 -B benchmarks/phase0/run_baseline.py --list
python3 -B benchmarks/phase0/run_baseline.py \
  --overrides /absolute/path/to/benchmark-overrides.json \
  --run-context /absolute/path/to/run-context.json \
  --output benchmarks/phase0/results/baseline.json
```

Every workload is marked `adapter_required` and deliberately fails preflight for execution until a
fixed-fixture correctness adapter is provided. Three entries retain a `legacy_probe_command` for
the existing Criterion coverage, but those probes do not implement the declared correctness gate
and are not valid baseline commands. Supply an override file with the same workload IDs and actual
argv arrays; do not use shell strings.

```bash
python3 -B benchmarks/phase0/run_baseline.py \
  --overrides /absolute/path/to/benchmark-overrides.json \
  --preflight --only full_text_retrieval

python3 -B benchmarks/phase0/run_baseline.py \
  --overrides /absolute/path/to/benchmark-overrides.json \
  --run-context /absolute/path/to/run-context.json \
  --output /absolute/path/to/results.json
```

Copy `run-context.example.json` outside the repository, replace every placeholder with the actual
fixture and build identity, and use a separate file for cold and warm runs. The runner rejects an
execution without this context.

An override example:

```json
{
  "full_text_retrieval": {
    "command": ["./target/release/mv-bench", "fts", "--fixture", "/absolute/fixture"],
    "adapter_required": false
  }
}
```

## Protocol

- S/M/L fixture definitions live in `workloads.json`.
- Use at least five process-level repetitions for latency/startup and three for destructive
  migration/restore; Criterion may sample internally.
- Run cold and warm cases separately. “Cold” means a new data root and declared OS-cache handling;
  never claim an OS cold cache unless it was actually evicted.
- Correctness gates run before performance. A faster wrong result is a failure.
- Record relevance judgments and brute-force vector truth beside the fixture.
- Every result names canonical schema, index schema, tokenizer, embedding model/hash/dimension, and
  graph projection version.
- Run failure injection for indexing, sync, migration, and restore.

## Comparison

Use identical fixture hashes, commands except implementation selector, host, compiler flags, and
correctness thresholds. Compare medians and confidence intervals, not one run. A proposed C++ path
also includes FFI copies, startup, binary size, sanitizer findings, packaging, and maintenance cost.
The default materiality gate is ≥30% p95 latency or ≥25% peak-RSS improvement on a release-blocking
workload.

## Missing adapters

The legacy repository has Criterion probes for SQLite exact/store and LanceDB vector operations,
but they do not bind the versioned fixture or assert the manifest's correctness criteria. No
mandatory workload has a standardized end-to-end adapter for this baseline. Creating those
adapters is QA infrastructure to complete before v2 product work.
