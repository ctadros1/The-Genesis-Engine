#!/bin/sh
set -eu

phase0_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase0_repo/scripts/phase0-env.sh"
cd "$phase0_repo"

cargo build --release --bin phase0-bench
phase0_tmp=$(mktemp -d)
trap 'rm -rf "$phase0_tmp"' EXIT HUP INT TERM

target/release/phase0-bench fixture --organisms 500 --ticks 500 --seed 0x5eedcafef00dbeef > "$phase0_tmp/first.json"
target/release/phase0-bench fixture --organisms 500 --ticks 500 --seed 0x5eedcafef00dbeef > "$phase0_tmp/second.json"
cmp "$phase0_tmp/first.json" "$phase0_tmp/second.json"
printf 'clean-process deterministic fixture: PASS\n'
sed -n '1p' "$phase0_tmp/first.json"

