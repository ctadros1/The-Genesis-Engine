#!/usr/bin/env python3
# Phase 13 confirmatory findings reduction, over the committed report files
# in this directory. Every number it prints is recomputable by hand from
# the report lines; the decision rules are the two pre-registrations' own
# (experiments/phase13-social-preregistration.md, commit 37983b7, and
# experiments/phase13-conflict-addendum-preregistration.md, commit f66b0e2).
# Usage: python3 phase13-social-confirmatory-reduction.py [reports-dir]
import os
import statistics as st
import sys
from collections import defaultdict
from math import comb

S = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__))
P = "phase13-social-confirmatory"

def rows(path, prefix="world "):
    out = []
    for line in open(os.path.join(S, path)):
        if line.startswith(prefix):
            out.append(dict(kv.split("=", 1) for kv in line[len(prefix):].split() if "=" in kv))
    return out

def by_arm(rs):
    d = defaultdict(list)
    for r in rs:
        d[r["condition"]].append(r)
    return d

def binom_tail(k, n, p=0.5):
    return sum(comb(n, i) * p**i * (1 - p) ** (n - i) for i in range(k, n + 1))

# --- C13.1 / C13.4: the pre-registered contrast lines, verbatim ----------
print("== C13.1 / C13.4 contrast lines (lifesim social-contrast output, verbatim)")
for name in ["AC", "AD", "AS"]:
    for line in open(os.path.join(S, f"{P}-contrast-{name}.txt")):
        if line.startswith("contrast "):
            print(f"  A-vs-{name[1]}: {line[9:].rstrip()}")

# --- The census at epoch 25 ---------------------------------------------
cen = by_arm(rows(f"{P}-census-e25.txt"))
med = lambda rs, k: int(st.median(int(r[k]) for r in rs))
tot = lambda rs, k: sum(int(r[k]) for r in rs)
print("\n== Census medians at epoch 25 (n=30 worlds per arm)")
print(f"  {'arm':>5} {'pop':>5} {'naive':>5} {'hearers':>7} {'speakers':>8} "
      f"{'emit&in':>7} {'r5expr':>7} {'emitted':>9} {'arrived(sum)':>12}")
for arm in ["A", "B", "C", "D", "S", "A8k", "A16k", "A32k"]:
    rs = cen[arm]
    print(f"  {arm:>5} {med(rs,'population'):>5} {med(rs,'naive'):>5} "
          f"{med(rs,'hearers'):>7} {med(rs,'speakers'):>8} {med(rs,'emit_and_in'):>7} "
          f"{med(rs,'rule5_expressed_organisms'):>7} {med(rs,'signals_emitted'):>9} "
          f"{tot(rs,'arrived'):>12}")
for arm in ["A", "C", "D", "S"]:
    rs = cen[arm]
    print(f"  {arm} minima: hearers {min(int(r['hearers']) for r in rs)}, "
          f"speakers {min(int(r['speakers']) for r in rs)}, "
          f"naive {min(int(r['naive']) for r in rs)}")

# --- C13.2: the F-curve --------------------------------------------------
fid = by_arm(rows(f"{P}-fidelity.txt"))
print("\n== C13.2 F-curve (median fidelity_delta_milli; corruption 0/8192/16384/32768)")
for arm in ["A", "A8k", "A16k", "A32k", "B", "C", "D", "S"]:
    ds = [int(r["delta_milli"]) for r in fid[arm] if r["delta_milli"] != "none"]
    none = sum(1 for r in fid[arm] if r["delta_milli"] == "none")
    exp = [int(r["exposed"]) for r in fid[arm]]
    tag = "curve" if arm.startswith("A") else "no-channel context" if arm in ("B", "C") else "context"
    print(f"  {arm:>5}: median {st.median(ds):>5} milli over {len(ds)} worlds ({none} none), "
          f"IQR [{sorted(ds)[len(ds)//4]},{sorted(ds)[3*len(ds)//4]}], "
          f"median exposed {int(st.median(exp))}  [{tag}]")
