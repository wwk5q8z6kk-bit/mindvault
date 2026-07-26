#!/usr/bin/env python3
"""Run declared MindVault benchmark commands without invoking a shell."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import platform
import statistics
import subprocess
import sys
import threading
import time
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = pathlib.Path(__file__).with_name("workloads.json")
REQUIRED_RUN_CONTEXT = {
    "build_profile",
    "canonical_schema_version",
    "cold_warm",
    "compiler_flags",
    "dataset_tier",
    "embedding_model",
    "filesystem",
    "fixture_sha256",
    "graph_projection_version",
    "index_schema_versions",
    "tokenizer",
}


def read_json(path: pathlib.Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: root must be an object")
    return value


def apply_overrides(
    workloads: dict[str, dict[str, Any]], overrides_path: pathlib.Path | None
) -> None:
    if overrides_path is None:
        return
    overrides = read_json(overrides_path)
    for workload_id, override in overrides.items():
        if workload_id not in workloads:
            raise ValueError(f"unknown override workload: {workload_id}")
        if not isinstance(override, dict):
            raise ValueError(f"override {workload_id} must be an object")
        workloads[workload_id].update(override)


def validate_run_context(context: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    missing = sorted(REQUIRED_RUN_CONTEXT - set(context))
    if missing:
        errors.append(f"run context is missing: {', '.join(missing)}")
    fixture_sha256 = context.get("fixture_sha256")
    if not isinstance(fixture_sha256, str) or len(fixture_sha256) != 64:
        errors.append("run context fixture_sha256 must be a 64-character hex digest")
    elif any(character not in "0123456789abcdefABCDEF" for character in fixture_sha256):
        errors.append("run context fixture_sha256 must be hexadecimal")
    elif fixture_sha256 == "0" * 64:
        errors.append("run context fixture_sha256 must not be the example sentinel")
    if context.get("cold_warm") not in {"cold", "warm"}:
        errors.append("run context cold_warm must be cold or warm")
    if context.get("dataset_tier") not in {"S", "M", "L", "adversarial"}:
        errors.append("run context dataset_tier must be S, M, L, or adversarial")
    index_versions = context.get("index_schema_versions")
    if not isinstance(index_versions, dict):
        errors.append("run context index_schema_versions must be an object")
    elif not index_versions or any(
        not isinstance(value, str) or not value.strip() or value == "replace-me"
        for value in index_versions.values()
    ):
        errors.append("run context index_schema_versions must contain concrete versions")
    embedding_model = context.get("embedding_model")
    if not isinstance(embedding_model, dict):
        errors.append("run context embedding_model must be an object")
    else:
        for field in ("provider", "model", "revision_or_hash"):
            value = embedding_model.get(field)
            if not isinstance(value, str) or not value.strip() or value == "replace-me":
                errors.append(f"run context embedding_model.{field} must be concrete")
        dimensions = embedding_model.get("dimensions")
        if not isinstance(dimensions, int) or dimensions < 1:
            errors.append("run context embedding_model.dimensions must be positive")
    for field in (
        "build_profile",
        "canonical_schema_version",
        "compiler_flags",
        "filesystem",
        "graph_projection_version",
        "tokenizer",
    ):
        value = context.get(field)
        if not isinstance(value, str) or not value.strip() or value == "replace-me":
            errors.append(f"run context {field} must be concrete")
    return errors


def validate(
    manifest: dict[str, Any], executable_workloads: set[str] | None = None
) -> list[str]:
    errors: list[str] = []
    required = manifest.get("required_workloads")
    workloads = manifest.get("workloads")
    if not isinstance(required, list) or not isinstance(workloads, dict):
        return ["manifest requires required_workloads array and workloads object"]
    missing = sorted(set(required) - set(workloads))
    if missing:
        errors.append(f"missing required workloads: {', '.join(missing)}")
    for workload_id in required:
        spec = workloads.get(workload_id)
        if not isinstance(spec, dict):
            errors.append(f"{workload_id}: specification must be an object")
            continue
        command = spec.get("command")
        if command is not None and (
            not isinstance(command, list)
            or not command
            or not all(isinstance(item, str) and item for item in command)
        ):
            errors.append(f"{workload_id}: command must be a non-empty string array or null")
        legacy_probe = spec.get("legacy_probe_command")
        if legacy_probe is not None and (
            not isinstance(legacy_probe, list)
            or not legacy_probe
            or not all(isinstance(item, str) and item for item in legacy_probe)
        ):
            errors.append(
                f"{workload_id}: legacy_probe_command must be a non-empty string array or null"
            )
        if (
            executable_workloads is not None
            and workload_id in executable_workloads
            and (spec.get("adapter_required") or command is None)
        ):
            errors.append(f"{workload_id}: executable adapter is required")
        repetitions = spec.get("repetitions")
        if not isinstance(repetitions, int) or repetitions < 1:
            errors.append(f"{workload_id}: repetitions must be a positive integer")
        timeout_seconds = spec.get("timeout_seconds")
        if not isinstance(timeout_seconds, int) or timeout_seconds < 1:
            errors.append(f"{workload_id}: timeout_seconds must be a positive integer")
    return errors


def git_value(*args: str) -> str | None:
    try:
        return subprocess.run(
            ["git", *args],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return None


def command_version(command: list[str]) -> str | None:
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    output = (result.stdout or result.stderr).strip()
    return output.splitlines()[0] if output else None


def host_profile() -> dict[str, Any]:
    cpu = command_version(["sysctl", "-n", "machdep.cpu.brand_string"])
    if cpu is None:
        cpu = platform.processor() or None
    memory_bytes: int | None = None
    memory_value = command_version(["sysctl", "-n", "hw.memsize"])
    if memory_value and memory_value.isdigit():
        memory_bytes = int(memory_value)
    return {
        "platform": platform.platform(),
        "machine": platform.machine(),
        "cpu": cpu,
        "cpu_count": os.cpu_count(),
        "memory_bytes": memory_bytes,
        "python": platform.python_version(),
    }


def sample_peak_rss(pid: int, stop: threading.Event, result: dict[str, int | None]) -> None:
    peak_kib = 0
    while not stop.wait(0.025):
        try:
            output = subprocess.run(
                ["ps", "-o", "rss=", "-p", str(pid)],
                check=False,
                capture_output=True,
                text=True,
                timeout=1,
            ).stdout.strip()
            if output:
                peak_kib = max(peak_kib, int(output.splitlines()[0].strip()))
        except (OSError, ValueError, subprocess.SubprocessError):
            pass
    result["peak_rss_kib"] = peak_kib or None


def run_once(command: list[str], timeout_seconds: int) -> dict[str, Any]:
    started = time.perf_counter()
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=False,
        env=os.environ.copy(),
    )
    stop = threading.Event()
    memory: dict[str, int | None] = {"peak_rss_kib": None}
    sampler = threading.Thread(
        target=sample_peak_rss, args=(process.pid, stop, memory), daemon=True
    )
    sampler.start()
    timed_out = False
    try:
        stdout, stderr = process.communicate(timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        timed_out = True
        process.kill()
        stdout, stderr = process.communicate()
    stop.set()
    sampler.join(timeout=1)
    elapsed = time.perf_counter() - started
    return {
        "exit_code": process.returncode,
        "timed_out": timed_out,
        "elapsed_seconds": elapsed,
        "peak_rss_kib": memory["peak_rss_kib"],
        "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
        "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
        "stdout_tail": stdout[-4000:].decode("utf-8", errors="replace"),
        "stderr_tail": stderr[-4000:].decode("utf-8", errors="replace"),
    }


def percentile(values: list[float], percentage: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, round((len(ordered) - 1) * percentage)))
    return ordered[index]


def summarize_runs(runs: list[dict[str, Any]]) -> dict[str, Any]:
    elapsed = [float(run["elapsed_seconds"]) for run in runs]
    rss = [
        int(run["peak_rss_kib"])
        for run in runs
        if isinstance(run.get("peak_rss_kib"), int)
    ]
    return {
        "elapsed_seconds": {
            "median": statistics.median(elapsed),
            "p95": percentile(elapsed, 0.95),
            "p99": percentile(elapsed, 0.99),
        },
        "peak_rss_kib_max": max(rss) if rss else None,
        "successful_runs": sum(
            run["exit_code"] == 0 and not run["timed_out"] for run in runs
        ),
        "timed_out_runs": sum(bool(run["timed_out"]) for run in runs),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=pathlib.Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--overrides", type=pathlib.Path)
    parser.add_argument("--run-context", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--validate", action="store_true")
    parser.add_argument("--preflight", action="store_true")
    args = parser.parse_args()

    manifest = read_json(args.manifest)
    workloads = manifest.get("workloads")
    if not isinstance(workloads, dict):
        print("invalid workload manifest", file=sys.stderr)
        return 2
    apply_overrides(workloads, args.overrides)

    selected = args.only or list(manifest.get("required_workloads", []))
    unknown = sorted(set(selected) - set(workloads))
    if unknown:
        print(f"unknown workloads: {', '.join(unknown)}", file=sys.stderr)
        return 2

    if args.list:
        for workload_id in selected:
            spec = workloads[workload_id]
            state = "adapter-required" if spec.get("adapter_required") else "ready"
            print(f"{workload_id}\t{state}\t{spec.get('description', '')}")
        return 0

    errors = validate(
        manifest,
        executable_workloads=None if args.validate else set(selected),
    )
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 2
    if args.validate or args.preflight:
        if args.preflight:
            print(f"ready: {len(selected)} selected workload(s)")
            return 0
        print(f"valid: {len(manifest['required_workloads'])} required workloads declared")
        return 0
    if args.output is None:
        print("--output is required for execution", file=sys.stderr)
        return 2
    if args.run_context is None:
        print("--run-context is required for execution", file=sys.stderr)
        return 2
    run_context = read_json(args.run_context)
    context_errors = validate_run_context(run_context)
    if context_errors:
        for error in context_errors:
            print(error, file=sys.stderr)
        return 2

    report: dict[str, Any] = {
        "schema_version": 1,
        "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "git_commit": git_value("rev-parse", "HEAD"),
        "git_dirty": bool(git_value("status", "--porcelain")),
        "host": host_profile(),
        "tool_versions": {
            "rustc": command_version(["rustc", "--version"]),
            "cargo": command_version(["cargo", "--version"]),
            "node": command_version(["node", "--version"]),
            "pnpm": command_version(["pnpm", "--version"]),
            "protoc": command_version(["protoc", "--version"]),
            "sqlite": command_version(["sqlite3", "--version"]),
        },
        "manifest_sha256": hashlib.sha256(args.manifest.read_bytes()).hexdigest(),
        "run_context_sha256": hashlib.sha256(args.run_context.read_bytes()).hexdigest(),
        "run_context": run_context,
        "results": {},
    }

    any_failure = False
    for workload_id in selected:
        spec = workloads[workload_id]
        command = spec["command"]
        repetitions = spec["repetitions"]
        timeout_seconds = spec["timeout_seconds"]
        runs = []
        for _ in range(repetitions):
            run = run_once(command, timeout_seconds)
            runs.append(run)
            any_failure = any_failure or run["exit_code"] != 0 or run["timed_out"]
        report["results"][workload_id] = {
            "description": spec.get("description"),
            "correctness": spec.get("correctness"),
            "command": command,
            "timeout_seconds": timeout_seconds,
            "runs": runs,
            "summary": summarize_runs(runs),
        }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    temporary.replace(args.output)
    print(args.output)
    return 1 if any_failure else 0


if __name__ == "__main__":
    raise SystemExit(main())
