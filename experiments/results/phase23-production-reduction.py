#!/usr/bin/env python3
"""Phase 23 production dose reduction (C23.1, C23.2, C23.3, C23.4, C23.5).

Reads four things from a campaign output directory and refuses anything
missing or inconsistent (counted, never silent):

  manifest.txt          - per world: ticks, births, final population,
                          capacity_rejections, and the three transition
                          columns (transition_materialized,
                          transition_deferred_cap,
                          transition_deferred_capacity) the manifest gained
                          for this phase;
  <arm>-seed<hex>.alfd  - the field series (field-series 3): the final
                          field mass (chem + microbial), the final
                          materialized count, and the field identity at
                          every sample;
  cohort.txt            - a saved `lifesim cohort --manifest` report: the
                          born and materialized cohorts' completed /
                          censored counts and median completed lifespans,
                          born-site food and occupants medians;
  lineage.txt           - a saved `lifesim lineage --manifest` report:
                          second-generation two-module organisms and the
                          multi-module total.

Per world and per arm it prints those numbers and the entity-cap gate
(final population at the cap, or any capacity rejection, or any
materialization deferred for capacity; the per-tick deferral beside it).
Then, on seed-paired differences of the born median:

  C23.1  each dose rung and the top dose against the control: the directed
         count of pairs where the rung clears the control by the SESOI,
         against the bar; the median difference and its bootstrap interval;
  C23.2  the other order against the shipped one at the top dose (and at
         the control dose): the directed count against the bar, the count
         of pairs within +-SESOI, and the bootstrap interval of the median
         paired difference read against +-SESOI (equivalence);
  C23.3  the top dose under the shipped order against the other order at
         the control dose (the reference, on the same seeds): the
         bootstrap interval's lower bound against -SESOI (non-inferiority)
         and the count of pairs at or above the reference less the SESOI.

Usage:

    python3 phase23-production-reduction.py <dir> <cohort.txt> <lineage.txt> <seeds> \\
        <max_entities> --control P1 --top PT --top-order PTD --reference-order P1D \\
        [--rungs P4,P16,P64] [--order-pairs P128D:P128:20] [--ceiling 36000] \\
        --sesoi N [--sesoi-top N] [--sesoi-reference N] --bar N [--pin-archive]

One SESOI per contrast, each a tenth of the level at which that contrast
is read (fixed from the pilot, never from the data reduced): `--sesoi`
for every contrast read at the control's level (the rungs and the top
dose against the control, the other order at the control dose),
`--sesoi-top` for the other order at the top dose (default: `--sesoi`),
`--sesoi-reference` for the top dose against the reference (default:
`--sesoi`), and a third field on each `--order-pairs` entry.
`--order-pairs` names further order contrasts (order arm : base arm :
sesoi) reported exactly as C23.2's; `--ceiling` is the config's `max_age_ticks`, so the
count of worlds whose born median sits at the age ceiling is reported per
arm (a born cohort that lives to the ceiling makes the order contrast a
contrast of ceilings, which the count makes visible).

`--pin-archive` tolerates a manifest without the transition columns (the
Phase 21 archive the script is pinned on predates them) and says so in
its header; the confirmatory is never read with it. Every definition and
number used here is fixed by experiments/phase23-production-preregistration.md;
this script counts, it does not decide.
"""

import random
import re
import sys
from pathlib import Path
from statistics import median

