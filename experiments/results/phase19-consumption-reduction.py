#!/usr/bin/env python3
"""Phase 19 chemistry-as-food reduction (C19.1, C19.3, C19.4, C19.5).

Reads a campaign output directory's `.alfd` field series (series version
3: the Phase 16 columns plus `consumed_milli`) and a saved
`lifesim demography --manifest` report (report 2, index v2, which starts
a materialized organism's life at its `Materialized` record), refuses
anything missing, short or conservation-broken (counted, never silent),
and prints:

  per world:  population, materialized_total, births, consumed_milli,
              final chem+microbial mass, peak max_modules,
              materialized completed / censored / median lifespan
  per seed pair (treatment minus baseline):
              materialized median lifespan delta      (C19.3)
              births delta                             (C19.4)
  counts:     pairs with lifespan delta  > SESOI_L     (C19.3, against the bar)
              pairs with births delta    > SESOI_B     (C19.4, against the bar)
              worlds with any body above one module, per arm   (C19.5)
              worlds at the entity cap, per arm  (the free-lunch check)

The field identity checked at every sample is the Phase 19 one (C19.1):
    chem + microbial == produced + deposited - materialized_milli - consumed

Usage:

    python3 phase19-consumption-reduction.py <dir> <demography.txt> <baseline> \\
        <treatment> <seeds> <ticks> <interval> <cells> <max_entities> <sesoi_l> <bar_l> <sesoi_b> <bar_b>

    seeds       a range `19001..19032` or a comma list; a range may carry
                exclusions after a slash, e.g. `19001..19032/19009,19023`

The pre-registration (experiments/phase19-consumption-preregistration.md)
fixes every definition and number used here; this script counts, it does
not decide.
"""

import re
import sys
from pathlib import Path
from statistics import median

SAMPLE = re.compile(
    r"sample tick=(\d+) fired=(\d+) seeded_milli=(\d+) chem_milli=(-?\d+) "
    r"produced_milli=(\d+) deposited_milli=(\d+) microbial_milli=(-?\d+) "
    r"occupied=(\d+) population=(\d+) materialized=(\d+) "
    r"materialized_milli=(-?\d+) max_modules=(\d+) multi_module=(\d+) births=(\d+) "
    r"consumed_milli=(-?\d+)"
)
COLUMNS = [
    "tick", "fired", "seeded", "chem", "produced", "deposited", "microbial",
    "occupied", "population", "materialized", "materialized_milli",
    "max_modules", "multi_module", "births", "consumed",
]
DEMOGRAPHY = re.compile(r"world condition=(\S+) seed=0x([0-9a-f]+) (.*)")


def parse_seeds(spec: str) -> list[int]:
    exclusions: set[int] = set()
    if "/" in spec:
        spec, excluded = spec.split("/", 1)
        exclusions = {int(value) for value in excluded.split(",") if value}
    if ".." in spec:
        low, high = spec.split("..")
        seeds = range(int(low), int(high) + 1)
    else:
        seeds = [int(value) for value in spec.split(",") if value]
    return [seed for seed in seeds if seed not in exclusions]


def read_series(path: Path) -> tuple[str, list[dict]]:
    header = ""
    rows = []
    for line in path.read_text().splitlines():
        if line.startswith("field-series"):
            header = line.strip()
            continue
        match = SAMPLE.fullmatch(line.strip())
        if match:
            rows.append(dict(zip(COLUMNS, (int(group) for group in match.groups()))))
    return header, rows


def read_demography(path: Path) -> tuple[list[str], dict]:
    header = []
    worlds = {}
    for line in path.read_text().splitlines():
        if line.startswith("demography-report") or line.startswith("index_version"):
            header.append(line.strip())
            continue
        match = DEMOGRAPHY.fullmatch(line.strip())
        if not match:
            continue
        fields = {}
        for token in match.group(3).split():
            key, _, value = token.partition("=")
            fields[key] = value
        worlds[(match.group(1), int(match.group(2), 16))] = fields
    return header, worlds


