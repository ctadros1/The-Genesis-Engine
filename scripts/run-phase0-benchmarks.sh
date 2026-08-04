#!/bin/sh
set -eu

phase0_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase0_repo/scripts/phase0-env.sh"
cd "$phase0_repo"

phase0_id=${1:-phase0-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase0_output="$phase0_repo/benchmarks/raw/$phase0_id"
mkdir -p "$phase0_output"

cargo build --release --bin phase0-bench
scripts/verify-determinism.sh > "$phase0_output/determinism.txt"

for phase0_count in 500 2000; do
  target/release/phase0-bench benchmark \
    --benchmark-id "$phase0_id" \
    --output "$phase0_output" \
    --organisms "$phase0_count" \
    --seed 0x5eedcafef00dbeef \
    --warmup 100 \
    --samples 50 \
    --ticks-per-sample 10 \
    > "$phase0_output/rust-$phase0_count-stdout.json"
done

printf '%s\n' "$phase0_output"

