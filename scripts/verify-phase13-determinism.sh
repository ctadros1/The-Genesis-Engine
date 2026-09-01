#!/bin/sh
# Phase 13 social-channel clean-process determinism checks (C13.12's fixture
# clause), modelled on verify-phase12.
#
# Every `lifesim` invocation below is a separate process, so passing here is
# evidence about process-independent replay rather than in-process equality.
# The fixture is the `--social` trace: the Phase 12 artifact trace's world
# plus the social section, founders additionally scripted to emit on signal
# channel 0 every tick and carrying one plastic rule-5 edge, so perception,
# the field, emission billing and the observational rule all run inside an
# affordable horizon (see `social_trace_config` in `crates/sim-cli`).
#
# Four clauses:
#   1. clean-process replay of the trace, two processes, constants pinned;
#   2. the trace is not a control: every mechanism's count is refused at zero;
#   3. conditions S (rule 5 withheld), D (scrambled delivery) and the
#      corruption arm each replay, are distinct lineages, and show their own
#      mechanism's counter - S's rule5_updates is asserted EXACTLY ZERO,
#      which is ADR-0029's by-counter ablation verification;
#   4. the Phase 1, 2, 9 and 12 fixtures are untouched with the section
#      disabled (C13.12; Phase 11's is checked by its own script).
set -eu

phase13_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase13_repo/scripts/phase0-env.sh"
cd "$phase13_repo"

cargo build --release --bin lifesim
phase13_tmp=$(mktemp -d)
trap 'rm -rf "$phase13_tmp"' EXIT HUP INT TERM

phase13_seed=0x5eedcafef00dbeef
phase13_ticks=8000
# Pinned on 2026-09-01 from the first run of this script. If either moves,
# record the old and new values in docs/22-decision-log.md and the reason.
phase13_config=0x252199db7099e9a5
phase13_state=0x5861f0fc8ab02957
phase12_config=0x21405a5c0591ceeb
phase12_state=0x24defb6052eb9d42
phase9_config=0x9abc0cd47914127f
phase9_state=0x5f0c4e95e4f5170f
phase1_state=0x1e3158a26afd3b39
phase2_state=0xff9dfcff5dffbf42

# --- clause 1: clean-process replay of the social trace ----------------------

target/release/lifesim fixture --ticks "$phase13_ticks" --phase2 --genome2 --social \
  --seed "$phase13_seed" > "$phase13_tmp/first.json"
target/release/lifesim fixture --ticks "$phase13_ticks" --phase2 --genome2 --social \
  --seed "$phase13_seed" > "$phase13_tmp/second.json"
cmp "$phase13_tmp/first.json" "$phase13_tmp/second.json"

for expected in \
  '"fixture_schema_version":9' \
  '"phase":"phase13"' \
  '"social_policy":"lifesim-social-v1"' \
  '"artifact_policy":"lifesim-artifact-v2"' \
  '"plasticity_policy":"lifesim-plasticity-v2"' \
  '"channel_registry":3' \
  '"rule_registry":2' \
  "\"config_hash\":\"$phase13_config\"" \
  "\"state_checksum\":\"$phase13_state\""
do
  if ! grep -q "$expected" "$phase13_tmp/first.json"; then
    printf 'phase13 fixture constant: FAIL\n' >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase13_tmp/first.json" >&2
    exit 1
  fi
done

# --- clause 2: the trace is not a control ------------------------------------
#
# A zero in any field below means the fixture stopped pinning that mechanism
# and the checksum above would not say so on its own (evidence trap 1). The
# list is every mechanism the social channel added - emission, billing, the
# committed field, the contact record, rule 5 - plus the artifact substrate
# it runs on, so the composed trace cannot lose its floor unnoticed.
# `corruption_draws` and `scrambled_deliveries` are deliberately absent:
# the base arm runs both knobs at zero by design and clause 3 owns them.
for forbidden in \
  '"population":0,' \
  '"births_total":0,' \
  '"binding_applied":0,' \
  '"signals_emitted":0,' \
  '"signal_cost_milli":0,' \
  '"rule5_updates":0,' \
  '"field_nonzero_cells":0,' \
  '"contact_committed":0,' \
  '"struck_terrain":0,' \
  '"picked_up":0,' \
  '"consumed_events":0,' \
  '"created_carcass":0,'
do
  if grep -q "$forbidden" "$phase13_tmp/first.json"; then
    printf 'phase13 fixture is vacuous: FAIL\n' >&2
    printf 'found %s, so the fixture pins nothing about that mechanism:\n' "$forbidden" >&2
    cat "$phase13_tmp/first.json" >&2
    exit 1
  fi
done
printf 'phase13 clean-process deterministic fixture: PASS\n'
sed -n '1p' "$phase13_tmp/first.json"

# --- clause 3: the condition arms replay and are distinct --------------------