rates = sorted(int(r["signals_emitted"]) / max(int(r["hearers"]), 1) for r in cen["A"])
r_med = rates[(len(rates) - 1) // 2]
print(f"  persistence>1 line: median deliveries per hearer (A, epoch 25) R = {r_med:.0f}; "
      f"line at 1000/R = {1000 / r_med:.3f} milli (context only, no borrowed threshold)")

# --- C13.3: tradition counts --------------------------------------------
trad = by_arm(rows(f"{P}-tradition.txt"))
a_hits = sum(1 for r in trad["A"] if int(r["findings"]) > 0)
c12 = sorted(trad["C"], key=lambda r: int(r["seed"], 16))[:12]
c_hits = sum(1 for r in c12 if int(r["findings"]) > 0)
t = lambda arm, k: sum(int(r[k]) for r in trad[arm])
print(f"\n== C13.3 traditions: A {a_hits}/30 worlds with findings (bar 15/30); "
      f"C {c_hits}/12 (bar 0/12)")
print(f"  A totals: candidates {t('A','candidates')}, rejected_end {t('A','rejected_end')}, "
      f"turnover {t('A','rejected_turnover')}, no_cohort {t('A','rejected_no_cohort')}, "
      f"control {t('A','rejected_control')}, findings {t('A','findings')}")
print(f"  findings per arm (all 8): { {a: sum(int(r['findings']) for r in rs) for a, rs in trad.items()} }")

# --- C13.4: the S-versus-A re-read --------------------------------------
s_hits = sum(1 for r in trad["S"] if int(r["findings"]) > 0)
print(f"\n== C13.4 S-arm re-read: tradition findings S {s_hits}/30 (A {a_hits}/30); "
      f"arrival contrast printed above")

# --- C13.8: chains vs null ----------------------------------------------
com = by_arm(rows(f"{P}-communities.txt"))
for arm in ["A", "C"]:
    hits = sum(1 for r in com[arm] if int(r["chains"]) > int(r["null_p95"]))
    ch = [int(r["chains"]) for r in com[arm]]
    print(f"\n== C13.8 {arm}: {hits}/30 worlds with chains > null_p95"
          + (" (bar 20/30 under A)" if arm == "A" else "")
          + f"; chains median {st.median(ch)}, max {max(ch)}")

# --- C13.9: paired A-vs-C co-present factor contrast --------------------
def factors(arm):
    out = {}
    for r in com[arm]:
        cw = int(r["copresent_within_rate_micro"])
        cb = int(r["copresent_between_rate_micro"])
        out[r["seed"]] = None if cw == 0 else cb * 1000 // cw
    return out
fa, fc = factors("A"), factors("C")
pairs = [(s, fa[s], fc[s]) for s in fa if s in fc and fa[s] is not None and fc[s] is not None]
diffs = [a - c for _, a, c in pairs]
reach = sum(1 for d in diffs if d >= 500)
print(f"\n== C13.9 primary: A-vs-C co-present factor contrast, SESOI 500 milli absolute increase")
print(f"  pairs {len(pairs)} (unusable {30 - len(pairs)}); reaching_absolute_directed {reach}/30 "
      f"(bar 20/30); exact binomial tail p={binom_tail(reach, len(pairs)):.3f}; "
      f"positive {sum(1 for d in diffs if d > 0)}/{len(pairs)}; "
      f"mean diff {int(st.mean(diffs))} milli; median {int(st.median(diffs))}")
for arm, f in [("A", fa), ("C", fc)]:
    vals = [v for v in f.values() if v is not None]
    print(f"  {arm} factors: median {st.median(vals)} milli, "
          f"{sum(1 for v in vals if v >= 1500)}/{len(vals)} worlds >= 1500 (descriptive)")

# --- C13.10: descriptives -----------------------------------------------
print(f"\n== C13.10 descriptives (expected null, per plan)")
for arm in ["A", "C"]:
    adv, dis = tot(com[arm], "advantage"), tot(com[arm], "disadvantage")
    even, co, tg = tot(com[arm], "even"), tot(com[arm], "coalitions"), tot(com[arm], "targets")
    print(f"  {arm}: between-attacks with local advantage {adv}, disadvantage {dis}, even {even}; "
          f"coalition targets {co} of {tg} attacked ({co*1000//max(tg,1)} milli)")
