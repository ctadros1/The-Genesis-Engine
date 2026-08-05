#!/bin/sh
# Phase 8 benchmark: the per-organism cost each demography mechanism adds,
# measured at the campaign's own configuration. ADR-0025 moved this phase
# earlier, so its cost applies to every campaign that follows and the plan
# asks for it explicitly. Writes a raw record with provenance under an
# ignored benchmarks/raw/<benchmark-id>/.
set -eu

phase8_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase8_repo/scripts/phase0-env.sh"
cd "$phase8_repo"

phase8_id=${1:-phase8-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase8_output="$phase8_repo/benchmarks/raw/$phase8_id"
mkdir -p "$phase8_output"

{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase8_id"
  printf '  "benchmark_schema_version": 4,\n'
  printf '  "baseline_record": "phase7-local-20260805T025643Z",\n'
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName)" "$(sw_vers -productVersion)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase8_output/provenance.json"

LIFESIM_BENCH_OUTPUT="$phase8_output" \
  cargo test --release -p sim-core --test bench_phase8 -- --ignored --nocapture \
  > "$phase8_output/phase8-bench.log" 2>&1

grep '^PHASE8-BENCH' "$phase8_output/phase8-bench.log" > "$phase8_output/measurements.txt" || true

printf '%s\n' "$phase8_output"
