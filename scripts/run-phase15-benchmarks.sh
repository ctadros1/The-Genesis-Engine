#!/bin/sh
# Phase 15 benchmark record, modelled on run-phase14-benchmarks.sh with the
# two D-113 harness lessons kept: serialised timing needs `--test-threads 1`,
# under which cargo prints `test <name> ... ` with no trailing newline, so
# extraction is `grep -o` (never `grep '^MARKER'`), and the record's exact
# line count is asserted so a silently short record is loud.
set -eu

phase15_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase15_repo/scripts/phase0-env.sh"
cd "$phase15_repo"

stamp=$(date -u +%Y%m%dT%H%M%SZ)
out="experiments/results/phase15-benchmark-measurements.txt"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT HUP INT TERM

cargo test --release -p sim-core --test bench_phase15 -- --ignored --nocapture --test-threads 1 \
  > "$tmp" 2>&1 || { cat "$tmp" >&2; exit 1; }

{
  printf '# phase15-local-%s\n' "$stamp"
  printf '# benchmark schema 10: PHASE15-BENCH markers, medians over 500 ticks after 100 warmup\n'
  grep -o 'PHASE15-BENCH.*' "$tmp"
} > "$out"

lines=$(grep -c '^PHASE15-BENCH' "$out")
if [ "$lines" -ne 2 ]; then
  printf 'phase15 benchmark record is short: %s of 2 marker lines (D-113)\n' "$lines" >&2
  cat "$out" >&2
  exit 1
fi
cat "$out"
