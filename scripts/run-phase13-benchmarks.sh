#!/bin/sh
# Phase 13 benchmark record, modelled on run-phase12-benchmarks.sh with the
# two D-113 harness lessons kept: serialised timing needs `--test-threads 1`,
# under which cargo prints `test <name> ... ` with no trailing newline, so
# extraction is `grep -o` (never `grep '^MARKER'`), and the record's exact
# line count is asserted so a silently short record is loud.
set -eu

phase13_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase13_repo/scripts/phase0-env.sh"
cd "$phase13_repo"

stamp=$(date -u +%Y%m%dT%H%M%SZ)
out="experiments/results/phase13-benchmark-measurements.txt"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT HUP INT TERM

cargo test --release -p sim-core --test bench_phase13 -- --ignored --nocapture --test-threads 1 \
  > "$tmp" 2>&1 || { cat "$tmp" >&2; exit 1; }

{
  printf '# phase13-local-%s\n' "$stamp"
  printf '# benchmark schema 10: PHASE13-BENCH markers, medians over 500 ticks after 100 warmup\n'
  grep -o 'PHASE13-BENCH.*' "$tmp"
} > "$out"

lines=$(grep -c '^PHASE13-BENCH' "$out")
if [ "$lines" -ne 7 ]; then
  printf 'phase13 benchmark record is short: %s of 7 marker lines (D-113)\n' "$lines" >&2
  cat "$out" >&2
  exit 1
fi
printf 'phase13 benchmark record written: %s (%s markers)\n' "$out" "$lines"
