#!/bin/sh
# Phase 9 benchmark: what a variable topology costs to evaluate and to
# store. This is C9.8's measurement - the structural caps shipped
# provisional with an explicit obligation to restate them against a measured
# snapshot budget - plus the plan's Benchmark Impact asks: controller cost
# against evolved topology size, and the distribution rather than the mean,
# "because evolved topology sizes will be skewed".
#
# Two crates, because sim-core is deliberately dependency-free and the
# snapshot half needs the codec.
set -eu

phase9_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase9_repo/scripts/phase0-env.sh"
cd "$phase9_repo"

phase9_id=${1:-phase9-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase9_output="$phase9_repo/benchmarks/raw/$phase9_id"
mkdir -p "$phase9_output"

{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase9_id"
  printf '  "benchmark_schema_version": 4,\n'
  printf '  "baseline_record": "phase8-local-20260805T120000Z",\n'
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName)" "$(sw_vers -productVersion)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase9_output/provenance.json"

LIFESIM_BENCH_OUTPUT="$phase9_output" \
  cargo test --release -p sim-core --test bench_phase9 -- --ignored --nocapture \
  > "$phase9_output/phase9-bench.log" 2>&1

LIFESIM_BENCH_OUTPUT="$phase9_output" \
  cargo test --release -p sim-persist --test bench_phase9_snapshot -- --ignored --nocapture \
  >> "$phase9_output/phase9-bench.log" 2>&1

grep '^PHASE9-BENCH' "$phase9_output/phase9-bench.log" > "$phase9_output/measurements.txt" || true

printf '%s\n' "$phase9_output"
