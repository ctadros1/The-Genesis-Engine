#!/bin/sh
# Phase 14 physiology-v2 clean-process determinism checks (C14.5's fixture
# clause), modelled on verify-phase13.
#
# Every `lifesim` invocation below is a separate process, so passing here is
# evidence about process-independent replay rather than in-process equality.
# The fixture is the `--physiology-v2` trace: the Phase 13 social trace's
# world plus morphology and the physiology section, founders additionally
# scripted to emit mate intent every tick, carrying prefer-far preference
# genes, and founder 0 growing a six-module body from one module - so
# growth billing, grown-prefix phenotypes, preference scoring and the
# choice event all run inside an affordable horizon (see
# `physiology_trace_config` in `crates/sim-cli`).
#
# Four clauses:
#   1. clean-process replay of the trace, two processes, constants pinned;
#   2. the trace is not a control: every Phase 14 mechanism's count is
#      refused at zero;
#   3. the scramble arm and the inert arm (both ADR-0030 gates off on the
#      same world) each replay, are distinct lineages, and the scramble
#      arm's own counter is nonzero - the checked-never-configured rule;
#   4. the Phase 13 fixture is untouched (its own script owns the fuller
#      preservation matrix; this clause pins the one constant a Phase 14
#      regression would move first).
set -eu

phase14_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase14_repo/scripts/phase0-env.sh"
cd "$phase14_repo"

cargo build --release --bin lifesim
phase14_tmp=$(mktemp -d)
trap 'rm -rf "$phase14_tmp"' EXIT HUP INT TERM

phase14_seed=0x5eedcafef00dbeef
phase14_ticks=8000
# Pinned on 2026-09-02 from the first run of this script. If any moves,
# record the old and new values in docs/22-decision-log.md and the reason.
phase14_config=0x87ef88945d7672af
phase14_state=0x92ceac70c45d7e3f
phase14_scramble_config=0x23dd738d0390373c
phase14_scramble_state=0x5a7d1f6ea56ef554
phase14_inert_config=0x27a9ff3d97be016e
phase14_inert_state=0x2178b53491694f21
phase13_config=0x252199db7099e9a5
phase13_state=0x5861f0fc8ab02957

# --- clause 1: clean-process replay of the physiology-v2 trace ---------------

target/release/lifesim fixture --ticks "$phase14_ticks" --phase2 --genome2 \
  --physiology-v2 --seed "$phase14_seed" > "$phase14_tmp/first.json"
target/release/lifesim fixture --ticks "$phase14_ticks" --phase2 --genome2 \
  --physiology-v2 --seed "$phase14_seed" > "$phase14_tmp/second.json"
cmp "$phase14_tmp/first.json" "$phase14_tmp/second.json"

for expected in \
  '"fixture_schema_version":10' \
  '"phase":"phase14"' \
  '"physiology_policy":"lifesim-physiology-v2"' \
  "\"config_hash\":\"$phase14_config\"" \
  "\"state_checksum\":\"$phase14_state\""
do
  if ! grep -q "$expected" "$phase14_tmp/first.json"; then
    printf 'phase14 fixture constant: FAIL\n' >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase14_tmp/first.json" >&2
    exit 1
  fi
done

# --- clause 2: the trace is not a control ------------------------------------
#
# A zero in any field below means the fixture stopped pinning that
# mechanism and the checksum above would not say so on its own (evidence
# trap 1): births carry choices, choices carry the preference scoring,
# modules and spend carry the growth billing, and the signal substrate
# rides along so the composed trace cannot lose its floor unnoticed.
# `scrambled_choices` is deliberately absent: the base arm runs the knob
# at zero by design and clause 3 owns it.
for forbidden in \
  '"population":0,' \
  '"births_total":0,' \
  '"modules_grown":0,' \
  '"growth_spent_milli":0,' \
  '"choices":0,' \
  '"signals_emitted":0,'
do
  if grep -q "$forbidden" "$phase14_tmp/first.json"; then
    printf 'phase14 fixture is vacuous: FAIL\n' >&2
    printf 'found %s, so the fixture pins nothing about that mechanism:\n' "$forbidden" >&2
    cat "$phase14_tmp/first.json" >&2
    exit 1
  fi
done
printf 'phase14 clean-process deterministic fixture: PASS\n'
sed -n '1p' "$phase14_tmp/first.json"

# --- clause 3: the arms replay, are distinct, and the scramble counts --------

run_arm() {
  target/release/lifesim fixture --ticks "$phase14_ticks" --phase2 --genome2 \
    --physiology-v2 "$@" --seed "$phase14_seed"
}
run_arm --physiology-v2-scramble > "$phase14_tmp/scramble1.json"
run_arm --physiology-v2-scramble > "$phase14_tmp/scramble2.json"
cmp "$phase14_tmp/scramble1.json" "$phase14_tmp/scramble2.json"
run_arm --physiology-v2-inert > "$phase14_tmp/inert1.json"
run_arm --physiology-v2-inert > "$phase14_tmp/inert2.json"
cmp "$phase14_tmp/inert1.json" "$phase14_tmp/inert2.json"

for pair in \
  "scramble1 $phase14_scramble_config $phase14_scramble_state" \
  "inert1 $phase14_inert_config $phase14_inert_state"
do
  set -- $pair
  if ! grep -q "\"config_hash\":\"$2\"" "$phase14_tmp/$1.json" \
    || ! grep -q "\"state_checksum\":\"$3\"" "$phase14_tmp/$1.json"; then
    printf 'phase14 arm %s constant: FAIL\n' "$1" >&2
    cat "$phase14_tmp/$1.json" >&2
    exit 1
  fi
done
if [ "$phase14_scramble_state" = "$phase14_state" ] \
  || [ "$phase14_inert_state" = "$phase14_state" ]; then
  printf 'phase14 arms are not distinct lineages: FAIL\n' >&2
  exit 1
fi
if grep -q '"scrambled_choices":0,' "$phase14_tmp/scramble1.json"; then
  printf 'phase14 scramble arm never scrambled: FAIL\n' >&2
  cat "$phase14_tmp/scramble1.json" >&2
  exit 1
fi
# The inert arm has both gates off, so it must print as the Phase 13
# schema and count no Phase 14 mechanism at all.
if ! grep -q '"fixture_schema_version":9' "$phase14_tmp/inert1.json"; then
  printf 'phase14 inert arm printed a phase14 schema: FAIL\n' >&2
  cat "$phase14_tmp/inert1.json" >&2
  exit 1
fi
printf 'phase14 scramble and inert arms replay and are distinct lineages: PASS\n'

# --- clause 4: the Phase 13 fixture is untouched -----------------------------

target/release/lifesim fixture --ticks "$phase14_ticks" --phase2 --genome2 --social \
  --seed "$phase14_seed" > "$phase14_tmp/phase13.json"
if ! grep -q "\"config_hash\":\"$phase13_config\"" "$phase14_tmp/phase13.json" \
  || ! grep -q "\"state_checksum\":\"$phase13_state\"" "$phase14_tmp/phase13.json"; then
  printf 'phase14 broke the phase13 fixture: FAIL\n' >&2
  cat "$phase14_tmp/phase13.json" >&2
  exit 1
fi
printf 'phase14 preserves the Phase 13 fixture: PASS\n'
