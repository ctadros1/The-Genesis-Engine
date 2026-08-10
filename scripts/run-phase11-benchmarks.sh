#!/bin/sh
# Phase 11 benchmark: what learned state costs to store and to checkpoint.
#
# This is C11.7's measurement. `PlasticityConfig::max_plastic_edges` shipped
# at 32 with an explicit obligation to restate it from measurement, the same
# obligation C9.8 discharged for the structural caps, and the number that
# settles it is bytes per plastic edge - not bytes per snapshot.
#
# Three conditions per tier, and the reasons are in the test's own header:
# `off` is the baseline, `evolved` is the realistic level and may be near zero
# because nothing in the founder is plastic, and `seeded` is the upper bound
# the cap has to be set against. Each byte count is printed beside the
# plastic-edge fraction that produced it, so a small number cannot be read as
# "cheap" when it means "nothing evolved".
#
# **Benchmark schema 7**, per the phase plan's numbering correction: schema 5
# was never emitted by any script and 6 is the highest in use (Phase 10).
# `TickPhase::ALL` is 9 rather than 8 as of the `learn` phase, so per-phase
# records are comparable only within a schema version.
#
# One crate, not two. `scripts/run-phase10-benchmarks.sh` invoked a
# `sim-persist` target that had never been written; under `set -eu` that
# aborted the script before it wrote its measurements file, which is why the
# Phase 10 record has no snapshot lines. The kernel-side `learn`-phase timing
# benchmark (`sim-core --test bench_phase11`) is not written yet, so it is
# **deliberately absent here** rather than invoked and missing.
set -eu

phase11_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase11_repo/scripts/phase0-env.sh"
cd "$phase11_repo"

phase11_id=${1:-phase11-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase11_output="$phase11_repo/benchmarks/raw/$phase11_id"
mkdir -p "$phase11_output"

{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase11_id"
  printf '  "benchmark_schema_version": 7,\n'
  printf '  "baseline_record": "phase9-local-20260805T160000Z",\n'
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName)" "$(sw_vers -productVersion)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase11_output/provenance.json"

LIFESIM_BENCH_OUTPUT="$phase11_output" \
  cargo test --release -p sim-persist --test bench_phase11_snapshot -- --ignored --nocapture \
  > "$phase11_output/phase11-bench.log" 2>&1

grep '^PHASE11-BENCH' "$phase11_output/phase11-bench.log" > "$phase11_output/measurements.txt" || true

printf '%s\n' "$phase11_output"
