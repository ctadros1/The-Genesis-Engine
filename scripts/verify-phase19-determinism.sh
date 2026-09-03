#!/bin/sh
# Phase 19 chemistry-as-food clean-process determinism checks, modelled on
# verify-phase16.
#
# Every `lifesim` invocation below is a separate process, so passing here is
# evidence about process-independent replay rather than in-process equality.
# The fixture is the `--transition --coupled` trace: the Phase 16 scratch
# world with coupling v2 (ADR-0034) - organisms may eat the substrate in
# their own cell once the biomass there is gone, at
# `chemistry.consumption_fraction_q16` = Q16_ONE and the shipped yield.
#
# Four clauses:
#   1. clean-process replay of the trace, two processes, constants pinned;
#   2. the trace is not a control: consumption, materialization, feeding,
#      spending and deaths are refused at zero, and BOTH identities close
#      from the printed totals alone - the field identity with the
#      materialized AND consumed terms subtracted (C19.1), the organism
#      energy identity with materialization added (the yield is inside
#      assimilated_milli, the loss inside deposited_milli);
#   3. the coupling-v1 trace (`--transition` alone) is untouched: its
#      config hash and state checksum are the Phase 16 pins, so the mouth
#      is hashed only when it is open and a v1 world replays byte for byte
#      (C19.2, ADR-0034 §"Unchanged when off");
#   4. the Phase 15 field fixture is untouched.
set -eu

phase19_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase19_repo/scripts/phase0-env.sh"
cd "$phase19_repo"

cargo build --release --bin lifesim
phase19_tmp=$(mktemp -d)
trap 'rm -rf "$phase19_tmp"' EXIT HUP INT TERM

phase19_seed=0x5eedcafef00dbeef
phase19_ticks=4000
# Pinned on 2026-09-03 from the first run of this script. If any moves,
# record the old and new values in docs/22-decision-log.md and the reason.
phase19_config=0x7cfe66d39cda2e2b
phase19_terrain=0xfacf6d2db889019f
phase19_state=0x2137b2286076cd63
# The Phase 16 pins (scripts/verify-phase16-determinism.sh), repeated here
# so clause 3 fails if either script drifts alone.
phase16_config=0xfdd1a4b549e5927b
phase16_state=0xe523f07aeebb8596
phase15_config=0xd6078bddc03ce7b5
phase15_state=0x12d171de7ef141b0

json_field() {
  sed -n "s/.*\"$2\":\([0-9-]*\).*/\1/p" "$1"
}

# --- clause 1: clean-process replay of the coupled trace ---------------------

target/release/lifesim fixture --ticks "$phase19_ticks" --transition --coupled \
  --seed "$phase19_seed" > "$phase19_tmp/first.json"
target/release/lifesim fixture --ticks "$phase19_ticks" --transition --coupled \
  --seed "$phase19_seed" > "$phase19_tmp/second.json"
cmp "$phase19_tmp/first.json" "$phase19_tmp/second.json"

for expected in \
  '"fixture_schema_version":13' \
  '"phase":"phase19"' \
  '"coupling_policy":"lifesim-chemistry-coupling-v2"' \
  '"organisms":0' \
  "\"config_hash\":\"$phase19_config\"" \
  "\"terrain_checksum\":\"$phase19_terrain\"" \
  "\"state_checksum\":\"$phase19_state\""
do
  grep -q -- "$expected" "$phase19_tmp/first.json" || {
    echo "phase19: expected $expected in the coupled fixture" >&2
    cat "$phase19_tmp/first.json" >&2
    exit 1
  }
done
echo "phase19 coupled trace replays across processes: PASS"

# --- clause 2: the trace is not a control ------------------------------------

# `births` is printed but not refused at zero (as in Phase 16): whether a
# fed unicell reaches the pairing threshold inside 4,000 ticks is C19.4's
# measurement, not a fixture claim.
for field in consumed_milli materialized materialized_milli population \
  microbial_total_milli produced_milli deposited_milli \
  spent_milli removed_at_death_milli assimilated_milli
do
  value=$(json_field "$phase19_tmp/first.json" "$field")
  [ -n "$value" ] && [ "$value" -gt 0 ] || {
    echo "phase19: $field is '$value' - the fixture became a control" >&2
    exit 1
  }
