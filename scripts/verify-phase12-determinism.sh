#!/bin/sh
# Phase 12 artifact-half clean-process determinism checks (C12.4's fixture
# clause and C12.8), modelled on verify-phase9 and verify-phase11.
#
# Every `lifesim` invocation below is a separate process, so passing here is
# evidence about process-independent replay rather than in-process equality.
# The fixture is the `--artifact` trace: a pinned schema-2 world whose
# founders are scripted to strike, pick up, place, drop and combine from tick
# one, so every object mechanism runs inside an affordable horizon (see
# `artifact_trace_config` in `crates/sim-cli/src/main.rs`).
#
# Four clauses:
#   1. clean-process replay of the trace, two processes, constants pinned;
#   2. the trace is not a control: every mechanism's count is refused at zero;
#   3. condition B (ephemeral) and condition C (inert) each replay and each
#      differ from A - so the campaign's control arms are distinct lineages,
#      not A wearing a flag;
#   4. the four earlier fixtures and the Phase 12 mutable-world lineage are
#      untouched with the section disabled (C12.8).
set -eu

phase12_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase12_repo/scripts/phase0-env.sh"
cd "$phase12_repo"

cargo build --release --bin lifesim
phase12_tmp=$(mktemp -d)
trap 'rm -rf "$phase12_tmp"' EXIT HUP INT TERM

phase12_seed=0x5eedcafef00dbeef
phase12_ticks=8000
# Pinned on 2026-08-16 from the first run of this script. If either moves,
# record the old and new values in docs/22-decision-log.md and the reason.
phase12_config=0xc64259e739b525d4
phase12_state=0x853d257398a2718c
phase9_config=0x9abc0cd47914127f
phase9_state=0x5f0c4e95e4f5170f
phase1_state=0x1e3158a26afd3b39
phase2_state=0xff9dfcff5dffbf42

# --- clause 1: clean-process replay of the artifact trace ---------------------

target/release/lifesim fixture --ticks "$phase12_ticks" --phase2 --genome2 --artifact \
  --seed "$phase12_seed" > "$phase12_tmp/first.json"
target/release/lifesim fixture --ticks "$phase12_ticks" --phase2 --genome2 --artifact \
  --seed "$phase12_seed" > "$phase12_tmp/second.json"
cmp "$phase12_tmp/first.json" "$phase12_tmp/second.json"

for expected in \
  '"fixture_schema_version":8' \
  '"phase":"phase12"' \
  '"artifact_policy":"lifesim-artifact-v1"' \
  '"material_policy":"lifesim-material-v1"' \
  '"channel_registry":2' \
  "\"config_hash\":\"$phase12_config\"" \
  "\"state_checksum\":\"$phase12_state\""
do
  if ! grep -q "$expected" "$phase12_tmp/first.json"; then
    printf 'phase12 fixture constant: FAIL\n' >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase12_tmp/first.json" >&2
    exit 1
  fi
done

# --- clause 2: the trace is not a control ------------------------------------
#
# A zero in any field below means the fixture stopped pinning that mechanism
# and the checksum above would not say so on its own (evidence trap 1). The
# list is every mechanism the artifact half added: extraction, pick-up,
# drop, placement, strikes on objects, fracture, combination, consumption,
# carcass objects, decay, refusals (a cap that never binds is a cap nobody
# tested), births (so the bind operator has something to act on) and the
# bind operator itself. `objects_depth2` is deliberately not in the list: a
# depth-two composite is a chance event of the script, not a mechanism, and
# refusing its zero would make the fixture depend on luck.
for forbidden in \
  '"population":0,' \
  '"births_total":0,' \
  '"binding_applied":0,' \
  '"objects_total":0,' \
  '"created_extracted":0,' \
  '"created_fractured":0,' \
  '"created_combined":0,' \
  '"created_carcass":0,' \
  '"picked_up":0,' \
  '"dropped":0,' \
  '"placed":0,' \
  '"struck_objects":0,' \
  '"struck_terrain":0,' \
  '"fractured":0,' \
  '"combined":0,' \
  '"consumed_events":0,' \
  '"decayed_away":0,' \
  '"refusals":0,' \
  '"cap_refusals":0,'
do
  if grep -q "$forbidden" "$phase12_tmp/first.json"; then
    printf 'phase12 fixture is vacuous: FAIL\n' >&2
    printf 'found %s, so the fixture pins nothing about that mechanism:\n' "$forbidden" >&2
    cat "$phase12_tmp/first.json" >&2
    exit 1
  fi
done
printf 'phase12 clean-process deterministic fixture: PASS\n'
sed -n '1p' "$phase12_tmp/first.json"

# --- clause 3: the two control conditions replay and are distinct -----------

target/release/lifesim fixture --ticks 2000 --phase2 --genome2 --artifact --artifact-ephemeral \
  --seed "$phase12_seed" > "$phase12_tmp/b1.json"
