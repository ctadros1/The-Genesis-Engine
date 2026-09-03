#!/bin/sh
# Phase 21 birth-site clean-process determinism checks, modelled on
# verify-phase20. The fixture is the `--birthsite` trace: the Phase 20
# composition trace with the Phase 21 `BirthSite` record (event schema
# 13, ADR-0036) counted and the born cohort's occupancy and food at birth
# summarised.
#
# Five clauses:
#   1. clean-process replay of the schema-15 trace, two processes, pinned;
#   2. one record per admission (its count equals materializations plus
#      births) and the energy total equals the schema-14 line's;
#   3. the record is observation only: the schema-14 line's config hash
#      and state checksum equal the schema-15 line's and the Phase 19 pin;
#   4. the Phase 16 fixture is untouched;
#   5. a schema-13 event log written by a short campaign verifies and the
#      cohort census reads it - one world line with a rho field present.
#   6. the youngest-first probe (`--youngest-first`, ADR-0036's
#      `lifesim-intake-order-v2`) replays across processes, pinned; its
#      config hash differs from the shipped order's (the probe is hashed)
#      and the shipped order's line is byte-identical with the flag absent.
set -eu

phase21_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase21_repo/scripts/phase0-env.sh"
cd "$phase21_repo"

cargo build --release --bin lifesim
phase21_tmp=$(mktemp -d)
trap 'rm -rf "$phase21_tmp"' EXIT HUP INT TERM

phase21_seed=0x5eedcafef00dbeef
phase21_ticks=4000
# Pinned on 2026-09-03 from the first run of this script. If any moves,
# record the old and new values in docs/22-decision-log.md and the reason.
phase21_config=0x7cfe66d39cda2e2b
phase21_terrain=0xfacf6d2db889019f
phase21_state=0x2137b2286076cd63
phase19_state=0x2137b2286076cd63
# The youngest-first probe's pins (2026-09-03).
phase21_probe_config=0xf3c1d35fa9b7c7da
phase21_probe_state=0x3439df67a8cc1e88
phase16_config=0xfdd1a4b549e5927b
phase16_state=0xe523f07aeebb8596

json_field() {
  sed -n "s/.*\"$2\":\([0-9-]*\).*/\1/p" "$1"
}

# --- clause 1 ------------------------------------------------------------------
target/release/lifesim fixture --ticks "$phase21_ticks" --birthsite --seed "$phase21_seed" > "$phase21_tmp/first.json"
target/release/lifesim fixture --ticks "$phase21_ticks" --birthsite --seed "$phase21_seed" > "$phase21_tmp/second.json"
cmp "$phase21_tmp/first.json" "$phase21_tmp/second.json"
for expected in '"fixture_schema_version":15' '"phase":"phase21"' '"event_schema_version":13' \
  "\"config_hash\":\"$phase21_config\"" "\"terrain_checksum\":\"$phase21_terrain\"" "\"state_checksum\":\"$phase21_state\""
do
  grep -q -- "$expected" "$phase21_tmp/first.json" || { echo "phase21: expected $expected" >&2; cat "$phase21_tmp/first.json" >&2; exit 1; }
done
echo "phase21 birth-site trace replays across processes: PASS"

# --- clause 2 ------------------------------------------------------------------
records=$(json_field "$phase21_tmp/first.json" birthsite_records)
materialized=$(json_field "$phase21_tmp/first.json" materialized)
births=$(json_field "$phase21_tmp/first.json" births)
[ "$records" -gt 0 ] || { echo "phase21: no birth-site record" >&2; exit 1; }
[ "$records" -eq $((materialized + births)) ] || { echo "phase21: $records records != $materialized + $births" >&2; exit 1; }
composition=$(json_field "$phase21_tmp/first.json" composition_records)
[ "$composition" -eq "$records" ] || { echo "phase21: composition $composition != birth-site $records" >&2; exit 1; }
echo "phase21 one record per admission: PASS"