done
refused=$(json_field "$phase19_tmp/first.json" refused)
[ "$refused" -eq 0 ] || {
  echo "phase19: $refused admissions were refused - a bug report, not a fixture" >&2
  exit 1
}

# The field identity from the printed totals: produced + deposited -
# materialized - consumed == chemistry + microbial, to the milli-unit.
chem=$(json_field "$phase19_tmp/first.json" chemistry_total_milli)
micro=$(json_field "$phase19_tmp/first.json" microbial_total_milli)
produced=$(json_field "$phase19_tmp/first.json" produced_milli)
deposited=$(json_field "$phase19_tmp/first.json" deposited_milli)
materialized_milli=$(json_field "$phase19_tmp/first.json" materialized_milli)
consumed=$(json_field "$phase19_tmp/first.json" consumed_milli)
[ $((chem + micro)) -eq $((produced + deposited - materialized_milli - consumed)) ] || {
  echo "phase19: field identity broken: $chem + $micro != $produced + $deposited - $materialized_milli - $consumed" >&2
  exit 1
}
# The organism energy identity from the printed ledger: initial +
# assimilated + materialized - spent - removed == the living total. The
# consumed term does not appear: its yield is already inside
# assimilated_milli and its loss inside deposited_milli.
organisms=$(json_field "$phase19_tmp/first.json" organism_energy_milli)
initial=$(json_field "$phase19_tmp/first.json" initial_energy_milli)
assimilated=$(json_field "$phase19_tmp/first.json" assimilated_milli)
spent=$(json_field "$phase19_tmp/first.json" spent_milli)
removed=$(json_field "$phase19_tmp/first.json" removed_at_death_milli)
[ "$organisms" -eq $((initial + assimilated + materialized_milli - spent - removed)) ] || {
  echo "phase19: energy identity broken: $organisms != $initial + $assimilated + $materialized_milli - $spent - $removed" >&2
  exit 1
}
# The yield cannot exceed what was assimilated in total, and the loss is
# inside the deposits: two bounds that a mis-credited yield would break.
[ "$assimilated" -ge $((consumed * 6 / 10)) ] || {
  echo "phase19: assimilated $assimilated is below the yield of consumed $consumed" >&2
  exit 1
}
[ "$deposited" -ge $((consumed - consumed * 6 / 10 - 1)) ] || {
  echo "phase19: deposited $deposited is below the loss of consumed $consumed" >&2
  exit 1
}
echo "phase19 every mechanism live and both identities close: PASS"

# --- clause 3: the coupling-v1 trace is the Phase 16 trace ------------------

target/release/lifesim fixture --ticks "$phase19_ticks" --transition \
  --seed "$phase19_seed" > "$phase19_tmp/v1.json"
for expected in \
  '"fixture_schema_version":12' \
  "\"config_hash\":\"$phase16_config\"" \
  "\"state_checksum\":\"$phase16_state\""
do
  grep -q -- "$expected" "$phase19_tmp/v1.json" || {
    echo "phase19: the coupling-v1 (Phase 16) fixture moved: expected $expected" >&2
    cat "$phase19_tmp/v1.json" >&2
    exit 1
  }
done
[ "$phase19_config" != "$phase16_config" ] || {
  echo "phase19: the coupled config hashes like the uncoupled one - the mouth is not hashed" >&2
  exit 1
}
echo "phase19 preserves the coupling-v1 (Phase 16) fixture: PASS"

# --- clause 4: the Phase 15 fixture is untouched -----------------------------

target/release/lifesim fixture --ticks 4000 --field \
  --seed 0x5eedcafef00dbeef > "$phase19_tmp/phase15.json"
for expected in \
  "\"config_hash\":\"$phase15_config\"" \
  "\"state_checksum\":\"$phase15_state\""
do
  grep -q -- "$expected" "$phase19_tmp/phase15.json" || {
    echo "phase19: the Phase 15 fixture moved: expected $expected" >&2
    cat "$phase19_tmp/phase15.json" >&2
    exit 1
  }
done
echo "phase19 preserves the Phase 15 fixture: PASS"
