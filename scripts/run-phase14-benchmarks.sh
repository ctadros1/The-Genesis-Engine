#!/bin/sh
# Phase 14 benchmark record, modelled on run-phase13-benchmarks.sh with the
# two D-113 harness lessons kept: serialised timing needs `--test-threads 1`,
# under which cargo prints `test <name> ... ` with no trailing newline, so
# extraction is `grep -o` (never `grep '^MARKER'`), and the record's exact
# line count is asserted so a silently short record is loud.
set -eu

phase14_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase14_repo/scripts/phase0-env.sh"
cd "$phase14_repo"

stamp=$(date -u +%Y%m%dT%H%M%SZ)
out="experiments/results/phase14-benchmark-measurements.txt"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT HUP INT TERM

cargo test --release -p sim-core --test bench_phase14 -- --ignored --nocapture --test-threads 1 \
  > "$tmp" 2>&1 || { cat "$tmp" >&2; exit 1; }

{
  printf '# phase14-local-%s\n' "$stamp"
  printf '# benchmark schema 10: PHASE14-BENCH markers, medians over 500 ticks after 100 warmup\n'
  grep -o 'PHASE14-BENCH.*' "$tmp"
} > "$out"

lines=$(grep -c '^PHASE14-BENCH' "$out")
if [ "$lines" -ne 2 ]; then
  printf 'phase14 benchmark record is short: %s of 2 marker lines (D-113)\n' "$lines" >&2
  cat "$out" >&2
  exit 1
fi
printf 'phase14 benchmark record written: %s\n' "$out"
cat "$out"
