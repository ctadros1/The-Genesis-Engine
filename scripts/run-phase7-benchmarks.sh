#!/bin/sh
# Phase 7 benchmark: per-phase cost of contest at the 500 and 2,000 tiers,
# and the carcass entity-count effect measured separately from the
# per-organism cost. Measured against the Phase 5 record. Writes a raw
# record with provenance under an ignored benchmarks/raw/<benchmark-id>/.
set -eu

phase7_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase7_repo/scripts/phase0-env.sh"
cd "$phase7_repo"

phase7_id=${1:-phase7-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase7_output="$phase7_repo/benchmarks/raw/$phase7_id"
mkdir -p "$phase7_output"

{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase7_id"
  printf '  "benchmark_schema_version": 4,\n'
  printf '  "baseline_record": "phase5-local-20260804T210059Z",\n'
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName)" "$(sw_vers -productVersion)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase7_output/provenance.json"

LIFESIM_BENCH_OUTPUT="$phase7_output" \
  cargo test --release -p sim-core --test bench_phase7 -- --ignored --nocapture \
  > "$phase7_output/phase7-bench.log" 2>&1

grep '^PHASE7-BENCH' "$phase7_output/phase7-bench.log" > "$phase7_output/measurements.txt" || true

printf '%s\n' "$phase7_output"
