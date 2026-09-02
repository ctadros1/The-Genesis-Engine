#!/bin/sh
# Phase 15 field-regime clean-process determinism checks, modelled on
# verify-phase14.
#
# Every `lifesim` invocation below is a separate process, so passing here is
# evidence about process-independent replay rather than in-process equality.
# The fixture is the `--field` trace: a phase-1 ecology with the whole field
# stack on - chemistry, the microbial classes, abiogenesis, and both
# coupling fractions (see `field_trace_config` in `crates/sim-cli`).
#
# Four clauses:
#   1. clean-process replay of the trace, two processes, constants pinned;
#   2. the trace is not a control: every Phase 15 mechanism's observable is
#      refused at zero, and the printed totals close the joint identity
#      exactly (produced + deposited == chemistry + microbial);
#   3. the scaffold arm replays, is a distinct lineage, and carries exactly
#      the same production total - redistribution never adds (ADR-0018);
#   4. the Phase 14 fixture is untouched (its own script owns the fuller
#      preservation matrix; this clause pins the one constant a Phase 15
#      regression would move first).
set -eu

phase15_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase15_repo/scripts/phase0-env.sh"
cd "$phase15_repo"

cargo build --release --bin lifesim
phase15_tmp=$(mktemp -d)
trap 'rm -rf "$phase15_tmp"' EXIT HUP INT TERM

phase15_seed=0x5eedcafef00dbeef
phase15_ticks=4000
# Pinned on 2026-09-02 from the first run of this script. If any moves,
# record the old and new values in docs/22-decision-log.md and the reason.
phase15_config=0xd6078bddc03ce7b5
phase15_terrain=0xfacf6d2db889019f
phase15_state=0x12d171de7ef141b0
phase15_scaffold_config=0x1d0f194f6551b50b
phase15_scaffold_state=0x8a061b15c9040b78
phase14_config=0x87ef88945d7672af
phase14_state=0x92ceac70c45d7e3f

# --- clause 1: clean-process replay of the field trace -----------------------

target/release/lifesim fixture --ticks "$phase15_ticks" --field \
  --seed "$phase15_seed" > "$phase15_tmp/first.json"
target/release/lifesim fixture --ticks "$phase15_ticks" --field \
  --seed "$phase15_seed" > "$phase15_tmp/second.json"
cmp "$phase15_tmp/first.json" "$phase15_tmp/second.json"

for expected in \
  '"fixture_schema_version":11' \
  '"phase":"phase15"' \
  '"chemistry_policy":"lifesim-chemistry-v1"' \
  '"microbial_policy":"lifesim-microbial-v1"' \
  "\"config_hash\":\"$phase15_config\"" \
  "\"terrain_checksum\":\"$phase15_terrain\"" \
  "\"state_checksum\":\"$phase15_state\""
do
  grep -q -- "$expected" "$phase15_tmp/first.json" || {
    echo "phase15: expected $expected in the field fixture" >&2
    cat "$phase15_tmp/first.json" >&2
    exit 1
  }
done
echo "phase15 field trace replays across processes: PASS"

# --- clause 2: the trace is not a control ------------------------------------

json_field() {
  sed -n "s/.*\"$2\":\([0-9-]*\).*/\1/p" "$1"
}

for field in produced_milli deposited_milli abiogenesis_fired \
  microbial_total_milli occupied_cells
do
  value=$(json_field "$phase15_tmp/first.json" "$field")
  [ -n "$value" ] && [ "$value" -gt 0 ] || {
    echo "phase15: $field is '$value' - the fixture became a control" >&2
    exit 1
  }
done

# The joint identity, from the printed totals alone: produced + deposited
# must equal chemistry + microbial to the milli-unit.
chem=$(json_field "$phase15_tmp/first.json" chemistry_total_milli)
micro=$(json_field "$phase15_tmp/first.json" microbial_total_milli)
produced=$(json_field "$phase15_tmp/first.json" produced_milli)
deposited=$(json_field "$phase15_tmp/first.json" deposited_milli)
[ $((chem + micro)) -eq $((produced + deposited)) ] || {
  echo "phase15: identity broken: $chem + $micro != $produced + $deposited" >&2
  exit 1
}
echo "phase15 every mechanism live and the identity closes: PASS"

# --- clause 3: the scaffold arm ----------------------------------------------

target/release/lifesim fixture --ticks "$phase15_ticks" --field-scaffold \
  --seed "$phase15_seed" > "$phase15_tmp/scaffold1.json"
target/release/lifesim fixture --ticks "$phase15_ticks" --field-scaffold \
  --seed "$phase15_seed" > "$phase15_tmp/scaffold2.json"
cmp "$phase15_tmp/scaffold1.json" "$phase15_tmp/scaffold2.json"

for expected in \
  "\"config_hash\":\"$phase15_scaffold_config\"" \
  "\"state_checksum\":\"$phase15_scaffold_state\""
do
  grep -q -- "$expected" "$phase15_tmp/scaffold1.json" || {
    echo "phase15: expected $expected in the scaffold fixture" >&2
    cat "$phase15_tmp/scaffold1.json" >&2
    exit 1
  }
done
[ "$phase15_scaffold_state" != "$phase15_state" ] || {
  echo "phase15: the scaffold arm is not a distinct lineage" >&2
  exit 1
}
scaffold_produced=$(json_field "$phase15_tmp/scaffold1.json" produced_milli)
[ "$scaffold_produced" -eq "$produced" ] || {
  echo "phase15: scaffold produced $scaffold_produced != neutral $produced" >&2
  exit 1
}
echo "phase15 scaffold arm replays, distinct lineage, same production: PASS"

# --- clause 4: the Phase 14 fixture is untouched -----------------------------

target/release/lifesim fixture --ticks 8000 --phase2 --genome2 \
  --physiology-v2 --seed 0x5eedcafef00dbeef > "$phase15_tmp/phase14.json"
for expected in \
  "\"config_hash\":\"$phase14_config\"" \
  "\"state_checksum\":\"$phase14_state\""
do
  grep -q -- "$expected" "$phase15_tmp/phase14.json" || {
    echo "phase15: the Phase 14 fixture moved: expected $expected" >&2
    cat "$phase15_tmp/phase14.json" >&2
    exit 1
  }
done
echo "phase15 preserves the Phase 14 fixture: PASS"
