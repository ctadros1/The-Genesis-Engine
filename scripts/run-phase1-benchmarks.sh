#!/bin/sh
# Phase 1 kernel benchmark at the documented 500 and 2,000 organism tiers.
# Writes raw CSV and summary JSON with provenance under an ignored
# benchmarks/raw/<benchmark-id>/ directory.
set -eu

phase1_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase1_repo/scripts/phase0-env.sh"
cd "$phase1_repo"

phase1_id=${1:-phase1-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase1_output="$phase1_repo/benchmarks/raw/$phase1_id"
mkdir -p "$phase1_output"

cargo build --release --bin lifesim
scripts/verify-phase1-determinism.sh > "$phase1_output/determinism.txt"

# Ecology scenarios (reproduction on; population follows the live
# trajectory) and fixed-population scenarios (reproduction off) at both
# documented tiers.
for phase1_count in 500 2000; do
  target/release/lifesim benchmark \
    --benchmark-id "$phase1_id" \
    --output "$phase1_output" \
    --organisms "$phase1_count" \
    --seed 0x5eedcafef00dbeef \
    --warmup 200 \
    --samples 50 \
    --ticks-per-sample 10 \
    > "$phase1_output/phase1-rust-$phase1_count-stdout.json"
  target/release/lifesim benchmark \
    --benchmark-id "$phase1_id" \
    --output "$phase1_output" \
    --organisms "$phase1_count" \
    --no-reproduction \
    --seed 0x5eedcafef00dbeef \
    --warmup 200 \
    --samples 50 \
    --ticks-per-sample 10 \
    > "$phase1_output/phase1-rust-$phase1_count-fixedpop-stdout.json"
done

printf '%s\n' "$phase1_output"
