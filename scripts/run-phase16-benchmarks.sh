#!/bin/sh
# Phase 16 benchmark record, modelled on run-phase15-benchmarks.sh with the
# two D-113 harness lessons kept: serialised timing needs `--test-threads 1`,
# under which cargo prints `test <name> ... ` with no trailing newline, so
# extraction is `grep -o` (never `grep '^MARKER'`), and the record's exact
# line count is asserted so a silently short record is loud.
set -eu

phase16_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase16_repo/scripts/phase0-env.sh"
cd "$phase16_repo"

stamp=$(date -u +%Y%m%dT%H%M%SZ)
out="experiments/results/phase16-benchmark-measurements.txt"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT HUP INT TERM

cargo test --release -p sim-core --test bench_phase16 -- --ignored --nocapture --test-threads 1 \
  > "$tmp" 2>&1 || { cat "$tmp" >&2; exit 1; }
cargo test --release -p sim-persist --test bench_phase16_snapshot -- --ignored --nocapture --test-threads 1 \
  >> "$tmp" 2>&1 || { cat "$tmp" >&2; exit 1; }

{
  printf '# phase16-local-%s\n' "$stamp"
  printf '# benchmark schema 10: PHASE16-BENCH markers, medians over 500 ticks after 100 warmup; burst = one timed tick\n'
  grep -o 'PHASE16-BENCH.*' "$tmp"
} > "$out"

lines=$(grep -c '^PHASE16-BENCH' "$out")
if [ "$lines" -ne 3 ]; then
  printf 'phase16 benchmark record is short: %s of 3 marker lines (D-113)\n' "$lines" >&2
  cat "$out" >&2
  exit 1
fi
cat "$out"
