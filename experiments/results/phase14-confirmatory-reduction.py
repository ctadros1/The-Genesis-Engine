#!/usr/bin/env python3
# Phase 14 confirmatory findings reduction, over the committed report
# files in this directory. Every number it prints is recomputable by hand
# from the report lines; the decision rules are the pre-registration's own
# (experiments/phase14-physiology-preregistration.md, committed before the
# campaign ran). Usage:
#   python3 phase14-confirmatory-reduction.py [reports-dir]
# expecting phase14-confirmatory-{development,assortment}.txt.
import os
import statistics as st
import sys
from collections import defaultdict
from math import comb

S = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__))
P = "phase14-confirmatory"
SESOI_MILLI = 10
BAR = 30


def rows(path):
    out = defaultdict(list)
    for line in open(os.path.join(S, path)):
        if line.startswith("world "):
            d = dict(kv.split("=", 1) for kv in line[6:].split() if "=" in kv)
            out[d["condition"]].append(d)
    return out


def binom_tail(k, n, p=0.5):
    return sum(comb(n, i) * p**i * (1 - p) ** (n - i) for i in range(k, n + 1))


def value(row, key):
    return None if row[key] == "none" else int(row[key])


# --- C14.2 primary: A-versus-P on cue 7, the locked form -----------------
ass = rows(f"{P}-assortment.txt")
a_by = {r["seed"]: r for r in ass["A"]}
p_by = {r["seed"]: r for r in ass["P"]}
unusable = []
differences = []
for seed in sorted(a_by):
    a = value(a_by[seed], "dev7")
    p = value(p_by.get(seed, {"dev7": "none"}), "dev7")
    if a is None or p is None:
        unusable.append(seed)
        continue
    differences.append(abs(a) - abs(p))
reaching = sum(1 for d in differences if d >= SESOI_MILLI)
n = len(differences)
print(f"== C14.2 primary: A-vs-P |dev7| contrast, SESOI {SESOI_MILLI} milli, bar {BAR}/50")
print(
    f"  pairs {n} (unusable {len(unusable)}{' ' + ','.join(unusable) if unusable else ''}); "
    f"reaching_absolute_directed {reaching}/{n}; exact binomial tail "
    f"p={binom_tail(reaching, n):.3f}; positive {sum(1 for d in differences if d > 0)}/{n}; "
    f"mean {st.mean(differences):.1f} milli; median {st.median(differences)}"
)
print("  per-cue A-vs-P deltas (reported, never decisive):")
for cue in range(9):
    key = f"dev{cue}"
    deltas = []
    for seed in sorted(a_by):
        a = value(a_by[seed], key)
        p = value(p_by.get(seed, {key: "none"}), key)
        if a is not None and p is not None:
            deltas.append(a - p)
    if deltas:
        print(
            f"    cue{cue}: mean {st.mean(deltas):.1f}, median {st.median(deltas)}, "
            f"min {min(deltas)}, max {max(deltas)}"
        )
used = [int(r["used"]) for r in ass["A"]]
single = [int(r["single"]) for r in ass["A"]]
print(
    f"  A informing choices: median {int(st.median(used))}, min {min(used)}; "
    f"single-candidate median {int(st.median(single))} (excluded and counted)"
)
identical = sum(
    1
    for seed in a_by
    if seed in p_by
    and all(a_by[seed][f"dev{c}"] == p_by[seed][f"dev{c}"] for c in range(9))
    and a_by[seed]["choices"] == p_by[seed]["choices"]
)
print(
    f"  neutral-equivalence signature: {identical}/{n} pairs with identical "
    f"choice counts and every deviation equal (preference never left neutral there)"
)

# --- C14.1: the within-A mortality directed count ------------------------
dev = rows(f"{P}-development.txt")
gaps = []
mort_unusable = []
for r in dev["A"]:
    j = value(r, "juvenile_mortality_micro")
    a = value(r, "adult_mortality_micro")
    if j is None or a is None:
        mort_unusable.append(r["seed"])
        continue
    gaps.append(j - a)
mort_reaching = sum(1 for g in gaps if g > 0)
print(f"\n== C14.1 mortality: juvenile > adult within-world, bar {BAR}/50 (multiplier 1.0)")
print(
    f"  worlds {len(gaps)} (unusable {len(mort_unusable)}); reaching {mort_reaching}/{len(gaps)}; "
    f"exact binomial tail p={binom_tail(mort_reaching, len(gaps)):.3f}; "
    f"gap median {st.median(gaps)} micro, min {min(gaps)}, max {max(gaps)}"
)
completions = [int(r["completions"]) for r in dev["A"]]
jspeed = [value(r, "juvenile_speed_milli") for r in dev["A"]]
aspeed = [value(r, "adult_speed_milli") for r in dev["A"]]
speed_pairs = [(j, a) for j, a in zip(jspeed, aspeed) if j is not None and a is not None]
print(
    f"  juvenile state at scale: completions median {int(st.median(completions))}; "
    f"speed (direction-free, mechanism in the pre-registration): juvenile median "
    f"{st.median([j for j, _ in speed_pairs])} vs adult {st.median([a for _, a in speed_pairs])} "
    f"milli-m/tick; juveniles faster in {sum(1 for j, a in speed_pairs if j > a)}/"
    f"{len(speed_pairs)} worlds"
)
for arm in ["B"]:
    zeros = all(int(r["juvenile_obs"]) == 0 for r in dev[arm])
    print(f"  {arm}-arm juvenile columns zero by construction: {zeros}")
