#!/usr/bin/env python3
"""Phase 20 lineage reduction (C20.1, C20.2, C20.3).

Reads a campaign output directory's manifest (per-run births, population,
materialized, nonviable_bodies, refused_node_budget and the four
pair_rejected_* counters) and a saved `lifesim lineage --manifest` report
(lineage-report 1, index lifesim-lineage-index-v1), refuses anything
missing (counted, never silent), and prints per world and per arm:

  multi_total, multi_born, multi_parents, multi_offspring_total,
  second_generation, second-generation per 10,000 births,
  multi vs matched-cohort lifespans with completed/censored counts,
  compositions and added types, the rejection counters, the entity-cap
  gate.

With ONE arm (Branches B/C/D-alone): the pooled second-generation rate per
10,000 births with its exact one-sided 97.5 percent upper bound
(Clopper-Pearson), compared to the pre-registered SESOI; the count of
worlds with any second-generation organism.
With TWO arms (Branch A): seed-paired directed count of worlds-with-any
(treatment minus baseline) against the pre-registered bar, the
birth-normalized rate beside it.

Usage:

    python3 phase20-lineage-reduction.py <dir> <lineage.txt> <seeds> <max_entities> \\
        <arm> [<treatment_arm>] --sesoi N --bar N

    seeds  a range `20001..20051` with exclusions after a slash, e.g.
           `20001..20051/20040`, or a comma list

The pre-registration (experiments/phase20-lineage-preregistration.md)
fixes every definition and number used here; this script counts, it does
not decide.
"""

import math
import re
import sys
from pathlib import Path
from statistics import median

LINEAGE = re.compile(r"world condition=(\S+) seed=0x([0-9a-f]+) (.*)")
MANIFEST = re.compile(r"^run condition=(\S+) seed=0x([0-9a-f]+) (.*)")


def parse_seeds(spec):
    exclusions = set()
    if "/" in spec:
        spec, excluded = spec.split("/", 1)
        exclusions = {int(v) for v in excluded.split(",") if v}
    if ".." in spec:
        low, high = spec.split("..")
        seeds = range(int(low), int(high) + 1)
    else:
        seeds = [int(v) for v in spec.split(",") if v]
    return [s for s in seeds if s not in exclusions]


def kv(text):
    out = {}
    for token in text.split():
        key, _, value = token.partition("=")
        out[key] = value
    return out


def read_lineage(path):
    header, worlds = [], {}
    for line in path.read_text().splitlines():
        if line.startswith("lineage-report") or line.startswith("index_version"):
            header.append(line.strip())
            continue
        m = LINEAGE.fullmatch(line.strip())
        if m:
            worlds[(m.group(1), int(m.group(2), 16))] = kv(m.group(3))
    return header, worlds


def read_manifest(path):
    runs = {}
    for line in path.read_text().splitlines():
        m = MANIFEST.match(line.strip())
        if m:
            runs[(m.group(1), int(m.group(2), 16))] = kv(m.group(3))
    return runs


def clopper_pearson_upper(k, n, alpha=0.025):
    """Exact one-sided upper bound on a binomial proportion, by bisection
    on the regularized incomplete beta through the binomial CDF."""
    if n == 0:
        return 1.0
    if k >= n:
        return 1.0

    def cdf(p):
        # P(X <= k) for X ~ Bin(n, p), computed in log space.
        total = 0.0
        for i in range(0, k + 1):
            total += math.exp(
                math.lgamma(n + 1) - math.lgamma(i + 1) - math.lgamma(n - i + 1)
                + i * math.log(p) + (n - i) * math.log1p(-p)
            )
        return total

    lo, hi = 0.0, 1.0
    for _ in range(200):
        mid = (lo + hi) / 2
        if mid <= 0.0:
            lo = mid
            continue
        if cdf(mid) > alpha:
            lo = mid
        else:
            hi = mid
    return hi