run_arm() {
  target/release/lifesim fixture --ticks 2000 --phase2 --genome2 --social "$@" \
    --seed "$phase13_seed"
}
run_arm > "$phase13_tmp/base.json"
run_arm --social-strict > "$phase13_tmp/s1.json"
run_arm --social-strict > "$phase13_tmp/s2.json"
cmp "$phase13_tmp/s1.json" "$phase13_tmp/s2.json"
run_arm --social-scramble > "$phase13_tmp/d1.json"
run_arm --social-scramble > "$phase13_tmp/d2.json"
cmp "$phase13_tmp/d1.json" "$phase13_tmp/d2.json"
run_arm --social-corrupt > "$phase13_tmp/c1.json"
run_arm --social-corrupt > "$phase13_tmp/c2.json"
cmp "$phase13_tmp/c1.json" "$phase13_tmp/c2.json"

field() { sed -n "s/.*\"$2\":\"\{0,1\}\([^\",}]*\)\"\{0,1\}.*/\1/p" "$1"; }
base_hash=$(field "$phase13_tmp/base.json" config_hash)
s_hash=$(field "$phase13_tmp/s1.json" config_hash)
d_hash=$(field "$phase13_tmp/d1.json" config_hash)
c_hash=$(field "$phase13_tmp/c1.json" config_hash)
for pair in "$base_hash $s_hash" "$base_hash $d_hash" "$base_hash $c_hash" \
  "$s_hash $d_hash" "$s_hash $c_hash" "$d_hash $c_hash"
do
  left=${pair%% *}
  right=${pair##* }
  if [ "$left" = "$right" ]; then
    printf 'phase13 conditions share a config hash: FAIL (%s)\n' "$pair" >&2
    exit 1
  fi
done
# Condition S: the observational rule is withheld and the ablation is
# verified by the mechanism's own counter, not the config flag (ADR-0029).
if ! grep -q '"rule5_updates":0,' "$phase13_tmp/s1.json"; then
  printf 'phase13 condition S ran the withheld rule: FAIL\n' >&2
  cat "$phase13_tmp/s1.json" >&2
  exit 1
fi
if grep -q '"rule_registry":2' "$phase13_tmp/s1.json"; then
  printf 'phase13 condition S still offers rule-registry 2: FAIL\n' >&2
  exit 1
fi
# Condition D scrambles and the corruption arm draws; each shows its own
# counter and the base arm shows neither (clause 2 left them unasserted).
if grep -q '"scrambled_deliveries":0,' "$phase13_tmp/d1.json"; then
  printf 'phase13 condition D never scrambled: FAIL\n' >&2
  exit 1
fi
if grep -q '"corruption_draws":0,' "$phase13_tmp/c1.json"; then
  printf 'phase13 corruption arm never drew: FAIL\n' >&2
  exit 1
fi
if ! grep -q '"scrambled_deliveries":0,' "$phase13_tmp/base.json"; then
  printf 'phase13 base arm scrambled: FAIL\n' >&2
  exit 1
fi
if ! grep -q '"corruption_draws":0,' "$phase13_tmp/base.json"; then
  printf 'phase13 base arm drew corruption: FAIL\n' >&2
  exit 1
fi
printf 'phase13 conditions S, D and corruption replay and are distinct lineages: PASS\n'

# --- clause 4: the earlier fixtures are untouched (C13.12) -------------------

target/release/lifesim fixture --ticks 500 --seed "$phase13_seed" > "$phase13_tmp/phase1.json"
target/release/lifesim fixture --ticks 500 --phase2 --seed "$phase13_seed" > "$phase13_tmp/phase2.json"
target/release/lifesim fixture --ticks 8000 --phase2 --genome2 --seed "$phase13_seed" \
  > "$phase13_tmp/phase9.json"
target/release/lifesim fixture --ticks 8000 --phase2 --genome2 --artifact \
  --seed "$phase13_seed" > "$phase13_tmp/phase12.json"
for pair in "phase1 $phase1_state" "phase2 $phase2_state" "phase9 $phase9_state" \
  "phase12 $phase12_state"
do
  phase=${pair%% *}
  expected=${pair##* }
  if ! grep -q "\"state_checksum\":\"$expected\"" "$phase13_tmp/$phase.json"; then
    printf 'phase13 fixture preservation (%s): FAIL\n' "$phase" >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase13_tmp/$phase.json" >&2
    exit 1
  fi
done
if ! grep -q "\"config_hash\":\"$phase12_config\"" "$phase13_tmp/phase12.json"; then
  printf 'phase13 fixture preservation (phase12 config): FAIL\n' >&2
  exit 1
fi
if ! grep -q '"channel_registry":2' "$phase13_tmp/phase12.json"; then
  printf 'phase13: a world without the section reports registry version 2: FAIL\n' >&2
  exit 1
fi
printf 'phase13 preserves the Phase 1, 2, 9 and 12 fixtures: PASS\n'
printf '  phase1 %s\n  phase2 %s\n  phase9 %s\n  phase12 %s\n' \
  "$phase1_state" "$phase2_state" "$phase9_state" "$phase12_state"
printf 'phase13: the Phase 11 fixture is checked by scripts/verify-phase11-determinism.sh\n'
