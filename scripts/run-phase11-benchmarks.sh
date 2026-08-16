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
# Two crates. `scripts/run-phase10-benchmarks.sh` invoked a `sim-persist`
# target that had never been written; under `set -eu` that aborted the script
# before it wrote its measurements file, which is why the Phase 10 record has
# no snapshot lines. This script's first revision therefore left the
# kernel-side benchmark out **deliberately, because it did not exist**.
#
# It exists now (`crates/sim-core/tests/bench_phase11.rs`: the `learn`-phase
# p50/p95 sweep, C11.6's 10^6-tick ledger horizon, and the learn-path
# allocation sweep), and this comment said "not written yet" for as long as
# that was false - so the script produced a record with no `learn` lines and
# nothing said so. Both targets are invoked below, each into its own log, and
# the measurements file is the concatenation.
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
  > "$phase11_output/phase11-snapshot.log" 2>&1

# The kernel-side half: `learn` phase p50/p95, the ledger horizon, and the
# allocation sweep. `--test-threads 1` because two of these are timed and a
# second benchmark running beside them is measured noise.
#
# **`--test-threads 1` is why the extraction below is `grep -o` and not
# `grep '^...'`, and the difference silently cost two measurements once.**
# Serialised, the harness prints `test <name> ... ` *without a trailing
# newline* and then the test's first `println!` lands on that same line. A
# line-anchored grep drops it, so every benchmark test loses its **first**
# measurement - which here was the zero-plastic-edge arm at tier 500, the
# baseline every other arm is compared against. Parallel runs do not do this,
# which is why the other phase scripts have never hit it and why this one did
# the moment it needed serialised timing. `-o` takes the marker to end of
# line, so the prefix is stripped rather than the line lost.
LIFESIM_BENCH_OUTPUT="$phase11_output" \
  cargo test --release -p sim-core --test bench_phase11 -- --ignored --nocapture --test-threads 1 \
  > "$phase11_output/phase11-kernel.log" 2>&1

grep -ho 'PHASE11-BENCH.*' \
  "$phase11_output/phase11-snapshot.log" \
  "$phase11_output/phase11-kernel.log" \
  > "$phase11_output/measurements.txt" || true

# The record must contain every measurement the two targets emit. A silently
# short record is the failure this guard exists for: it is what the
# line-anchored grep produced, and nothing downstream would have noticed.
# 6 snapshot + 6 checkpoint-stall from sim-persist; 6 learn + 6 learn-alloc
# + 1 ledger from sim-core.
phase11_expected=25
phase11_found=$(grep -c . "$phase11_output/measurements.txt")
if [ "$phase11_found" -ne "$phase11_expected" ]; then
  printf 'phase11 benchmark record is short: %s measurements, expected %s\n' \
    "$phase11_found" "$phase11_expected" >&2
  exit 1
fi

printf '%s\n' "$phase11_output"
