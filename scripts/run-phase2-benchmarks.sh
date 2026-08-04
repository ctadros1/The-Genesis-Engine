#!/bin/sh
# Phase 2 kernel benchmark at the documented 500 and 2,000 entity tiers,
# with controllers enabled. Ecology scenarios keep pairing on (population
# follows the live trajectory); -fixedpop scenarios disable reproduction
# for comparable controller cost at a near-constant population. Each
# summary includes per-phase timings (sense = sensor gathering,
# controllers = controller evaluation) and the offline similarity-analysis
# runtime measured separately from tick cost.
set -eu

phase2_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase2_repo/scripts/phase0-env.sh"
cd "$phase2_repo"

phase2_id=${1:-phase2-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase2_output="$phase2_repo/benchmarks/raw/$phase2_id"
mkdir -p "$phase2_output"

cargo build --release --bin lifesim
scripts/verify-phase2-determinism.sh > "$phase2_output/determinism.txt"

for phase2_count in 500 2000; do
  target/release/lifesim benchmark \
    --benchmark-id "$phase2_id" \
    --output "$phase2_output" \
    --organisms "$phase2_count" \
    --phase2 \
    --seed 0x5eedcafef00dbeef \
    --warmup 200 \
    --samples 50 \
    --ticks-per-sample 10 \
    > "$phase2_output/phase2-rust-$phase2_count-stdout.json"
  target/release/lifesim benchmark \
    --benchmark-id "$phase2_id" \
    --output "$phase2_output" \
    --organisms "$phase2_count" \
    --phase2 \
    --no-reproduction \
    --seed 0x5eedcafef00dbeef \
    --warmup 200 \
    --samples 50 \
    --ticks-per-sample 10 \
    > "$phase2_output/phase2-rust-$phase2_count-fixedpop-stdout.json"
done

printf '%s\n' "$phase2_output"
