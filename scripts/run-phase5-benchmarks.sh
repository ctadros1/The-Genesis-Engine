#!/bin/sh
# Phase 5 benchmark: headless throughput per world at both tiers, scheduler
# scaling and host contention at worker counts 1/2/4/8, event-log write cost
# and growth rate, and the checkpoint stall on the tick thread
# (synchronous versus asynchronous) measured against the Phase 4 record
# phase4-local-20260804T141013Z. Writes a raw record with provenance under
# an ignored benchmarks/raw/<benchmark-id>/.
set -eu

phase5_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase5_repo/scripts/phase0-env.sh"
cd "$phase5_repo"

phase5_id=${1:-phase5-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase5_output="$phase5_repo/benchmarks/raw/$phase5_id"
mkdir -p "$phase5_output"

{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase5_id"
  printf '  "benchmark_schema_version": 3,\n'
  printf '  "baseline_record": "phase4-local-20260804T141013Z",\n'
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName)" "$(sw_vers -productVersion)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string)"
  printf '  "cpu_threads": "%s",\n' "$(sysctl -n hw.logicalcpu)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase5_output/provenance.json"

LIFESIM_BENCH_OUTPUT="$phase5_output" \
  cargo test --release -p sim-experiment --test bench_phase5 -- --ignored --nocapture \
  > "$phase5_output/phase5-bench.log" 2>&1

grep '^PHASE5-BENCH' "$phase5_output/phase5-bench.log" > "$phase5_output/measurements.txt" || true

printf '%s\n' "$phase5_output"
