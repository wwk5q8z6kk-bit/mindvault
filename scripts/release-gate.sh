#!/usr/bin/env bash
# mindvault release gate — the single command that decides PASS / FAIL / UNVERIFIED.
#
# Exit codes:  0 pass · 1 fail · 2 unverified (a check could not be measured)
#
# WHY SERIAL (--test-threads=1) — do not "optimise" this away:
#
#   `crates/mv-server/tests/api_integration.rs` mutates PROCESS-WIDE environment
#   variables to exercise real behaviour: MINDVAULT_COMMAND_ADMISSION_MODE,
#   MINDVAULT_NAMESPACE_NODE_QUOTA, MINDVAULT_TEST_FAIL_POST_UNSEAL_*.
#   `ScopedEnvVar` takes a mutex, but that only serialises WRITERS against each
#   other. It cannot stop the other ~30 tests from concurrently READING the
#   mutated environment: `AppState` calls `CommandAdmissionPolicy::from_env()`
#   at construction (crates/mv-server/src/state.rs:376).
#
#   Measured 2026-07-28:
#     parallel : 22 passed / 15 failed  (5 of 5 runs) — ten tests expect 201, get 403
#     serial   : 37 passed /  0 failed
#
#   The failures are a test-harness constraint, NOT a product defect. Making the
#   readers take the same lock deadlocks, because the writer already holds it
#   before calling setup_with_config(). The durable fix is to move the five
#   env-mutating tests into their own test binary (cargo gives each binary its
#   own process); until then, serial is the correct execution mode.
#
# KNOWN RESIDUAL — mv-index META_LOCK flake, ~6.7% (1 in 15 runs, 2026-07-28):
#   sealed_tantivy_files_do_not_store_plaintext_payload panics in idx.commit()
#   with "Failed to acquire Lockfile: IoError(NotFound)". This is active work in
#   flight (see commit "fix: stop sealed Tantivy META_LOCK"). It is a SETUP
#   failure — the test never reaches its plaintext assertion — so it is not
#   evidence of a plaintext leak. The gate reports it honestly rather than
#   retrying until green: a gate that retries until it likes the answer is not
#   a gate.

set -uo pipefail
cd "$(dirname "$0")/.."

GATE="${HOA_GATE:-$HOME/.claude/bin/hoa-gate}"
OUT="${1:-target/quality_report.json}"
mkdir -p "$(dirname "$OUT")"

if [[ ! -x "$GATE" ]]; then
    echo "release-gate: $GATE not found or not executable" >&2
    echo "release-gate: UNVERIFIED — cannot measure without the gate" >&2
    exit 2
fi

echo "== 1/3  compile =="
"$GATE" verify --command "cargo check --workspace --quiet" \
    --cwd "$PWD" --name "mindvault-compile" --timeout 900 >/dev/null || {
    rc=$?; echo "compile: FAILED (exit $rc)"; exit $rc; }
echo "compile: ok"

echo "== 2/3  clippy =="
"$GATE" verify --command "cargo clippy --workspace --all-targets --quiet -- -D warnings" \
    --cwd "$PWD" --name "mindvault-clippy" --timeout 1200 >/dev/null
clippy_rc=$?
[[ $clippy_rc -eq 0 ]] && echo "clippy: ok" || echo "clippy: exit $clippy_rc"

echo "== 3/3  test suite (serial — see header) =="
"$GATE" verify --command "cargo test --workspace --quiet -- --test-threads=1" \
    --cwd "$PWD" --name "mindvault-suite" --out "$OUT" --timeout 2400
suite_rc=$?

echo
echo "report: $OUT"
if [[ $suite_rc -eq 0 && $clippy_rc -eq 0 ]]; then
    echo "RELEASE GATE: PASS"
    exit 0
elif [[ $suite_rc -eq 2 ]]; then
    echo "RELEASE GATE: UNVERIFIED — a check could not be measured; this is not a pass"
    exit 2
else
    echo "RELEASE GATE: FAIL"
    exit 1
fi
