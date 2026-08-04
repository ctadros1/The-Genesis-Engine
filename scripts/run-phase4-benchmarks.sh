#!/bin/sh
# Phase 4 persistence benchmark: save/restore duration and snapshot size at
# 500/2,000 entities, uncompressed versus zstd levels 1/3/9. Writes a raw
# record with provenance under an ignored benchmarks/raw/<benchmark-id>/.
set -eu

phase4_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase4_repo/scripts/phase0-env.sh"
cd "$phase4_repo"

phase4_id=${1:-phase4-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase4_output="$phase4_repo/benchmarks/raw/$phase4_id"
mkdir -p "$phase4_output"

{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase4_id"
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName)" "$(sw_vers -productVersion)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase4_output/provenance.json"

LIFESIM_BENCH_OUTPUT="$phase4_output" \
  cargo test --release -p sim-persist --test bench_saves -- --ignored --nocapture \
  > "$phase4_output/persistence-bench.log" 2>&1

printf '%s\n' "$phase4_output"
