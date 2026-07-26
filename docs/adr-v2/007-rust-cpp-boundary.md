# ADR 007: Rust/C++ Boundary

Status: accepted constraint

## Problem

Decide whether any subsystem warrants C++ without increasing safety, packaging, and maintenance
risk for speculative speed.

## Current state

The core is Rust with native dependencies; no C++ subsystem is required. No mandatory benchmark
shows Rust is inadequate.

## Alternatives

1. Rust only;
2. C++ for retrieval/indexing/extraction;
3. external C++ process;
4. rewrite core in C++.

## Measurements required

Identical workload and semantics, release optimization, p95/p99, throughput, peak RSS, startup,
binary size, crash behavior, FFI copies, build/signing time, vulnerability surface, and engineering
cost.

## Security implications

C++ increases memory-safety and FFI risk. An external process contains crashes better than in-process
FFI but adds IPC.

## Privacy implications

Copies across FFI/IPC and crash dumps can expose content; data minimization and zeroization matter.

## Migration implications

None unless a benchmark gate passes. Any prototype uses an adapter and synthetic corpus.

## Chosen direction

Rust remains the implementation language. C++ is permitted only for an isolated hotspot after
identical benchmarks show at least the predeclared material benefit and total cost remains lower.

## Rejected alternatives

Language-preference rewrites and broad C++ adoption are rejected. No current measurement supports
them.

## Reversal path

Keep a Rust reference implementation and adapter; remove the C++ module without schema/export
change.

## Acceptance criteria

Correctness equivalence, recommended ≥30% p95 or ≥25% peak-RSS improvement on a release-blocking
workload, security review, packaging proof, and owner approval. Otherwise no C++.
