#!/bin/sh
# Phase 20 body-composition clean-process determinism checks, modelled on
# verify-phase19.
#
# Every `lifesim` invocation below is a separate process. The fixture is the
# `--composition` trace: the Phase 19 coupled scratch world with the Phase 20
# `BodyComposition` record (event schema 12, ADR-0035) counted as the world
# emits it.
#
# Five clauses:
#   1. clean-process replay of the schema-14 trace, two processes, pinned;
#   2. the record is emitted once per admission - its count equals
#      materializations plus births - and both identities close from the
#      printed totals (the record moves no ledger);
#   3. the record is observation only: the schema-13 (`--transition
#      --coupled`) line's state checksum equals the schema-14 line's and
#      both equal the Phase 19 pin;
#   4. the Phase 16 fixture is untouched;
#   5. a schema-12 event log written by a short campaign verifies, and the
#      census (`lifesim lineage`) reads it - one world line with a
#      composition field present.
set -eu

phase20_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase20_repo/scripts/phase0-env.sh"
cd "$phase20_repo"

cargo build --release --bin lifesim
phase20_tmp=$(mktemp -d)
trap 'rm -rf "$phase20_tmp"' EXIT HUP INT TERM

phase20_seed=0x5eedcafef00dbeef
phase20_ticks=4000
# Pinned on 2026-09-03 from the first run of this script. If any moves,
# record the old and new values in docs/22-decision-log.md and the reason.
# The event_schema_version literal moved 12 -> 13 on 2026-09-03 (Phase 21,
# ADR-0036: the BirthSite record); the config, terrain and state pins did
# not move, which is what the record's neutrality asserts.
phase20_config=0x7cfe66d39cda2e2b
phase20_terrain=0xfacf6d2db889019f
phase20_state=0x2137b2286076cd63
# The Phase 19 and Phase 16 pins, repeated so clauses 3 and 4 fail if any
# script drifts alone.
phase19_state=0x2137b2286076cd63
phase16_config=0xfdd1a4b549e5927b
phase16_state=0xe523f07aeebb8596

json_field() {
  sed -n "s/.*\"$2\":\([0-9-]*\).*/\1/p" "$1"
}

# --- clause 1: clean-process replay ------------------------------------------

target/release/lifesim fixture --ticks "$phase20_ticks" --composition \
  --seed "$phase20_seed" > "$phase20_tmp/first.json"
target/release/lifesim fixture --ticks "$phase20_ticks" --composition \
  --seed "$phase20_seed" > "$phase20_tmp/second.json"
cmp "$phase20_tmp/first.json" "$phase20_tmp/second.json"
for expected in \
  '"fixture_schema_version":14' \
  '"phase":"phase20"' \
  '"event_schema_version":13' \
  '"organisms":0' \
  "\"config_hash\":\"$phase20_config\"" \
  "\"terrain_checksum\":\"$phase20_terrain\"" \
  "\"state_checksum\":\"$phase20_state\""
do
  grep -q -- "$expected" "$phase20_tmp/first.json" || {
    echo "phase20: expected $expected in the composition fixture" >&2
    cat "$phase20_tmp/first.json" >&2
    exit 1
  }
done
echo "phase20 composition trace replays across processes: PASS"

# --- clause 2: one record per admission, identities exact --------------------

records=$(json_field "$phase20_tmp/first.json" composition_records)
materialized=$(json_field "$phase20_tmp/first.json" materialized)
births=$(json_field "$phase20_tmp/first.json" births)
[ "$records" -gt 0 ] || { echo "phase20: no composition record - the fixture became a control" >&2; exit 1; }
[ "$records" -eq $((materialized + births)) ] || {
  echo "phase20: $records records != $materialized materialized + $births births" >&2
  exit 1
}
seen=$(json_field "$phase20_tmp/first.json" max_modules_seen)
[ "$seen" -ge 1 ] || { echo "phase20: max_modules_seen is $seen" >&2; exit 1; }
chem=$(json_field "$phase20_tmp/first.json" chemistry_total_milli)
micro=$(json_field "$phase20_tmp/first.json" microbial_total_milli)
produced=$(json_field "$phase20_tmp/first.json" produced_milli)
deposited=$(json_field "$phase20_tmp/first.json" deposited_milli)
materialized_milli=$(json_field "$phase20_tmp/first.json" materialized_milli)
consumed=$(json_field "$phase20_tmp/first.json" consumed_milli)
[ $((chem + micro)) -eq $((produced + deposited - materialized_milli - consumed)) ] || {
  echo "phase20: field identity broken" >&2; exit 1; }
