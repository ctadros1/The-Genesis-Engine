#!/usr/bin/env python3
"""Phase 22 lineages-under-the-other-order reduction (C22.1, C22.2, C22.3).

Reads a campaign output directory's manifest, a saved `lifesim lineage
--manifest` report (lineage-report 1, index lifesim-lineage-index-v1) and
a saved `lifesim cohort --manifest` report (cohort-report 1), refuses
anything missing (counted, never silent), and prints per world and per
arm: second-generation organisms, births, the rate per 10,000 births,
multi-module total and parents, the largest second-generation count,
born parents, the born median lifespan; then the seed-paired directed
count of worlds where the treatment has a second-generation organism
and the baseline none, against the bar; the count of worlds with any
per arm; the median rate per arm; the entity-cap gate.

Usage:

    python3 phase22-lineages-reduction.py <dir> <lineage.txt> <cohort.txt> <seeds> \
        <max_entities> <baseline> <treatment> --bar N

The pre-registration (experiments/phase22-lineages-preregistration.md)
fixes every definition and number used here; this script counts, it does
not decide.
"""

import math
import re
import sys
from pathlib import Path
from statistics import median

LINE = re.compile(r"world condition=(\S+) seed=0x([0-9a-f]+) (.*)")
MANIFEST = re.compile(r"^run (?:index=\d+ )?condition=(\S+) seed=0x([0-9a-f]+) (.*)")


def parse_seeds(spec):
    exclusions = set()
    if "/" in spec:
        spec, excluded = spec.split("/", 1)
        exclusions = {int(v) for v in excluded.split(",") if v}
    out = []
    for part in spec.split():
        if ".." in part:
            low, high = part.split("..")
            out.extend(range(int(low), int(high) + 1))
        else:
            out.extend(int(v) for v in part.split(",") if v)
    return [s for s in out if s not in exclusions]


def kv(text):
    out = {}
    for token in text.split():
        key, _, value = token.partition("=")
        out[key] = value
    return out


def read_report(path, prefix, index_name):
    header, worlds = [], {}
    for line in path.read_text().splitlines():
        if line.startswith(prefix) or line.startswith("index_version"):
            header.append(line.strip())
            continue
        m = LINE.fullmatch(line.strip())
        if m:
            worlds[(m.group(1), int(m.group(2), 16))] = kv(m.group(3))
    ok = any(h.startswith(prefix + " 1 ") for h in header) and any(index_name in h for h in header)
    return ok, worlds


def read_manifest(path):
    runs = {}
    for line in path.read_text().splitlines():
        m = MANIFEST.match(line.strip())
        if m:
            runs[(m.group(1), int(m.group(2), 16))] = kv(m.group(3))
    return runs


