#!/bin/sh
# Phase 17 clean-process report reproducibility (C17.3), modelled on the
# other verify scripts: the era report for the archived four-seed pilot is
# produced by two separate `lifesim` processes at the locked parameters
# (experiments/phase17-era-preregistration.md) and compared byte for byte;
# the header must echo every locked parameter; and every world line must
# be present - a report that silently skipped a world would compare equal
# to itself and prove nothing (trap 1).
set -eu

phase17_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase17_repo/scripts/phase0-env.sh"
cd "$phase17_repo"

cargo build --release --bin lifesim
phase17_tmp=$(mktemp -d)
trap 'rm -rf "$phase17_tmp"' EXIT HUP INT TERM

phase17_pilot="${PHASE17_PILOT:-$phase17_repo/../runs/phase17-era-pilot-0xa29832d1958f074c}"
[ -f "$phase17_pilot/manifest.txt" ] || {
  echo "phase17: pilot archive not found at $phase17_pilot (set PHASE17_PILOT)" >&2
  exit 1
}

target/release/lifesim era --manifest "$phase17_pilot/manifest.txt" \
  --penalty 200000000 --window 1000 --burn-in 10000 --max-segments 8 --features \
  > "$phase17_tmp/first.txt"
target/release/lifesim era --manifest "$phase17_pilot/manifest.txt" \
  --penalty 200000000 --window 1000 --burn-in 10000 --max-segments 8 --features \
  > "$phase17_tmp/second.txt"
cmp "$phase17_tmp/first.txt" "$phase17_tmp/second.txt"
echo "phase17 era report replays byte-identically across processes: PASS"

head -1 "$phase17_tmp/first.txt" | grep -q \
  "^era-report 1 campaign phase17-era-pilot detector lifesim-era-v1 window 1000 penalty 200000000 max_segments 8 burn_in 10000 features 22$" || {
  echo "phase17: the header does not echo the locked parameters:" >&2
  head -1 "$phase17_tmp/first.txt" >&2
  exit 1
}
worlds=$(grep -c "^world " "$phase17_tmp/first.txt")
[ "$worlds" -eq 8 ] || {
  echo "phase17: expected 8 world lines, found $worlds" >&2
  exit 1
}
null_boundaries=$(grep -A1 "^world condition=NULL" "$phase17_tmp/first.txt" | grep -c "^boundary" || true)
[ "$null_boundaries" -eq 0 ] || {
  echo "phase17: $null_boundaries NULL boundaries at the locked penalty (the pilot calibrated zero)" >&2
  exit 1
}
grep -q "^no segments above threshold$" "$phase17_tmp/first.txt" || {
  echo "phase17: the explicit negative result line is missing" >&2
  exit 1
}
echo "phase17 header echoes the locked parameters, all worlds present, negatives explicit: PASS"
