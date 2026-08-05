#!/bin/sh
# Phase 6 benchmark: environment-phase cost with each climate field enabled
# independently, reclassification cadence cost, and one-time founder
# generation cost by origin mode. Measured against the Phase 1 record's
# environment-phase figure. Writes a raw record with provenance under an
# ignored benchmarks/raw/<benchmark-id>/.
set -eu

phase6_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase6_repo/scripts/phase0-env.sh"
cd "$phase6_repo"

phase6_id=${1:-phase6-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase6_output="$phase6_repo/benchmarks/raw/$phase6_id"
mkdir -p "$phase6_output"

{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase6_id"
  printf '  "benchmark_schema_version": 4,\n'
  printf '  "baseline_record": "phase1-local-20260804T034607Z",\n'
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName)" "$(sw_vers -productVersion)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase6_output/provenance.json"

LIFESIM_BENCH_OUTPUT="$phase6_output" \
  cargo test --release -p sim-core --test bench_phase6 -- --ignored --nocapture \
  > "$phase6_output/phase6-bench.log" 2>&1

grep '^PHASE6-BENCH' "$phase6_output/phase6-bench.log" > "$phase6_output/measurements.txt" || true

printf '%s\n' "$phase6_output"