def main():
    args = sys.argv[1:]
    if "--bar" not in args or len(args) < 9:
        print(__doc__, file=sys.stderr)
        return 2
    bar = int(args[args.index("--bar") + 1])
    positional = [a for i, a in enumerate(args) if not a.startswith("--") and not (i > 0 and args[i - 1] == "--bar")]
    directory = Path(positional[0]); lineage_path = Path(positional[1]); cohort_path = Path(positional[2])
    seeds = parse_seeds(positional[3]); max_entities = int(positional[4])
    baseline, treatment = positional[5], positional[6]
    ok_l, lineage = read_report(lineage_path, "lineage-report", "lifesim-lineage-index-v1")
    ok_c, cohort = read_report(cohort_path, "cohort-report", "lifesim-cohort-index-v1")
    manifest = read_manifest(directory / "manifest.txt")
    failures = []
    if not ok_l:
        failures.append(f"{lineage_path.name}: not a lineage-report 1 / index v1 report")
    if not ok_c:
        failures.append(f"{cohort_path.name}: not a cohort-report 1 / index v1 report")
    worlds = {}
    for arm in (baseline, treatment):
        for seed in seeds:
            key = (arm, seed)
            missing = [name for name, table in (("lineage", lineage), ("cohort", cohort), ("manifest", manifest)) if key not in table]
            if missing:
                failures.append(f"{arm} seed {seed}: no {', '.join(missing)} line")
                continue
            ln, co, mf = lineage[key], cohort[key], manifest[key]
            births = int(mf.get("births", "0"))
            sg = int(ln["second_generation"])
            worlds[key] = {
                "second_generation": sg,
                "births": births,
                "rate_per_10k": (sg * 10_000 / births) if births else 0.0,
                "multi_total": int(ln["multi_total"]),
                "multi_parents": int(ln["multi_parents"]),
                "multi_offspring": int(ln["multi_offspring_total"]),
                "born_parents": int(ln.get("born_parents", "0")),
                "max_modules": int(ln["max_modules"]),
                "compositions": ln.get("multi_compositions", "-"),
                "born_median": int(co["born_median_lifespan_ticks"]),
                "mat_median": int(co["mat_median_lifespan_ticks"]),
                "population": int(mf.get("population", "0")),
                "capacity_rejections": int(mf.get("capacity_rejections", "0")),
            }
    for f in failures:
        print(f"REFUSED {f}")
    if failures:
        print(f"reduction refused: {len(failures)} defective or missing worlds")
        return 1
    print(f"phase22 reduction: {baseline} vs {treatment}, {len(seeds)} seeds, bar {bar}")
    for arm in (baseline, treatment):
        rows = [worlds[(arm, s)] for s in seeds]
        print(f"condition {arm}")
        for s in seeds:
            w = worlds[(arm, s)]
            print("  seed", s, " ".join(f"{k}={v}" if not isinstance(v, float) else f"{k}={v:.3f}" for k, v in w.items()))
        med = lambda k: median(w[k] for w in rows)
        any_sg = sum(1 for w in rows if w["second_generation"] > 0)
        print(f"  worlds with any second-generation organism: {any_sg} / {len(seeds)}; pooled {sum(w['second_generation'] for w in rows)} "
              f"over {sum(w['births'] for w in rows)} births; median rate per 10,000 {med('rate_per_10k'):.3f}; largest per-world second-generation count {max(w['second_generation'] for w in rows)}")
        print(f"  multi-module: total median {med('multi_total')}, parents in {sum(1 for w in rows if w['multi_parents']>0)} worlds; born median lifespan {med('born_median')}; materialized {med('mat_median')}")
        at_cap = sum(1 for w in rows if w["population"] >= max_entities or w["capacity_rejections"] > 0)
        print(f"  worlds at the entity cap {max_entities} (final population at the cap, or any capacity rejection during the run): {at_cap} / {len(seeds)}")
    count = sum(1 for s in seeds if worlds[(treatment, s)]["second_generation"] > 0 and worlds[(baseline, s)]["second_generation"] == 0)
    reverse = sum(1 for s in seeds if worlds[(baseline, s)]["second_generation"] > 0 and worlds[(treatment, s)]["second_generation"] == 0)
    print(f"C22.1 rule 1 - pairs where {treatment} has a second-generation organism and {baseline} none: {count} / {len(seeds)} "
          f"(reverse pairs {reverse}) (bar {bar}: {'MET' if count >= bar else 'NOT MET'})")
    # The unconditional tail's false-positive rate depends on the baseline
    # arm's true rate; restate it at the upper end of the observed O1 rate
    # (exact 97.5 percent Clopper-Pearson bound on worlds-with-any).
    base_any = sum(1 for s in seeds if worlds[(baseline, s)]["second_generation"] > 0)
    n = len(seeds)
    def cp_upper(k, n, alpha=0.025):
        if k >= n: return 1.0
        lo, hi = 0.0, 1.0
        def cdf(p): return sum(math.exp(math.lgamma(n+1)-math.lgamma(i+1)-math.lgamma(n-i+1)+i*math.log(p)+(n-i)*math.log1p(-p)) for i in range(k+1))
        for _ in range(200):
            mid = (lo+hi)/2
            if mid <= 0: lo = mid; continue
            if cdf(mid) > alpha: lo = mid
            else: hi = mid
        return hi
    p0 = cp_upper(base_any, n)
    q = p0 * (1 - p0)
    alpha = sum(math.comb(n, i) * q**i * (1-q)**(n-i) for i in range(bar, n + 1))
    print(f"  rule 1's false-positive rate at the upper bound of the observed {baseline} rate ({base_any}/{n} -> {p0:.3f}): {alpha:.4f}")
    # Rule 2: the exact conditional sign test on the discordant pairs.
    discordant = count + reverse
    p_value = sum(math.comb(discordant, i) for i in range(count, discordant + 1)) / 2 ** discordant if discordant else 1.0
    print(f"C22.1 rule 2 - conditional sign test on {discordant} discordant pairs ({count} {treatment}-only vs {reverse} {baseline}-only): one-sided p = {p_value:.2e} "
          f"({'MET' if p_value < 0.01 else 'NOT MET'} at 0.01)")
    print(f"C22.1: {'MET' if count >= bar and p_value < 0.01 else 'NOT MET'} (both rules required)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