LINE = re.compile(r"world condition=(\S+) seed=0x([0-9a-f]+) (.*)")
MANIFEST = re.compile(r"^run (?:index=\d+ )?condition=(\S+) seed=0x([0-9a-f]+) (.*)")
SAMPLE = re.compile(
    r"sample tick=(\d+) fired=(\d+) seeded_milli=(\d+) chem_milli=(-?\d+) "
    r"produced_milli=(\d+) deposited_milli=(\d+) microbial_milli=(-?\d+) "
    r"occupied=(\d+) population=(\d+) materialized=(\d+) "
    r"materialized_milli=(-?\d+) max_modules=(\d+) multi_module=(\d+) births=(\d+) "
    r"consumed_milli=(-?\d+)"
)
SAMPLE_KEYS = [
    "tick", "fired", "seeded", "chem", "produced", "deposited", "microbial",
    "occupied", "population", "materialized", "materialized_milli",
    "max_modules", "multi_module", "births", "consumed",
]
BOOTSTRAP_RESAMPLES = 10_000
BOOTSTRAP_SEED = 23


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


def read_field_series(path, ticks, failures):
    """The final sample of a field-series 3 file, after the identity and
    the sample count are checked at every row (Phase 19's gates)."""
    if not path.exists():
        failures.append(f"{path.name}: missing")
        return None
    lines = path.read_text().splitlines()
    if not lines or not lines[0].startswith("field-series 3 "):
        failures.append(f"{path.name}: header is not field-series 3")
        return None
    interval = int(lines[0].split()[-1])
    rows = []
    for line in lines[1:]:
        m = SAMPLE.fullmatch(line.strip())
        if m:
            rows.append(dict(zip(SAMPLE_KEYS, (int(v) for v in m.groups()))))
    expected = ticks // interval
    if len(rows) != expected:
        failures.append(f"{path.name}: {len(rows)} samples, expected {expected}")
        return None
    for row in rows:
        defect = row["chem"] + row["microbial"] - (
            row["produced"] + row["deposited"] - row["materialized_milli"] - row["consumed"]
        )
        if defect != 0:
            failures.append(f"{path.name}: field identity defect {defect} milli at tick {row['tick']}")
            return None
    return rows[-1]


def bootstrap_interval(values, resamples=BOOTSTRAP_RESAMPLES, seed=BOOTSTRAP_SEED):
    """A 95 percent percentile interval of the median, from a seeded
    resample so the same numbers always give the same interval."""
    rng = random.Random(seed)
    n = len(values)
    medians = sorted(median(rng.choice(values) for _ in range(n)) for _ in range(resamples))
    low = medians[int(0.025 * resamples)]
    high = medians[min(resamples - 1, int(0.975 * resamples))]
    return low, high


def opt(args, name, cast, default=None):
    if name not in args:
        if default is None:
            raise SystemExit(f"missing {name}\n{__doc__}")
        return default
    return cast(args[args.index(name) + 1])


