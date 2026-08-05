#!/bin/sh
# Phase 5 clean-process determinism checks.
#
# Covers acceptance criterion A5.7 (both fixtures reproduce from clean
# processes under the new execution paths) and the clean-process half of
# A5.2 (a campaign's manifest does not depend on worker count).
#
# Each `lifesim` invocation below is a separate process, so passing here is
# evidence about process-independent replay, not just in-process equality.
set -eu

phase5_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase5_repo/scripts/phase0-env.sh"
cd "$phase5_repo"

cargo build --release --bin lifesim
phase5_tmp=$(mktemp -d)
trap 'rm -rf "$phase5_tmp"' EXIT HUP INT TERM

phase1_expected=0x1e3158a26afd3b39
phase2_expected=0xff9dfcff5dffbf42

# --- A5.7: fixtures under the event-log execution path -----------------------

target/release/lifesim run --ticks 500 --seed 0x5eedcafef00dbeef \
  --event-log "$phase5_tmp/phase1.alev" > "$phase5_tmp/phase1.json"
target/release/lifesim run --ticks 500 --phase2 --seed 0x5eedcafef00dbeef \
  --event-log "$phase5_tmp/phase2.alev" > "$phase5_tmp/phase2.json"

for phase in phase1 phase2; do
  case "$phase" in
    phase1) expected=$phase1_expected ;;
    phase2) expected=$phase2_expected ;;
  esac
  if ! grep -q "\"state_checksum\":\"$expected\"" "$phase5_tmp/$phase.json"; then
    printf 'phase5 fixture preservation (%s): FAIL\n' "$phase" >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase5_tmp/$phase.json" >&2
    exit 1
  fi
  # The log must actually be a valid, complete log, so the equality above
  # is not because recording silently did nothing.
  target/release/lifesim verify-events "$phase5_tmp/$phase.alev" \
    > "$phase5_tmp/$phase-events.json"
done
printf 'phase5 fixture preservation with event log enabled: PASS\n'
printf '  phase1 %s\n  phase2 %s\n' "$phase1_expected" "$phase2_expected"

# --- A5.2: a campaign manifest is independent of worker count ---------------

cat > "$phase5_tmp/campaign.txt" <<'CAMPAIGN'
# Two conditions on the fixture seed, plus a spread of small worlds.
campaign phase5-determinism
ticks 400
seeds 1 2 5 6 7 8
base preset phase2
base cells_x 64
base cells_y 64
base initial_organisms 40
base max_entities 400
condition control
condition costly
set costly basal_cost_milli_per_s 160
vary basal_cost_milli_per_s
output events off
output snapshots off
CAMPAIGN

for workers in 1 2 8; do
  target/release/lifesim batch --campaign "$phase5_tmp/campaign.txt" \
    --output "$phase5_tmp/out$workers" --workers "$workers" > /dev/null
  grep -v '^workers ' "$phase5_tmp/out$workers/manifest.txt" \
    > "$phase5_tmp/manifest-$workers.txt"
done

cmp "$phase5_tmp/manifest-1.txt" "$phase5_tmp/manifest-2.txt"
cmp "$phase5_tmp/manifest-1.txt" "$phase5_tmp/manifest-8.txt"
printf 'phase5 campaign manifest identical at 1, 2, and 8 workers: PASS\n'

# The comparison must succeed on a well-formed campaign.
target/release/lifesim report --manifest "$phase5_tmp/out1/manifest.txt" \
  > "$phase5_tmp/report.txt"
printf 'phase5 comparison report: PASS\n'
sed -n '1,6p' "$phase5_tmp/report.txt"
