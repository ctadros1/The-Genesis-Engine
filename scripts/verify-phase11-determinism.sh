#!/bin/sh
# Phase 11 clean-process determinism checks (acceptance criterion C11.5).
#
# C11.5 asks for a 10^6-tick single-organism plasticity trace that reproduces
# bit-identically **across clean processes**. Every `lifesim` invocation below
# is a separate process, so passing here is evidence about process-independent
# replay of the fixed-point learned-state path rather than in-process equality
# - which is what an `#[ignore]`d long-run test would have measured, and is
# not what the criterion says.
#
# The trace is `lifesim fixture --plasticity`: one immortal, sterile organism
# whose two edges are plastic. Immortal and sterile on purpose, and the
# reasons are in `plasticity_trace_config`: no reproduction means the network
# never changes, so this is one individual's lifetime rather than a lineage's,
# and no energy cost means the organism cannot starve at tick 3,000 and leave
# 997,000 ticks of empty world being reported as a determinism pass.
#
# Modelled on verify-phase9-determinism.sh, including the two things that
# script does and verify-phase1/2 do not: it pins the expected constants with
# `grep -q` rather than only comparing two runs of the same build, and it
# asserts the fixture is not a control.
set -eu

phase11_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase11_repo/scripts/phase0-env.sh"
cd "$phase11_repo"

cargo build --release --bin lifesim
phase11_tmp=$(mktemp -d)
trap 'rm -rf "$phase11_tmp"' EXIT HUP INT TERM

phase11_seed=0x5eedcafef00dbeef
phase11_ticks=1000000
phase11_config=0xae34cd2b6f7a3e13
phase11_state=0x53b354bd94e82bcf
# The mid-run horizon exists only to prove the trace is still moving at the
# end; see the accumulation clause below.
phase11_short_ticks=100000
phase11_short_learned=100
phase11_learned=171
phase9_config=0x9abc0cd47914127f
phase9_state=0x5f0c4e95e4f5170f

trace() {
  target/release/lifesim fixture --ticks "$1" --phase2 --genome2 --plasticity \
    --seed "$phase11_seed"
}

# --- C11.5 clause 1: clean-process replay of the 10^6-tick trace ------------

trace "$phase11_ticks" > "$phase11_tmp/first.json"
trace "$phase11_ticks" > "$phase11_tmp/second.json"
cmp "$phase11_tmp/first.json" "$phase11_tmp/second.json"

for expected in \
  "\"config_hash\":\"$phase11_config\"" \
  "\"state_checksum\":\"$phase11_state\"" \
  "\"mean_abs_learned_milli\":$phase11_learned,"
do
  if ! grep -q "$expected" "$phase11_tmp/first.json"; then
    printf 'phase11 trace constant: FAIL\n' >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase11_tmp/first.json" >&2
    exit 1
  fi
done

# --- C11.5 clause 2: the trace is not a control -----------------------------
#
# A state checksum reproduces just as happily for a run in which the organism
# died on tick 3, the learn phase never fired, or every delta stayed at zero:
# it would simply be a different constant that two runs still agree on. Each
# forbidden string below is one of those worlds.
#
# `plasticity_anomalies_total` is required to be zero rather than nonzero. It
# counts faults and clamp saturations, and a trace that saturates has stopped
# measuring accumulation and started measuring a constant - the runaway
# plasticity this phase's risk table names.
for forbidden in \
  '"population":0,' \
  '"plastic_edges_total":0,' \
  '"plasticity_updates_total":0,' \
  '"mean_abs_learned_milli":0,'
do
  if grep -q "$forbidden" "$phase11_tmp/first.json"; then
    printf 'phase11 trace is vacuous: FAIL\n' >&2
    printf 'found %s, so the trace pins nothing about that mechanism:\n' "$forbidden" >&2
    cat "$phase11_tmp/first.json" >&2
    exit 1
  fi
done
for required in \
  '"plasticity_anomalies_total":0,' \
  '"controller_faults_total":0}'
do
  if ! grep -q "$required" "$phase11_tmp/first.json"; then
    printf 'phase11 trace saturated or faulted: FAIL\n' >&2
    printf 'expected %s in:\n' "$required" >&2
    cat "$phase11_tmp/first.json" >&2
    exit 1
  fi
done

# --- C11.5 clause 3: the trace is still accumulating at the horizon ---------
#
# The sharpest way this fixture could quietly stop being evidence. A plastic
# edge with a decay term settles at an equilibrium where the decay cancels the
# delta, and once it is there every further tick repeats one step: the
# checksum still reproduces, the counters still climb, and the last 90% of the
# run tests nothing that the first 10% did not. That is not hypothetical - it
# is what the first cut of this fixture did, with the input bound to
# `energy_fraction`, which is constant in a world with no energy costs.
#
# The binding was moved to `age_fraction`, which sweeps monotonically and
# never repeats, so the equilibrium keeps moving. This clause is what says so:
# the learned magnitude at 100,000 ticks must differ from the one at
# 1,000,000.
trace "$phase11_short_ticks" > "$phase11_tmp/short.json"
if ! grep -q "\"mean_abs_learned_milli\":$phase11_short_learned," "$phase11_tmp/short.json"; then
  printf 'phase11 mid-run constant: FAIL\n' >&2
  cat "$phase11_tmp/short.json" >&2
  exit 1
fi
if [ "$phase11_short_learned" = "$phase11_learned" ]; then
  printf 'phase11 trace reached a fixed point: FAIL\n' >&2
  printf 'the learned magnitude at %s ticks equals the one at %s, so the\n' \
    "$phase11_short_ticks" "$phase11_ticks" >&2
  printf 'trace stopped accumulating and the horizon is decorative\n' >&2
  exit 1
fi
printf 'phase11 clean-process deterministic plasticity trace: PASS\n'
printf '  learned magnitude %s milli at %s ticks, %s milli at %s ticks\n' \
  "$phase11_short_learned" "$phase11_short_ticks" \
  "$phase11_learned" "$phase11_ticks"
sed -n '1p' "$phase11_tmp/first.json"

# --- C11.8 clause: the schema-2 lineage is untouched ------------------------
#
# Checked here as well as in verify-phase9, because the clause that breaks
# when plasticity leaks out of its config section is that one, and the Phase
# 11 script is where a reader will look for the Phase 11 answer.

target/release/lifesim fixture --ticks 8000 --phase2 --genome2 \
  --seed "$phase11_seed" > "$phase11_tmp/phase9.json"
for expected in \
  "\"config_hash\":\"$phase9_config\"" \
  "\"state_checksum\":\"$phase9_state\""
do
  if ! grep -q "$expected" "$phase11_tmp/phase9.json"; then
    printf 'phase11 preserves the phase9 fixture: FAIL\n' >&2
    printf 'expected %s in:\n' "$expected" >&2
    cat "$phase11_tmp/phase9.json" >&2
    exit 1
  fi
done
printf 'phase11 preserves the phase9 fixture: PASS\n'
printf '  phase9 config %s state %s\n' "$phase9_config" "$phase9_state"
