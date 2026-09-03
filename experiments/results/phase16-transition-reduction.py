#!/usr/bin/env python3
"""Phase 16 transition reduction (C16.5, C16.6).

Reads a campaign output directory's `.alfd` field series (series version
2: the Phase 15 columns plus `materialized`, `materialized_milli`,
`max_modules`, `multi_module`, `births`), refuses anything missing, short or
conservation-broken (counted, never silent), and prints per condition:

  worlds materialized / n           (C16.5's count)
  median first-materialization tick
  median final population
  median final materialized_total
  worlds with any body above one module / n   (C16.6's count)
  median peak max_modules over the series
  median final multi_module fraction (milli)

Usage:

    python3 phase16-transition-reduction.py <dir> <conditions> <seeds> <ticks> <interval> <cells>

    conditions  comma-separated condition names, e.g. T0,N,S2,S4
    seeds       a range `17001..17030` or a comma list; a range may carry
                exclusions after a slash, e.g. `17001..17032/17005,17009`
    ticks       the campaign's run length
    interval    the field interval (`output field N`)
    cells       cells_x * cells_y

The pre-registration (experiments/phase16-transition-preregistration.md)
fixes every definition used here; nothing below decides a criterion.
"""

import re
import sys
from pathlib import Path
from statistics import median

SAMPLE = re.compile(
    r"sample tick=(\d+) fired=(\d+) seeded_milli=(\d+) chem_milli=(-?\d+) "
    r"produced_milli=(\d+) deposited_milli=(\d+) microbial_milli=(-?\d+) "
    r"occupied=(\d+) population=(\d+) materialized=(\d+) "
    r"materialized_milli=(-?\d+) max_modules=(\d+) multi_module=(\d+) births=(\d+)(?: .*)?"
)
COLUMNS = [
    "tick", "fired", "seeded", "chem", "produced", "deposited", "microbial",
    "occupied", "population", "materialized", "materialized_milli",
    "max_modules", "multi_module", "births",
]


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


def read_series(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text().splitlines():
        match = SAMPLE.fullmatch(line.strip())
        if match:
            rows.append(dict(zip(COLUMNS, (int(group) for group in match.groups()))))
    return rows


def main() -> int:
    if len(sys.argv) != 7:
        print(__doc__, file=sys.stderr)
        return 2
    directory = Path(sys.argv[1])
    conditions = [name for name in sys.argv[2].split(",") if name]
    seeds = parse_seeds(sys.argv[3])
    ticks = int(sys.argv[4])
    interval = int(sys.argv[5])
    cells = int(sys.argv[6])
    expected_samples = ticks // interval
    failures: list[str] = []
    worlds: dict[tuple[str, int], list[dict]] = {}
    for condition in conditions:
        for seed in seeds:
            path = directory / f"{condition}-seed{seed:016x}.alfd"
            if not path.is_file():
                failures.append(f"missing series {path.name}")
                continue
            header = path.read_text().splitlines()[0] if path.stat().st_size else ""
            if not (header.startswith("field-series 2 ") or header.startswith("field-series 3 ")):
                failures.append(f"{path.name}: not a version-2/3 field series ({header[:32]!r})")
                continue
            rows = read_series(path)
            if len(rows) != expected_samples:
                failures.append(f"{path.name}: {len(rows)} samples, expected {expected_samples}")
                continue
            final = rows[-1]
            defect = (
                final["produced"] + final["deposited"] - final["materialized_milli"]
                - final["chem"] - final["microbial"]
            )
            if defect != 0:
                failures.append(f"{path.name}: field identity defect {defect} milli at the final sample")
                continue
            if final["materialized_milli"] < 0:
                failures.append(f"{path.name}: negative materialized_milli")
                continue
            worlds[(condition, seed)] = rows
    if failures:
        for failure in failures:
            print(f"REFUSED {failure}")
        print(f"reduction refused: {len(failures)} defective or missing series")
        return 1

    print(
        "# condition n materialized/n median_first_tick median_final_population "
        "median_final_materialized median_final_births births_worlds/n "
        "multi_module_worlds/n median_peak_max_modules "
        "median_final_multi_fraction_milli median_occupancy"
    )
    for condition in conditions:
        series = [worlds[(condition, seed)] for seed in seeds]
        n = len(series)
        materialized = [rows for rows in series if rows[-1]["materialized"] > 0]
        first_ticks = [
            next(row["tick"] for row in rows if row["materialized"] > 0) for rows in materialized
        ]
        final_population = [rows[-1]["population"] for rows in series]
        final_materialized = [rows[-1]["materialized"] for rows in series]
        final_births = [rows[-1]["births"] for rows in series]
        births_worlds = sum(1 for births in final_births if births > 0)
        peak_modules = [max(row["max_modules"] for row in rows) for rows in series]
        multi_worlds = sum(1 for peak in peak_modules if peak > 1)
        final_multi_fraction = [
            (rows[-1]["multi_module"] * 1000 // rows[-1]["population"]) if rows[-1]["population"] else 0
            for rows in series
        ]
        occupancy = [rows[-1]["occupied"] / cells for rows in series]
        print(
            f"{condition} {n} {len(materialized)}/{n} "
            f"{median(first_ticks) if first_ticks else 'none'} "
            f"{median(final_population)} {median(final_materialized)} "
            f"{median(final_births)} {births_worlds}/{n} "
            f"{multi_worlds}/{n} {median(peak_modules)} {median(final_multi_fraction)} "
            f"{median(occupancy):.4f}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
