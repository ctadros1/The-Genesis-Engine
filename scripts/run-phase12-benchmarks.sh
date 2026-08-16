#!/bin/sh
# Phase 12 benchmarks: both halves of the phase, one record.
#
# The mutable-world half's numbers (composed checksum recompute, tick cost
# disabled/quiet/patched, modification-write cost) were measured by hand on
# 2026-08-10 and copied into the plan's "Measured" table; no raw record was
# written and no script existed, which is the omission this file closes. The
# artifact half (ADR-0028) adds two measurements of its own:
#
#   - `PHASE12-BENCH artifact-tick`: median tick cost, disabled / enabled
#     with nobody bound / the `--artifact` trace's scripted population, with
#     the object count and action totals that produced the last one, so a
#     cheap number cannot be read as "objects are cheap" when it means
#     "nothing happened";
#   - `PHASE12-BENCH snapshot`: bytes per object in section 15 at 0 / 256 /
#     4,096 simple objects and 4,096 with a composite in every fourth, beside
#     the whole file, encode/decode/restore times, and a checksum round trip.
#
# **Benchmark schema 7**, unchanged from Phase 11: `TickPhase::ALL` is still
# 9, so per-phase records remain comparable.
#
# `--test-threads 1` for the kernel target because it is timed; the marker
# extraction is `grep -o` for the reason `run-phase11-benchmarks.sh` records
# (serialised, the harness prints the test name and the first measurement on
# one line, and a line-anchored grep loses the first measurement of every
# test).
set -eu

phase12_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase12_repo/scripts/phase0-env.sh"
cd "$phase12_repo"

phase12_id=${1:-phase12-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase12_output="$phase12_repo/benchmarks/raw/$phase12_id"
mkdir -p "$phase12_output"

{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase12_id"
  printf '  "benchmark_schema_version": 7,\n'
  printf '  "baseline_record": "phase11-local-20260816T063000Z",\n'
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName 2>/dev/null || uname -s)" "$(sw_vers -productVersion 2>/dev/null || uname -r)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase12_output/provenance.json"

LIFESIM_BENCH_OUTPUT="$phase12_output" \
  cargo test --release -p sim-persist --test bench_phase12_snapshot -- --ignored --nocapture \
  > "$phase12_output/phase12-snapshot.log" 2>&1

LIFESIM_BENCH_OUTPUT="$phase12_output" \
  cargo test --release -p sim-core --test bench_phase12 -- --ignored --nocapture --test-threads 1 \
  > "$phase12_output/phase12-kernel.log" 2>&1

# The mutable-world benches print without the marker (they predate it), so
# their three prefixes are collected by name beside the marked lines.
grep -ho 'PHASE12-BENCH.*\|composed_terrain_checksum .*\|tick_cost_us .*\|modification_writes .*' \
  "$phase12_output/phase12-snapshot.log" \
  "$phase12_output/phase12-kernel.log" \
  > "$phase12_output/measurements.txt" || true

# 4 snapshot + 1 artifact-tick + 1 composed checksum + 1 tick_cost_us +
# 3 modification_writes.
phase12_expected=10
phase12_found=$(grep -c . "$phase12_output/measurements.txt")
if [ "$phase12_found" -ne "$phase12_expected" ]; then
  printf 'phase12 benchmark record is short: %s measurements, expected %s\n' \
    "$phase12_found" "$phase12_expected" >&2
  exit 1
fi

printf '%s\n' "$phase12_output"