organisms=$(json_field "$phase20_tmp/first.json" organism_energy_milli)
initial=$(json_field "$phase20_tmp/first.json" initial_energy_milli)
assimilated=$(json_field "$phase20_tmp/first.json" assimilated_milli)
spent=$(json_field "$phase20_tmp/first.json" spent_milli)
removed=$(json_field "$phase20_tmp/first.json" removed_at_death_milli)
[ "$organisms" -eq $((initial + assimilated + materialized_milli - spent - removed)) ] || {
  echo "phase20: energy identity broken" >&2; exit 1; }
echo "phase20 one record per admission and both identities close: PASS"

# --- clause 3: observation only ------------------------------------------------

target/release/lifesim fixture --ticks "$phase20_ticks" --transition --coupled \
  --seed "$phase20_seed" > "$phase20_tmp/schema13.json"
grep -q -- '"fixture_schema_version":13' "$phase20_tmp/schema13.json" || {
  echo "phase20: the schema-13 line moved" >&2; exit 1; }
grep -q -- "\"state_checksum\":\"$phase19_state\"" "$phase20_tmp/schema13.json" || {
  echo "phase20: the Phase 19 fixture moved" >&2; cat "$phase20_tmp/schema13.json" >&2; exit 1; }
[ "$phase20_state" = "$phase19_state" ] || {
  echo "phase20: the composition record is hashed - the schema-14 state differs from schema 13" >&2; exit 1; }
echo "phase20 the record moves no checksum and the Phase 19 fixture holds: PASS"

# --- clause 4: the Phase 16 fixture is untouched -------------------------------

target/release/lifesim fixture --ticks 4000 --transition \
  --seed 0x5eedcafef00dbeef > "$phase20_tmp/phase16.json"
for expected in "\"config_hash\":\"$phase16_config\"" "\"state_checksum\":\"$phase16_state\""; do
  grep -q -- "$expected" "$phase20_tmp/phase16.json" || {
    echo "phase20: the Phase 16 fixture moved: expected $expected" >&2; exit 1; }
done
echo "phase20 preserves the Phase 16 fixture: PASS"

# --- clause 5: a schema-12 log verifies and the census reads it ----------------

cat > "$phase20_tmp/short.campaign" <<CAMPAIGN
campaign phase20-verify-short
ticks 1500
workers 1
seeds 20999
check-interval 500
base preset phase2
base cells_x 32
base cells_y 32
base initial_organisms 0
base max_entities 1000
base origin.mode scratch
base genome2.enabled true
base morphology.enabled true
base chemistry.enabled true
base chemistry.field_steps_per_tick 1
base chemistry.microbial_enabled true
base chemistry.abiogenesis_enabled true
base chemistry.production_milli_per_step 40
base chemistry.excretion_fraction_q16 32768
base chemistry.remains_fraction_q16 32768
base chemistry.consumption_fraction_q16 65536
base transition.enabled true
base transition.check_interval_ticks 25
base transition.density_floor_milli 4000
base transition.persistence_checks 2
base transition.organism_energy_milli 4000
condition short
output events on
output snapshots off
output field off
CAMPAIGN
target/release/lifesim batch --campaign "$phase20_tmp/short.campaign" \
  --output "$phase20_tmp/short" --workers 1 > "$phase20_tmp/short.log" 2>&1 || {
  cat "$phase20_tmp/short.log" >&2; exit 1; }
log=$(ls "$phase20_tmp"/short/*.alev | head -1)
target/release/lifesim verify-events "$log" --expect-events > "$phase20_tmp/verify.txt" 2>&1 || {
  cat "$phase20_tmp/verify.txt" >&2; exit 1; }
target/release/lifesim lineage --manifest "$phase20_tmp/short/manifest.txt" > "$phase20_tmp/lineage.txt"
grep -q '^lineage-report 1 ' "$phase20_tmp/lineage.txt" || { cat "$phase20_tmp/lineage.txt" >&2; exit 1; }
[ "$(grep -c '^world ' "$phase20_tmp/lineage.txt")" -eq 1 ] || { cat "$phase20_tmp/lineage.txt" >&2; exit 1; }
grep -q 'multi_compositions=' "$phase20_tmp/lineage.txt" || { cat "$phase20_tmp/lineage.txt" >&2; exit 1; }
echo "phase20 a schema-12 log verifies and the census reads it: PASS"
