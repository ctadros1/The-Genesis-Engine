#!/bin/sh
# Two-clean-process deterministic fixture for the Phase 2 kernel
# (phase2-behavior-v1 replay lineage).
set -eu

phase2_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase2_repo/scripts/phase0-env.sh"
cd "$phase2_repo"

cargo build --release --bin lifesim
phase2_tmp=$(mktemp -d)
trap 'rm -rf "$phase2_tmp"' EXIT HUP INT TERM

target/release/lifesim fixture --ticks 500 --phase2 --seed 0x5eedcafef00dbeef > "$phase2_tmp/first.json"
target/release/lifesim fixture --ticks 500 --phase2 --seed 0x5eedcafef00dbeef > "$phase2_tmp/second.json"
cmp "$phase2_tmp/first.json" "$phase2_tmp/second.json"
printf 'phase2 clean-process deterministic fixture: PASS\n'
sed -n '1p' "$phase2_tmp/first.json"
