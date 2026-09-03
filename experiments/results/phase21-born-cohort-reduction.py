#!/usr/bin/env python3
"""Phase 21 born-cohort reduction (C21.1, C21.2, C21.3).

Reads a campaign output directory's manifest and a saved
`lifesim cohort --manifest` report (cohort-report 1, index
lifesim-cohort-index-v1), refuses anything missing (counted, never
silent), and prints per world and per arm the census's numbers, then:

  observational shape (one arm):
    worlds where rho_food_milli >= SESOI_F        (against BAR_F)
    worlds where rho_occupants_milli <= -SESOI_O  (against BAR_O)
    the site ratio and the C21.3 splits, medians over worlds
  probe shape (two arms, seed-paired):
    worlds where born median lifespan (treatment - baseline) > SESOI_T
    (against BAR_T), the materialized median delta beside it

Usage:

    python3 phase21-born-cohort-reduction.py <dir> <cohort.txt> <seeds> <max_entities> \\
        <baseline> [<treatment>] --sesoi-food N --bar-food N --sesoi-occ N --bar-occ N \\
        [--sesoi-ticks N --bar-ticks N]

The pre-registration (experiments/phase21-born-cohort-preregistration.md)
fixes every definition and number used here; this script counts, it does
not decide.
"""

import re
import sys
from pathlib import Path
from statistics import median

COHORT = re.compile(r"world condition=(\S+) seed=0x([0-9a-f]+) (.*)")
MANIFEST = re.compile(r"^run (?:index=\d+ )?condition=(\S+) seed=0x([0-9a-f]+) (.*)")


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


def read_report(path, header_prefix, index_name):
    header, worlds = [], {}
    for line in path.read_text().splitlines():
        if line.startswith(header_prefix) or line.startswith("index_version"):
            header.append(line.strip())
            continue
        m = COHORT.fullmatch(line.strip())
        if m:
            worlds[(m.group(1), int(m.group(2), 16))] = kv(m.group(3))
    ok = any(h.startswith(header_prefix + " 1 ") for h in header) and any(index_name in h for h in header)
    return ok, worlds


def read_manifest(path):
    runs = {}
    for line in path.read_text().splitlines():
        m = MANIFEST.match(line.strip())
        if m:
            runs[(m.group(1), int(m.group(2), 16))] = kv(m.group(3))
    return runs


def opt(args, name, default=None, cast=float):
    if name in args:
        return cast(args[args.index(name) + 1])
    return default


