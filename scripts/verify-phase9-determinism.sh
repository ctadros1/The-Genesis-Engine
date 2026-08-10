#!/bin/sh
# Phase 9 clean-process determinism checks (acceptance criterion C9.7).
#
# Every `lifesim` invocation below is a separate process, so passing here is
# evidence about process-independent replay rather than in-process equality.
#
# Two things this does that verify-phase1 and verify-phase2 do not, and the
# omission there is a gap worth not repeating: it pins the expected constants
# with `grep -q` the way verify-phase5 does, and it asserts the fixture is not
# a control. Those two scripts only `cmp` two runs of the same build, so a
# change that moved a checksum consistently would pass both of them.
set -eu

phase9_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase9_repo/scripts/phase0-env.sh"
cd "$phase9_repo"

cargo build --release --bin lifesim
phase9_tmp=$(mktemp -d)
trap 'rm -rf "$phase9_tmp"' EXIT HUP INT TERM

phase9_seed=0x5eedcafef00dbeef
phase9_ticks=8000
phase9_config=0x9abc0cd47914127f
phase9_state=0x5f0c4e95e4f5170f
phase1_state=0x1e3158a26afd3b39
phase2_state=0xff9dfcff5dffbf42

# --- C9.7 clause 1: clean-process replay of the Phase 9 fixture -------------

target/release/lifesim fixture --ticks "$phase9_ticks" --phase2 --genome2 \
  --seed "$phase9_seed" > "$phase9_tmp/first.json"
target/release/lifesim fixture --ticks "$phase9_ticks" --phase2 --genome2 \
  --seed "$phase9_seed" > "$phase9_tmp/second.json"
cmp "$phase9_tmp/first.json" "$phase9_tmp/second.json"

for expected in \
  "\"config_hash\":\"$phase9_config\"" \
  "\"state_checksum\":\"$phase9_state\""
do
  if ! grep -q "$expected" "$phase9_tmp/first.json"; then
    printf 'phase9 fixture constant: FAIL\n' >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase9_tmp/first.json" >&2
    exit 1
  fi
done

# The horizon has to be long enough that the fixture pins the mechanisms it
# claims to pin. `maturity_age_ticks` is 600 and founders spawn at age 0, so
# at the Phase 1/2 horizon of 500 ticks **nothing has reproduced at all** - a
# 500-tick schema-2 fixture would be silently a control, pinning meiosis,
# structural mutation, and the schema-2 birth path by pinning none of them.
# A zero in any field below means the fixture stopped being evidence, and the
# checksum above would not say so on its own.
#
# `duplications_applied` is checked separately from
# `structural_mutations_applied` because the latter counts point mutations,
# and a point mutation changes no structure.
for forbidden in \
  '"births_total":0,' \
  '"paired_births_total":0,' \
  '"deaths_total":0,' \
  '"structural_mutations_applied":0,' \
  '"duplications_applied":0,' \
  '"structural_mutations_rejected":0}'
do
  if grep -q "$forbidden" "$phase9_tmp/first.json"; then
    printf 'phase9 fixture is vacuous: FAIL\n' >&2
    printf 'found %s, so the fixture pins nothing about that mechanism:\n' "$forbidden" >&2
    cat "$phase9_tmp/first.json" >&2
    exit 1
  fi
done
printf 'phase9 clean-process deterministic fixture: PASS\n'
sed -n '1p' "$phase9_tmp/first.json"

# --- C9.7 clause 4: the schema-1 lineages are untouched ---------------------
#
# Checked here as well as in their own scripts, because the clause that
# breaks when schema 2 leaks out of its config section is this one, and the
# Phase 9 script is where a reader will look for the Phase 9 answer.

target/release/lifesim fixture --ticks 500 --seed "$phase9_seed" \
  > "$phase9_tmp/phase1.json"
target/release/lifesim fixture --ticks 500 --phase2 --seed "$phase9_seed" \
  > "$phase9_tmp/phase2.json"
for pair in "phase1 $phase1_state" "phase2 $phase2_state"; do
  phase=${pair%% *}
  expected=${pair##* }
  if ! grep -q "\"state_checksum\":\"$expected\"" "$phase9_tmp/$phase.json"; then
    printf 'phase9 fixture preservation (%s): FAIL\n' "$phase" >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase9_tmp/$phase.json" >&2
    exit 1
  fi
done
printf 'phase9 preserves the schema-1 fixtures: PASS\n'
printf '  phase1 %s\n  phase2 %s\n' "$phase1_state" "$phase2_state"
