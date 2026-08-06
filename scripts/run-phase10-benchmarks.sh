#!/bin/sh
# Phase 10 benchmark: what a modular body costs, plus C10.9's long ledger.
#
# The plan is explicit that **caps are set from this measurement, not before
# it**, and asks for the per-organism cost against module count as a
# *distribution* rather than a mean, since evolved sizes are skewed. C10.9's
# million-tick ledger run is included rather than left to a separate
# command, because a slow check invoked separately is a check that quietly
# stops being run.
#
# Benchmark schema 6.
set -eu

phase10_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase10_repo/scripts/phase0-env.sh"
cd "$phase10_repo"

phase10_id=${1:-phase10-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase10_output="$phase10_repo/benchmarks/raw/$phase10_id"
mkdir -p "$phase10_output"

{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase10_id"
  printf '  "benchmark_schema_version": 6,\n'
  printf '  "baseline_record": "phase9-local-20260805T160000Z",\n'
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName)" "$(sw_vers -productVersion)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase10_output/provenance.json"

LIFESIM_BENCH_OUTPUT="$phase10_output" \
  cargo test --release -p sim-core --test bench_phase10 -- --ignored --nocapture \
  > "$phase10_output/phase10-bench.log" 2>&1

LIFESIM_BENCH_OUTPUT="$phase10_output" \
  cargo test --release -p sim-persist --test bench_phase10 -- --ignored --nocapture \
  >> "$phase10_output/phase10-bench.log" 2>&1

grep '^PHASE10-BENCH' "$phase10_output/phase10-bench.log" > "$phase10_output/measurements.txt" || true

printf '%s\n' "$phase10_output"
