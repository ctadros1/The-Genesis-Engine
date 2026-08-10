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

# There is deliberately **no `sim-persist` invocation here**, and its absence
# is the fix rather than the omission.
#
# This script used to run `cargo test -p sim-persist --test bench_phase10`.
# That target has never existed - Phase 10's snapshot claim (C10.10, "bodies
# are derived, so snapshot size is unaffected") was argued from the fact that
# no body is stored, not measured - so cargo exited with "no test target named
# bench_phase10", and under `set -eu` the script aborted on that line, before
# the `grep` that writes `measurements.txt`. Every Phase 10 benchmark run
# therefore produced a log and no measurements file, which is why the
# committed `experiments/results/phase10-benchmark-measurements.txt` has no
# snapshot lines in it.
#
# Invoking a target that does not exist is worse than not invoking one: it
# turns a missing measurement into a failed script, and a failed script into a
# missing record for the measurements that *did* run. The snapshot measurement
# lives in `scripts/run-phase11-benchmarks.sh`, where the section that costs
# per-organism bytes actually exists.
grep '^PHASE10-BENCH' "$phase10_output/phase10-bench.log" > "$phase10_output/measurements.txt" || true

printf '%s\n' "$phase10_output"
