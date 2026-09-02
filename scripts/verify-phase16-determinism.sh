#!/bin/sh
# Phase 16 transition clean-process determinism checks, modelled on
# verify-phase15.
#
# Every `lifesim` invocation below is a separate process, so passing here is
# evidence about process-independent replay rather than in-process equality.
# The fixture is the `--transition` trace: a scratch world (no founders)
# with the whole Phase 15 field stack, pinned schema-2 policy, morphology,
# and the field-to-individual transition on (see `transition_trace_config`
# in `crates/sim-cli`).
#
# Four clauses:
#   1. clean-process replay of the trace, two processes, constants pinned;
#   2. the trace is not a control: materialization, the population it
#      produced, its feeding, spending and deaths are refused at zero,
#      and BOTH identities close from the printed totals alone - the field
#      identity with the materialized term subtracted, and the organism
#      energy identity with it added (C16.1 from the fixture's own record);
#   3. the transition-disabled field-only scratch world stays empty and
#      replays (C16.8's "disabled reproduces the field-only world");
#   4. the Phase 15 fixture is untouched - the admission refactor and the
#      new section moved nothing (C16.8).
set -eu

phase16_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase16_repo/scripts/phase0-env.sh"
cd "$phase16_repo"

cargo build --release --bin lifesim
phase16_tmp=$(mktemp -d)
trap 'rm -rf "$phase16_tmp"' EXIT HUP INT TERM

phase16_seed=0x5eedcafef00dbeef
phase16_ticks=4000
# Pinned on 2026-09-02 from the first run of this script. If any moves,
# record the old and new values in docs/22-decision-log.md and the reason.
phase16_config=0xfdd1a4b549e5927b
phase16_terrain=0xfacf6d2db889019f
phase16_state=0xe523f07aeebb8596
phase15_config=0xd6078bddc03ce7b5
phase15_state=0x12d171de7ef141b0

json_field() {
  sed -n "s/.*\"$2\":\([0-9-]*\).*/\1/p" "$1"
}

# --- clause 1: clean-process replay of the transition trace ------------------

target/release/lifesim fixture --ticks "$phase16_ticks" --transition \
  --seed "$phase16_seed" > "$phase16_tmp/first.json"
target/release/lifesim fixture --ticks "$phase16_ticks" --transition \
  --seed "$phase16_seed" > "$phase16_tmp/second.json"
cmp "$phase16_tmp/first.json" "$phase16_tmp/second.json"

for expected in \
  '"fixture_schema_version":12' \
  '"phase":"phase16"' \
  '"transition_policy":"lifesim-transition-v1"' \
  '"genome_map_version":1' \
  '"organisms":0' \
  "\"config_hash\":\"$phase16_config\"" \
  "\"terrain_checksum\":\"$phase16_terrain\"" \
  "\"state_checksum\":\"$phase16_state\""
do
  grep -q -- "$expected" "$phase16_tmp/first.json" || {
    echo "phase16: expected $expected in the transition fixture" >&2
    cat "$phase16_tmp/first.json" >&2
    exit 1
  }
done
echo "phase16 transition trace replays across processes: PASS"

# --- clause 2: the trace is not a control ------------------------------------

# `births` is printed but not refused at zero: a materialized unicell (a
# slow, blind gut) starves before its trait-derived maturity in this
# ecology, so the trace pins materialization, feeding, death and the
# deposits - not reproduction (recorded in D-130).
for field in materialized transition_events materialized_milli population \
  abiogenesis_fired microbial_total_milli produced_milli deposited_milli \
  spent_milli removed_at_death_milli assimilated_milli
do
  value=$(json_field "$phase16_tmp/first.json" "$field")
  [ -n "$value" ] && [ "$value" -gt 0 ] || {
    echo "phase16: $field is '$value' - the fixture became a control" >&2
    exit 1
  }
done
refused=$(json_field "$phase16_tmp/first.json" refused)
[ "$refused" -eq 0 ] || {
  echo "phase16: $refused admissions were refused - a bug report, not a fixture" >&2
  exit 1
}

# The field identity from the printed totals: produced + deposited -
# materialized == chemistry + microbial, to the milli-unit.
chem=$(json_field "$phase16_tmp/first.json" chemistry_total_milli)
micro=$(json_field "$phase16_tmp/first.json" microbial_total_milli)
produced=$(json_field "$phase16_tmp/first.json" produced_milli)
deposited=$(json_field "$phase16_tmp/first.json" deposited_milli)
materialized_milli=$(json_field "$phase16_tmp/first.json" materialized_milli)
[ $((chem + micro)) -eq $((produced + deposited - materialized_milli)) ] || {
  echo "phase16: field identity broken: $chem + $micro != $produced + $deposited - $materialized_milli" >&2
  exit 1
}
# The organism energy identity from the printed ledger: initial +
# assimilated + materialized - spent - removed == the living total.
organisms=$(json_field "$phase16_tmp/first.json" organism_energy_milli)
initial=$(json_field "$phase16_tmp/first.json" initial_energy_milli)
assimilated=$(json_field "$phase16_tmp/first.json" assimilated_milli)
spent=$(json_field "$phase16_tmp/first.json" spent_milli)
removed=$(json_field "$phase16_tmp/first.json" removed_at_death_milli)
[ "$organisms" -eq $((initial + assimilated + materialized_milli - spent - removed)) ] || {
  echo "phase16: energy identity broken: $organisms != $initial + $assimilated + $materialized_milli - $spent - $removed" >&2
  exit 1
}
echo "phase16 every mechanism live and both identities close: PASS"

# --- clause 3: the field-only scratch control replays and stays empty --------

target/release/lifesim fixture --ticks "$phase16_ticks" --transition --transition-off \
  --seed "$phase16_seed" > "$phase16_tmp/off1.json"
target/release/lifesim fixture --ticks "$phase16_ticks" --transition --transition-off \
  --seed "$phase16_seed" > "$phase16_tmp/off2.json"
cmp "$phase16_tmp/off1.json" "$phase16_tmp/off2.json"
off_population=$(json_field "$phase16_tmp/off1.json" population)
[ "$off_population" -eq 0 ] || {
  echo "phase16: the transition-disabled scratch world holds $off_population organisms" >&2
  exit 1
}
off_micro=$(json_field "$phase16_tmp/off1.json" microbial_total_milli)
[ "$off_micro" -gt 0 ] || {
  echo "phase16: the field-only scratch world carries no density" >&2
  exit 1
}
echo "phase16 field-only scratch control replays and stays empty: PASS"

# --- clause 4: the Phase 15 fixture is untouched -----------------------------

target/release/lifesim fixture --ticks 4000 --field \
  --seed 0x5eedcafef00dbeef > "$phase16_tmp/phase15.json"
for expected in \
  "\"config_hash\":\"$phase15_config\"" \
  "\"state_checksum\":\"$phase15_state\""
do
  grep -q -- "$expected" "$phase16_tmp/phase15.json" || {
    echo "phase16: the Phase 15 fixture moved: expected $expected" >&2
    cat "$phase16_tmp/phase15.json" >&2
    exit 1
  }
done
echo "phase16 preserves the Phase 15 fixture: PASS"