def main() -> int:
    if len(sys.argv) != 14:
        print(__doc__, file=sys.stderr)
        return 2
    directory = Path(sys.argv[1])
    demography_path = Path(sys.argv[2])
    baseline = sys.argv[3]
    treatment = sys.argv[4]
    seeds = parse_seeds(sys.argv[5])
    ticks = int(sys.argv[6])
    interval = int(sys.argv[7])
    cells = int(sys.argv[8])
    max_entities = int(sys.argv[9])
    sesoi_l = int(sys.argv[10])
    bar_l = int(sys.argv[11])
    sesoi_b = int(sys.argv[12])
    bar_b = int(sys.argv[13])
    expected_samples = ticks // interval

    demo_header, demography = read_demography(demography_path)
    failures = []
    if "demography-report 2" not in " ".join(demo_header) or \
            "lifesim-demography-index-v2" not in " ".join(demo_header):
        failures.append(f"{demography_path.name}: not a report-2 / index-v2 demography report: {demo_header}")

    worlds: dict[tuple[str, int], dict] = {}
    for condition in (baseline, treatment):
        for seed in seeds:
            stem = f"{condition}-seed{seed:016x}"
            path = directory / f"{stem}.alfd"
            if not path.exists():
                failures.append(f"{stem}.alfd: missing")
                continue
            header, rows = read_series(path)
            if not header.startswith("field-series 3 "):
                failures.append(f"{stem}.alfd: header '{header}' is not field-series 3")
                continue
            if len(rows) != expected_samples:
                failures.append(f"{stem}.alfd: {len(rows)} samples, expected {expected_samples}")
                continue
            if any(row["occupied"] > cells for row in rows):
                failures.append(f"{stem}.alfd: occupied above {cells} cells")
                continue
            defects = [
                row["chem"] + row["microbial"]
                - (row["produced"] + row["deposited"] - row["materialized_milli"] - row["consumed"])
                for row in rows
            ]
            if any(defect != 0 for defect in defects):
                first = next(i for i, d in enumerate(defects) if d != 0)
                failures.append(
                    f"{stem}.alfd: field identity defect {defects[first]} milli at tick {rows[first]['tick']}"
                )
                continue
            if condition == baseline and rows[-1]["consumed"] != 0:
                failures.append(f"{stem}.alfd: baseline consumed {rows[-1]['consumed']} milli - arms mislabeled")
                continue
            demo = demography.get((condition, seed))
            if demo is None:
                failures.append(f"{stem}: no demography line")
                continue
            final = rows[-1]
            worlds[(condition, seed)] = {
                "population": final["population"],
                "materialized": final["materialized"],
                "births": final["births"],
                "consumed": final["consumed"],
                "field_mass": final["chem"] + final["microbial"],
                "peak_modules": max(row["max_modules"] for row in rows),
                "mat_completed": int(demo["materialized_completed"]),
                "mat_censored": int(demo["materialized_censored"]),
                "mat_median": int(demo["materialized_median_lifespan"]),
                "all_median": int(demo["median_lifespan"]),
                "starvation_share": int(demo["starvation_share_milli"]),
            }
    for failure in failures:
        print(f"REFUSED {failure}")
    if failures:
        print(f"reduction refused: {len(failures)} defective or missing series")
        return 1

    print(f"phase19 reduction: {baseline} vs {treatment}, {len(seeds)} seeds, {ticks} ticks, "
          f"SESOI lifespan {sesoi_l} bar {bar_l}, SESOI births {sesoi_b} bar {bar_b}")
    for condition in (baseline, treatment):
        print(f"condition {condition}")
        for seed in seeds:
            w = worlds[(condition, seed)]
            print(f"  seed {seed} population={w['population']} materialized={w['materialized']} "
                  f"births={w['births']} consumed_milli={w['consumed']} field_mass={w['field_mass']} "
                  f"peak_modules={w['peak_modules']} mat_completed={w['mat_completed']} "
                  f"mat_censored={w['mat_censored']} mat_median_lifespan={w['mat_median']} "
                  f"all_median_lifespan={w['all_median']} starvation_share_milli={w['starvation_share']}")
        values = [worlds[(condition, s)] for s in seeds]
        print(f"  median population {median(v['population'] for v in values)}; "
              f"median materialized {median(v['materialized'] for v in values)}; "
              f"median births {median(v['births'] for v in values)}; "
              f"median consumed_milli {median(v['consumed'] for v in values)}; "
              f"median field_mass {median(v['field_mass'] for v in values)}; "
              f"median materialized median lifespan {median(v['mat_median'] for v in values)}")
        above_one = sum(1 for v in values if v["peak_modules"] > 1)
        at_cap = sum(1 for v in values if v["population"] >= max_entities)
        no_completed = sum(1 for v in values if v["mat_completed"] == 0)
        print(f"  worlds with any body above one module: {above_one} / {len(seeds)}   (C19.5)")
        print(f"  worlds at the entity cap {max_entities}: {at_cap} / {len(seeds)}")
        print(f"  worlds with no completed materialized lifespan: {no_completed} / {len(seeds)}")

    print("pairs (treatment - baseline)")
    l_count = 0
    b_count = 0
    deltas_l = []
    deltas_b = []
    for seed in seeds:
        b = worlds[(baseline, seed)]
        t = worlds[(treatment, seed)]
        dl = t["mat_median"] - b["mat_median"]
        db = t["births"] - b["births"]
        deltas_l.append(dl)
        deltas_b.append(db)
        l_count += dl > sesoi_l
        b_count += db > sesoi_b
        print(f"  seed {seed} lifespan_delta={dl} births_delta={db}")
    print(f"  median lifespan delta {median(deltas_l)}; min {min(deltas_l)}; max {max(deltas_l)}")
    print(f"  median births delta {median(deltas_b)}; min {min(deltas_b)}; max {max(deltas_b)}")
    print(f"C19.3 pairs with materialized median lifespan delta > {sesoi_l}: {l_count} / {len(seeds)} "
          f"(bar {bar_l}: {'MET' if l_count >= bar_l else 'NOT MET'})")
    print(f"C19.4 pairs with births delta > {sesoi_b}: {b_count} / {len(seeds)} "
          f"(bar {bar_b}: {'MET' if b_count >= bar_b else 'NOT MET'})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