def main():
    args = sys.argv[1:]
    valued = {"--control", "--top", "--top-order", "--reference-order", "--rungs", "--order-pairs", "--ceiling",
              "--sesoi", "--sesoi-top", "--sesoi-reference", "--bar"}
    pin_archive = "--pin-archive" in args
    positional = [a for i, a in enumerate(args) if not a.startswith("--") and not (i > 0 and args[i - 1] in valued)]
    if len(positional) != 5:
        print(__doc__, file=sys.stderr)
        return 2
    directory = Path(positional[0]); cohort_path = Path(positional[1]); lineage_path = Path(positional[2])
    seeds = parse_seeds(positional[3]); max_entities = int(positional[4])
    control = opt(args, "--control", str); top = opt(args, "--top", str)
    top_order = opt(args, "--top-order", str); reference_order = opt(args, "--reference-order", str)
    rungs = [r for r in opt(args, "--rungs", str, "").split(",") if r]
    ceiling = opt(args, "--ceiling", int, 0)
    sesoi = opt(args, "--sesoi", float); bar = opt(args, "--bar", int)
    sesoi_top = opt(args, "--sesoi-top", float, sesoi); sesoi_reference = opt(args, "--sesoi-reference", float, sesoi)
    order_pairs = []
    for entry in [p for p in opt(args, "--order-pairs", str, "").split(",") if p]:
        parts = entry.split(":")
        if len(parts) != 3:
            raise SystemExit(f"--order-pairs entry must be order:base:sesoi, got {entry!r}")
        order_pairs.append((parts[0], parts[1], float(parts[2])))
    arms = []
    for arm in [control] + rungs + [top, top_order, reference_order] + [a for pair in order_pairs for a in pair[:2]]:
        if arm not in arms:
            arms.append(arm)

    ok_c, cohort = read_report(cohort_path, "cohort-report", "lifesim-cohort-index-v1")
    ok_l, lineage = read_report(lineage_path, "lineage-report", "lifesim-lineage-index-v1")
    manifest = read_manifest(directory / "manifest.txt")
    failures = []
    if not ok_c:
        failures.append(f"{cohort_path.name}: not a cohort-report 1 / index v1 report")
    if not ok_l:
        failures.append(f"{lineage_path.name}: not a lineage-report 1 / index v1 report")
    worlds = {}
    transition_keys = ("transition_materialized", "transition_deferred_cap", "transition_deferred_capacity")
    for arm in arms:
        for seed in seeds:
            key = (arm, seed)
            missing = [name for name, table in (("cohort", cohort), ("lineage", lineage), ("manifest", manifest)) if key not in table]
            if missing:
                failures.append(f"{arm} seed {seed}: no {', '.join(missing)} line")
                continue
            co, ln, mf = cohort[key], lineage[key], manifest[key]
            ticks = int(mf["ticks"])
            final = read_field_series(directory / f"{arm}-seed{seed:016x}.alfd", ticks, failures)
            if final is None:
                continue
            if all(k in mf for k in transition_keys):
                materialized_manifest = int(mf["transition_materialized"])
                deferred_cap = int(mf["transition_deferred_cap"])
                deferred_capacity = int(mf["transition_deferred_capacity"])
                if materialized_manifest != final["materialized"]:
                    failures.append(f"{arm} seed {seed}: manifest materialized {materialized_manifest} != field series {final['materialized']}")
                    continue
            elif pin_archive:
                deferred_cap = deferred_capacity = None
            else:
                failures.append(f"{arm} seed {seed}: manifest lacks the transition columns")
                continue
            mat_count = int(co["mat_completed"]) + int(co["mat_censored"])
            if mat_count != final["materialized"]:
                failures.append(f"{arm} seed {seed}: cohort materialized {mat_count} != field series {final['materialized']}")
                continue
            worlds[key] = {
                "born_median": int(co["born_median_lifespan_ticks"]),
                "born_completed": int(co["born_completed"]),
                "born_censored": int(co["born_censored"]),
                "mat_median": int(co["mat_median_lifespan_ticks"]),
                "materialized": mat_count,
                "born_food": int(co["born_site_food_median"]),
                "born_occupants": int(co["born_occupants_median"]),
                "births": int(mf["births"]),
                "population": int(mf["population"]),
                "field_mass": final["chem"] + final["microbial"],
                "capacity_rejections": int(mf["capacity_rejections"]),
                "deferred_capacity": deferred_capacity,
                "deferred_cap": deferred_cap,
                "second_generation": int(ln["second_generation"]),
                "multi_total": int(ln["multi_total"]),
            }
    for f in failures:
        print(f"REFUSED {f}")
    if failures:
        print(f"reduction refused: {len(failures)} defective or missing worlds")
        return 1

    print(f"phase23 reduction: control {control}, rungs {rungs}, top {top}, top-order {top_order}, "
          f"reference-order {reference_order}, further order pairs {order_pairs}, age ceiling {ceiling or 'none'}; "
          f"{len(seeds)} seeds; SESOI {sesoi} ticks at the control level, {sesoi_top} at the top dose, "
          f"{sesoi_reference} against the reference; bar {bar}; "
          f"transition columns {'TOLERATED ABSENT (pin on an archive)' if pin_archive else 'REQUIRED'}")

    def fmt(v):
        return "n/a" if v is None else str(v)

    for arm in arms:
        rows = [worlds[(arm, s)] for s in seeds]
        print(f"condition {arm}")
        for s in seeds:
            print("  seed", s, " ".join(f"{k}={fmt(v)}" for k, v in worlds[(arm, s)].items()))
        med = lambda k: median(w[k] for w in rows)
        capped = [w for w in rows if w["population"] >= max_entities or w["capacity_rejections"] > 0
                  or (w["deferred_capacity"] or 0) > 0]
        deferred_cap_worlds = sum(1 for w in rows if (w["deferred_cap"] or 0) > 0)
        print(f"  born median (median over worlds) {med('born_median')}; materialized median {med('mat_median')}; "
              f"materialized count {med('materialized')}; births {med('births')}; population {med('population')}; "
              f"field mass {med('field_mass')}; born-site food {med('born_food')}; born occupants {med('born_occupants')}; "
              f"multi-module total {med('multi_total')}; "
              f"worlds with any second-generation organism {sum(1 for w in rows if w['second_generation'] > 0)} / {len(seeds)}; "
              f"worlds with the born median at the age ceiling {sum(1 for w in rows if ceiling and w['born_median'] >= ceiling)} / {len(seeds)}")
        print(f"  entity-cap gate (final population at {max_entities}, or any capacity rejection, or any "
              f"materialization deferred for capacity): {len(capped)} / {len(seeds)}; "
              f"worlds with any per-tick materialization deferral: "
              f"{'n/a' if rows[0]['deferred_cap'] is None else deferred_cap_worlds} / {len(seeds)}")

    def paired(a, b):
        return [worlds[(a, s)]["born_median"] - worlds[(b, s)]["born_median"] for s in seeds]

    for rung in rungs + [top]:
        deltas = paired(rung, control)
        count = sum(1 for d in deltas if d > sesoi)
        low, high = bootstrap_interval(deltas)
        print(f"C23.1 {rung} vs {control}: pairs with born median delta > {sesoi}: {count} / {len(seeds)} "
              f"(bar {bar}: {'MET' if count >= bar else 'NOT MET'}); delta median {median(deltas)} "
              f"interval [{low}, {high}] min {min(deltas)} max {max(deltas)}")

    for label, order_arm, base_arm, band in [("top dose", top_order, top, sesoi_top), ("control dose", reference_order, control, sesoi)] + [
            ("further pair", o, b, s_) for o, b, s_ in order_pairs]:
        deltas = paired(order_arm, base_arm)
        count = sum(1 for d in deltas if d > band)
        within = sum(1 for d in deltas if abs(d) <= band)
        low, high = bootstrap_interval(deltas)
        if count >= bar:
            reading = "the order still matters (directed count at or above the bar)"
        elif -band <= low and high <= band:
            reading = "the order stopped mattering (interval within +-SESOI)"
        else:
            reading = "undecided (neither the bar nor equivalence)"
        print(f"C23.2 {order_arm} vs {base_arm} ({label}, SESOI {band}): pairs with delta > {band}: {count} / {len(seeds)} "
              f"(bar {bar}: {'MET' if count >= bar else 'NOT MET'}); pairs within +-{band}: {within} / {len(seeds)}; "
              f"delta median {median(deltas)} interval [{low}, {high}]; reading: {reading}")

    deltas = paired(top, reference_order)
    low, high = bootstrap_interval(deltas)
    at_or_above = sum(1 for d in deltas if d >= -sesoi_reference)
    print(f"C23.3 {top} vs {reference_order} (the reference on the same seeds, SESOI {sesoi_reference}): delta median {median(deltas)} "
          f"interval [{low}, {high}]; lower bound >= -{sesoi_reference}: {'REACHED' if low >= -sesoi_reference else 'NOT REACHED'}; "
          f"pairs at or above the reference less the SESOI: {at_or_above} / {len(seeds)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