target/release/lifesim fixture --ticks 2000 --phase2 --genome2 --artifact --artifact-ephemeral \
  --seed "$phase12_seed" > "$phase12_tmp/b2.json"
cmp "$phase12_tmp/b1.json" "$phase12_tmp/b2.json"
target/release/lifesim fixture --ticks 2000 --phase2 --genome2 --artifact --artifact-inert \
  --seed "$phase12_seed" > "$phase12_tmp/c1.json"
target/release/lifesim fixture --ticks 2000 --phase2 --genome2 --artifact --artifact-inert \
  --seed "$phase12_seed" > "$phase12_tmp/c2.json"
cmp "$phase12_tmp/c1.json" "$phase12_tmp/c2.json"
target/release/lifesim fixture --ticks 2000 --phase2 --genome2 --artifact \
  --seed "$phase12_seed" > "$phase12_tmp/a.json"
field() { sed -n "s/.*\"$2\":\"\{0,1\}\([^\",}]*\)\"\{0,1\}.*/\1/p" "$1"; }
a_hash=$(field "$phase12_tmp/a.json" config_hash)
b_hash=$(field "$phase12_tmp/b1.json" config_hash)
c_hash=$(field "$phase12_tmp/c1.json" config_hash)
a_state=$(field "$phase12_tmp/a.json" state_checksum)
b_state=$(field "$phase12_tmp/b1.json" state_checksum)
c_state=$(field "$phase12_tmp/c1.json" state_checksum)
if [ "$a_hash" = "$b_hash" ] || [ "$a_hash" = "$c_hash" ] || [ "$b_hash" = "$c_hash" ]; then
  printf 'phase12 conditions share a config hash: FAIL (%s %s %s)\n' "$a_hash" "$b_hash" "$c_hash" >&2
  exit 1
fi
if [ "$a_state" = "$b_state" ] || [ "$a_state" = "$c_state" ]; then
  printf 'phase12 a control condition reproduces condition A: FAIL\n' >&2
  exit 1
fi
# Condition C: no action creates anything (carcass objects are the ecology,
# not an action, and exist under C as under A), yet the actions fire and are
# counted, so C's rate is a firing rate and not a refusal rate.
for expected in '"created_extracted":0,' '"created_combined":0,' '"created_fractured":0,'; do
  if ! grep -q "$expected" "$phase12_tmp/c1.json"; then
    printf 'phase12 inert condition let an action create an object: FAIL\n' >&2
    cat "$phase12_tmp/c1.json" >&2
    exit 1
  fi
done
for forbidden in '"struck_terrain":0,' '"picked_up":0,' '"combined":0,'; do
  if grep -q "$forbidden" "$phase12_tmp/c1.json"; then
    printf 'phase12 inert condition did not fire: FAIL\n' >&2
    cat "$phase12_tmp/c1.json" >&2
    exit 1
  fi
done
printf 'phase12 conditions B and C replay and are distinct lineages: PASS\n'
printf '  A %s  B %s  C %s\n' "$a_state" "$b_state" "$c_state"

# --- clause 4: the earlier fixtures are untouched (C12.8) -------------------

target/release/lifesim fixture --ticks 500 --seed "$phase12_seed" > "$phase12_tmp/phase1.json"
target/release/lifesim fixture --ticks 500 --phase2 --seed "$phase12_seed" > "$phase12_tmp/phase2.json"
target/release/lifesim fixture --ticks 8000 --phase2 --genome2 --seed "$phase12_seed" \
  > "$phase12_tmp/phase9.json"
for pair in "phase1 $phase1_state" "phase2 $phase2_state" "phase9 $phase9_state"; do
  phase=${pair%% *}
  expected=${pair##* }
  if ! grep -q "\"state_checksum\":\"$expected\"" "$phase12_tmp/$phase.json"; then
    printf 'phase12 fixture preservation (%s): FAIL\n' "$phase" >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase12_tmp/$phase.json" >&2
    exit 1
  fi
done
if ! grep -q "\"config_hash\":\"$phase9_config\"" "$phase12_tmp/phase9.json"; then
  printf 'phase12 fixture preservation (phase9 config): FAIL\n' >&2
  exit 1
fi
if ! grep -q '"channel_registry":1' "$phase12_tmp/phase9.json"; then
  printf 'phase12: a world without the section reports registry version 1: FAIL\n' >&2
  exit 1
fi
printf 'phase12 preserves the Phase 1, 2 and 9 fixtures: PASS\n'
printf '  phase1 %s\n  phase2 %s\n  phase9 %s\n' "$phase1_state" "$phase2_state" "$phase9_state"
printf 'phase12: the Phase 11 fixture is checked by scripts/verify-phase11-determinism.sh\n'