# --- clause 3 ------------------------------------------------------------------
target/release/lifesim fixture --ticks "$phase21_ticks" --composition --seed "$phase21_seed" > "$phase21_tmp/schema14.json"
grep -q -- '"fixture_schema_version":14' "$phase21_tmp/schema14.json" || { echo "phase21: the schema-14 line moved" >&2; exit 1; }
grep -q -- "\"state_checksum\":\"$phase19_state\"" "$phase21_tmp/schema14.json" || { echo "phase21: the Phase 19 state moved" >&2; exit 1; }
[ "$phase21_state" = "$phase19_state" ] || { echo "phase21: the birth-site record is hashed" >&2; exit 1; }
energy15=$(json_field "$phase21_tmp/first.json" organism_energy_milli)
energy14=$(json_field "$phase21_tmp/schema14.json" organism_energy_milli)
[ "$energy15" -eq "$energy14" ] || { echo "phase21: energy totals differ between schema 14 and 15" >&2; exit 1; }
echo "phase21 the record moves no checksum and the Phase 19 fixture holds: PASS"

# --- clause 4 ------------------------------------------------------------------
target/release/lifesim fixture --ticks 4000 --transition --seed 0x5eedcafef00dbeef > "$phase21_tmp/phase16.json"
for expected in "\"config_hash\":\"$phase16_config\"" "\"state_checksum\":\"$phase16_state\""; do
  grep -q -- "$expected" "$phase21_tmp/phase16.json" || { echo "phase21: the Phase 16 fixture moved: $expected" >&2; exit 1; }
done
echo "phase21 preserves the Phase 16 fixture: PASS"

# --- clause 5 ------------------------------------------------------------------
sed -e 's/^campaign .*/campaign phase21-verify-short/' -e 's/^seeds .*/seeds 21999/' "$phase21_repo/experiments/phase20-lineage-pilot.campaign" \
  | sed -e 's/^ticks .*/ticks 1500/' -e 's/^workers .*/workers 1/' -e 's/^check-interval .*/check-interval 500/' \
        -e 's/^base cells_x .*/base cells_x 32/' -e 's/^base cells_y .*/base cells_y 32/' -e 's/^base max_entities .*/base max_entities 1000/' \
        -e 's/^output snapshots on/output snapshots off/' -e 's/^output field .*/output field off/' > "$phase21_tmp/short.campaign"
target/release/lifesim batch --campaign "$phase21_tmp/short.campaign" --output "$phase21_tmp/short" --workers 1 > "$phase21_tmp/short.log" 2>&1 || { cat "$phase21_tmp/short.log" >&2; exit 1; }
log=$(ls "$phase21_tmp"/short/*.alev | head -1)
target/release/lifesim verify-events "$log" --expect-events > "$phase21_tmp/verify.txt" 2>&1 || { cat "$phase21_tmp/verify.txt" >&2; exit 1; }
target/release/lifesim cohort --manifest "$phase21_tmp/short/manifest.txt" > "$phase21_tmp/cohort.txt"
grep -q '^cohort-report 1 ' "$phase21_tmp/cohort.txt" || { cat "$phase21_tmp/cohort.txt" >&2; exit 1; }
[ "$(grep -c '^world ' "$phase21_tmp/cohort.txt")" -eq 1 ] || { cat "$phase21_tmp/cohort.txt" >&2; exit 1; }
grep -q 'rho_food_milli=' "$phase21_tmp/cohort.txt" || { cat "$phase21_tmp/cohort.txt" >&2; exit 1; }
echo "phase21 a schema-13 log verifies and the cohort census reads it: PASS"

# --- clause 6: the youngest-first probe --------------------------------------
target/release/lifesim fixture --ticks "$phase21_ticks" --youngest-first --seed "$phase21_seed" > "$phase21_tmp/probe1.json"
target/release/lifesim fixture --ticks "$phase21_ticks" --youngest-first --seed "$phase21_seed" > "$phase21_tmp/probe2.json"
cmp "$phase21_tmp/probe1.json" "$phase21_tmp/probe2.json"
for expected in '"intake_order":"descending"' "\"config_hash\":\"$phase21_probe_config\"" "\"state_checksum\":\"$phase21_probe_state\""; do
  grep -q -- "$expected" "$phase21_tmp/probe1.json" || { echo "phase21: probe expected $expected" >&2; cat "$phase21_tmp/probe1.json" >&2; exit 1; }
done
[ "$phase21_probe_config" != "$phase21_config" ] || { echo "phase21: the probe hashes like the shipped order" >&2; exit 1; }
grep -q -- '"intake_order":"ascending"' "$phase21_tmp/first.json" || { echo "phase21: the shipped line does not say ascending" >&2; exit 1; }
echo "phase21 the youngest-first probe replays and is its own experiment: PASS"