def main():
    args = sys.argv[1:]
    if "--sesoi" not in args or "--bar" not in args or len(args) < 7:
        print(__doc__, file=sys.stderr)
        return 2
    sesoi = int(args[args.index("--sesoi") + 1])
    bar = int(args[args.index("--bar") + 1])
    positional = [a for i, a in enumerate(args) if not a.startswith("--") and not (i > 0 and args[i - 1] in ("--sesoi", "--bar"))]
    directory = Path(positional[0])
    lineage_path = Path(positional[1])
    seeds = parse_seeds(positional[2])
    max_entities = int(positional[3])
    baseline = positional[4]
    treatment = positional[5] if len(positional) > 5 else None
    arms = [baseline] + ([treatment] if treatment else [])

    header, lineage = read_lineage(lineage_path)
    manifest = read_manifest(directory / "manifest.txt")
    failures = []
    if not any(h.startswith("lineage-report 1 ") for h in header) or not any("lifesim-lineage-index-v1" in h for h in header):
        failures.append(f"{lineage_path.name}: not a lineage-report 1 / index v1 report: {header}")
    worlds = {}
    for arm in arms:
        for seed in seeds:
            key = (arm, seed)
            if key not in lineage:
                failures.append(f"{arm} seed {seed}: no lineage line")
                continue
            if key not in manifest:
                failures.append(f"{arm} seed {seed}: no manifest run")
                continue
            ln, mf = lineage[key], manifest[key]
            births = int(mf.get("births", mf.get("births_total", "0")))
            # `materialized` is not a manifest field; the field series'
            # final sample carries it (series 3, `materialized=`).
            materialized = 0
            series = directory / f"{arm}-seed{seed:016x}.alfd"
            if series.exists():
                for line in reversed(series.read_text().splitlines()):
                    m = re.search(r"\bmaterialized=(\d+)", line)
                    if line.startswith("sample") and m:
                        materialized = int(m.group(1))
                        break
            worlds[key] = {
                "births": births,
                "population": int(mf.get("population", "0")),
                "materialized": materialized,
                "nonviable": int(mf.get("nonviable_bodies", "0")),
                "refused_budget": int(mf.get("refused_node_budget", "0")),
                "rej_energy": int(mf.get("pair_rejected_energy", "0")),
                "rej_capacity": int(mf.get("pair_rejected_capacity", "0")),
                "rej_placement": int(mf.get("pair_rejected_placement", "0")),
                "rej_nonviable": int(mf.get("pair_rejected_nonviable", "0")),
                "multi_total": int(ln["multi_total"]),
                "multi_born": int(ln["multi_born"]),
                "multi_parents": int(ln["multi_parents"]),
                "multi_offspring": int(ln["multi_offspring_total"]),
                "second_generation": int(ln["second_generation"]),
                "multi_median": int(ln["multi_median_lifespan_ticks"]),
                "multi_completed": int(ln["multi_completed_lifespans"]),
                "multi_censored": int(ln["multi_censored"]),
                "cohort_median": int(ln["cohort_median_lifespan_ticks"]),
                "cohort_completed": int(ln["cohort_completed"]),
                "cohort_censored": int(ln["cohort_censored"]),
                "max_modules": int(ln["max_modules"]),
                "first_multi_tick": int(ln["first_multi_tick"]),
                "compositions": ln.get("multi_compositions", "-"),
                "added": ln.get("added_modules", "-"),
            }
    for f in failures:
        print(f"REFUSED {f}")
    if failures:
        print(f"reduction refused: {len(failures)} defective or missing worlds")
        return 1

    print(f"phase20 reduction: arms {arms}, {len(seeds)} seeds, SESOI {sesoi}, bar {bar}")
    for arm in arms:
        print(f"condition {arm}")
        rows = [worlds[(arm, s)] for s in seeds]
        for s in seeds:
            w = worlds[(arm, s)]
            rate = (w["second_generation"] * 10_000 / w["births"]) if w["births"] else 0.0
            print(f"  seed {s} births={w['births']} population={w['population']} materialized={w['materialized']} "
                  f"multi_total={w['multi_total']} multi_born={w['multi_born']} multi_parents={w['multi_parents']} "
                  f"multi_offspring={w['multi_offspring']} second_generation={w['second_generation']} rate_per_10k={rate:.2f} "
                  f"multi_median={w['multi_median']} multi_completed={w['multi_completed']} multi_censored={w['multi_censored']} "
                  f"cohort_median={w['cohort_median']} cohort_completed={w['cohort_completed']} cohort_censored={w['cohort_censored']} "
                  f"max_modules={w['max_modules']} first_multi_tick={w['first_multi_tick']} nonviable={w['nonviable']} "
                  f"refused_budget={w['refused_budget']} rej_energy={w['rej_energy']} rej_capacity={w['rej_capacity']} "
                  f"rej_placement={w['rej_placement']} rej_nonviable={w['rej_nonviable']} compositions={w['compositions']} added={w['added']}")
        any_multi = sum(1 for w in rows if w["multi_total"] > 0)
        any_parent = sum(1 for w in rows if w["multi_parents"] > 0)
        any_second = sum(1 for w in rows if w["second_generation"] > 0)
        births = sum(w["births"] for w in rows)
        second = sum(w["second_generation"] for w in rows)
        at_cap = sum(1 for w in rows if w["population"] >= max_entities)
        gaps = [w["multi_median"] - w["cohort_median"] for w in rows if w["multi_completed"] > 0 and w["cohort_completed"] > 0]
        print(f"  worlds with any multi-module organism: {any_multi} / {len(seeds)}; with a multi-module parent: {any_parent}; "
              f"with any second-generation organism: {any_second}")
        print(f"  pooled second-generation {second} over {births} births = {second * 10_000 / births if births else 0:.3f} per 10,000; "
              f"exact 97.5% upper bound {clopper_pearson_upper(second, births) * 10_000 if births else float('nan'):.3f} per 10,000")
        print(f"  median multi minus cohort lifespan over worlds with both: {median(gaps) if gaps else 'n/a'} (n={len(gaps)})")
        print(f"  worlds at the entity cap {max_entities}: {at_cap} / {len(seeds)}")
        if treatment is None:
            met = (clopper_pearson_upper(second, births) * 10_000 if births else float('inf')) < sesoi
            print(f"C20.1 equivalence: upper bound {'BELOW' if met else 'NOT BELOW'} the SESOI of {sesoi} per 10,000 births")
    if treatment:
        count = 0
        for s in seeds:
            b, t = worlds[(baseline, s)], worlds[(treatment, s)]
            count += (t["second_generation"] > 0) and not (b["second_generation"] > 0)
        print(f"C20.1 pairs where {treatment} has a second-generation organism and {baseline} has none: {count} / {len(seeds)} "
              f"(bar {bar}: {'MET' if count >= bar else 'NOT MET'})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
