#!/bin/sh
# Two-clean-process deterministic fixture for the Phase 1 kernel.
set -eu

phase1_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase1_repo/scripts/phase0-env.sh"
cd "$phase1_repo"

cargo build --release --bin lifesim
phase1_tmp=$(mktemp -d)
trap 'rm -rf "$phase1_tmp"' EXIT HUP INT TERM

target/release/lifesim fixture --ticks 500 --seed 0x5eedcafef00dbeef > "$phase1_tmp/first.json"
target/release/lifesim fixture --ticks 500 --seed 0x5eedcafef00dbeef > "$phase1_tmp/second.json"
cmp "$phase1_tmp/first.json" "$phase1_tmp/second.json"
printf 'phase1 clean-process deterministic fixture: PASS\n'
sed -n '1p' "$phase1_tmp/first.json"