def main():
    args = sys.argv[1:]
    flags = {"--sesoi-food", "--bar-food", "--sesoi-occ", "--bar-occ", "--sesoi-ticks", "--bar-ticks"}
    positional = [a for i, a in enumerate(args) if not a.startswith("--") and not (i > 0 and args[i - 1] in flags)]
    if len(positional) < 5:
        print(__doc__, file=sys.stderr)
        return 2
    directory = Path(positional[0]); report_path = Path(positional[1])
    seeds = parse_seeds(positional[2]); max_entities = int(positional[3])
    baseline = positional[4]; treatment = positional[5] if len(positional) > 5 else None
    sesoi_food = opt(args, "--sesoi-food", 0.0); bar_food = opt(args, "--bar-food", 0, int)
    sesoi_occ = opt(args, "--sesoi-occ", 0.0); bar_occ = opt(args, "--bar-occ", 0, int)
    sesoi_ticks = opt(args, "--sesoi-ticks", 0.0); bar_ticks = opt(args, "--bar-ticks", 0, int)
    arms = [baseline] + ([treatment] if treatment else [])

    ok, report = read_report(report_path, "cohort-report", "lifesim-cohort-index-v1")
    manifest = read_manifest(directory / "manifest.txt")
    failures = [] if ok else [f"{report_path.name}: not a cohort-report 1 / index v1 report"]
    worlds = {}
    for arm in arms:
        for seed in seeds:
            key = (arm, seed)
            if key not in report:
                failures.append(f"{arm} seed {seed}: no cohort line"); continue
            if key not in manifest:
                failures.append(f"{arm} seed {seed}: no manifest run"); continue
            r, mf = report[key], manifest[key]
            try:
                worlds[key] = {
                    "population": int(mf.get("population", "0")),
                    "births": int(mf.get("births", "0")),
                    "born_median": int(r["born_median_lifespan_ticks"]),
                    "born_completed": int(r["born_completed"]),
                    "born_censored": int(r["born_censored"]),
                    "mat_median": int(r["mat_median_lifespan_ticks"]),
                    "mat_completed": int(r["mat_completed"]),
                    "born_food": int(r["born_site_food_median"]),
                    "mat_food": int(r["mat_site_food_median"]),
                    "food_ratio": int(r["food_ratio_milli"]),
                    "born_occ": int(r["born_occupants_median"]),
                    "mat_occ": int(r["mat_occupants_median"]),
                    "rho_food": int(r["rho_food_milli"]),
                    "rho_occ": int(r["rho_occupants_milli"]),
                    "blocks_used": int(r["blocks_used"]),
                    "blocks_skipped": int(r["blocks_skipped"]),
                    "pooled_food": int(r["pooled_rho_food_milli"]),
                    "pooled_occ": int(r["pooled_rho_occupants_milli"]),
                    "partial_food": int(r["partial_food_milli"]),
                    "partial_occ": int(r["partial_occupants_milli"]),
                    "matured": int(r["born_reached_maturity"]),
                    "reproduced": int(r["born_reproduced"]),
                    "matured_q": r.get("reached_maturity_food_quartile", "-"),
                    "reproduced_q": r.get("reproduced_food_quartile", "-"),
                    "matured_occ": r.get("reached_maturity_occupants", "-"),
                    "reproduced_occ": r.get("reproduced_occupants", "-"),
                    "waste": int(r.get("waste_median", "0")),
                    "polymer": int(r.get("polymer_median", "0")),
                    "microbial": int(r.get("microbial_median", "0")),
                }
            except KeyError as missing:
                failures.append(f"{arm} seed {seed}: cohort line lacks {missing}")
    for f in failures:
        print(f"REFUSED {f}")
    if failures:
        print(f"reduction refused: {len(failures)} defective or missing worlds"); return 1

    print(f"phase21 reduction: arms {arms}, {len(seeds)} seeds; SESOI food {sesoi_food} bar {bar_food}; "
          f"SESOI occupants {sesoi_occ} bar {bar_occ}; SESOI ticks {sesoi_ticks} bar {bar_ticks}")
    for arm in arms:
        rows = [worlds[(arm, s)] for s in seeds]
        print(f"condition {arm}")
        for s in seeds:
            w = worlds[(arm, s)]
            print("  seed", s, " ".join(f"{k}={v}" for k, v in w.items()))
        med = lambda k: median(w[k] for w in rows)
        print(f"  born median lifespan (median over worlds) {med('born_median')}; materialized {med('mat_median')}; "
              f"born completed {med('born_completed')} censored {med('born_censored')}")
        print(f"  site food: born {med('born_food')} vs materialized {med('mat_food')} (ratio milli median {med('food_ratio')}); "
              f"occupants: born {med('born_occ')} vs materialized {med('mat_occ')}")
        print(f"  rho food (median block) median {med('rho_food')} range {min(w['rho_food'] for w in rows)}..{max(w['rho_food'] for w in rows)}; "
              f"rho occupants median {med('rho_occ')} range {min(w['rho_occ'] for w in rows)}..{max(w['rho_occ'] for w in rows)}; "
              f"blocks used median {med('blocks_used')} skipped median {med('blocks_skipped')}")
        print(f"  pooled rho food {med('pooled_food')} occupants {med('pooled_occ')}; within-block partials food {med('partial_food')} occupants {med('partial_occ')}")
        print(f"  matured median {med('matured')} of born; reproduced median {med('reproduced')}; waste/polymer/microbial medians {med('waste')}/{med('polymer')}/{med('microbial')}")
        at_cap = sum(1 for w in rows if w["population"] >= max_entities)
        print(f"  worlds at the entity cap {max_entities}: {at_cap} / {len(seeds)}")
        if treatment is None:
            food_count = sum(1 for w in rows if w["rho_food"] >= sesoi_food)
            occ_count = sum(1 for w in rows if w["rho_occ"] <= -sesoi_occ)
            print(f"C21.1 worlds with rho_food >= {sesoi_food}: {food_count} / {len(seeds)} (bar {bar_food}: {'MET' if food_count >= bar_food else 'NOT MET'})")
            print(f"C21.1 worlds with rho_occupants <= -{sesoi_occ}: {occ_count} / {len(seeds)} (bar {bar_occ}: {'MET' if occ_count >= bar_occ else 'NOT MET'})")
    if treatment:
        count = 0; deltas = []; mat_deltas = []
        for s in seeds:
            b, t = worlds[(baseline, s)], worlds[(treatment, s)]
            d = t["born_median"] - b["born_median"]; deltas.append(d); mat_deltas.append(t["mat_median"] - b["mat_median"])
            count += d > sesoi_ticks
        print(f"pairs: born median delta median {median(deltas)} min {min(deltas)} max {max(deltas)}; materialized delta median {median(mat_deltas)}")
        print(f"C21.1 pairs with born median delta > {sesoi_ticks}: {count} / {len(seeds)} (bar {bar_ticks}: {'MET' if count >= bar_ticks else 'NOT MET'})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
