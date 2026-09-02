#!/usr/bin/env python3
"""Phase 15 field-scaffold reduction (C15.3).

Committed BEFORE the campaign ran, alongside
experiments/phase15-field-preregistration.md, which fixes every
definition used here. Usage:

    python3 phase15-field-reduction.py <campaign-output-dir>

Reads the 150 `.alfd` field series (5 conditions x 30 seeds), refuses
anything missing, short, or conservation-broken (counted, never
silent), and prints the pre-registered per-condition curve: persistence
fraction, median formation rate, median sustainment ratio, median
occupancy, and each S condition's difference from N.
"""

import re
import sys
from pathlib import Path
from statistics import median

CONDITIONS = ["N", "S1", "S2", "S3", "S4"]
# 16001..16032 excluding 16005 and 16009 - the pre-registration's
# recorded seed amendment (preflight refused the two before any world ran).
SEEDS = [seed for seed in range(16001, 16033) if seed not in (16005, 16009)]
TICKS = 60_000
INTERVAL = 500
EXPECTED_SAMPLES = TICKS // INTERVAL  # ticks 500..60000 inclusive
WINDOW_START = TICKS - 10_000  # the stated window: final 10,000 ticks
CELLS = 64 * 64
PERSIST_FLOOR_MILLI = 10_000  # ten times the largest single seeding

SAMPLE = re.compile(
    r"sample tick=(\d+) fired=(\d+) seeded_milli=(\d+) chem_milli=(-?\d+) "
    r"produced_milli=(\d+) deposited_milli=(\d+) microbial_milli=(-?\d+) "
    r"occupied=(\d+) population=(\d+)"
)


def read_series(path: Path):
    rows = []
    for line in path.read_text().splitlines():
        match = SAMPLE.fullmatch(line.strip())
        if match:
            rows.append([int(group) for group in match.groups()])
    return rows


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    directory = Path(sys.argv[1])
    failures = []
    worlds = {}
    for condition in CONDITIONS:
        for seed in SEEDS:
            path = directory / f"{condition}-seed{seed:016x}.alfd"
            if not path.is_file():
                failures.append(f"missing series {path.name}")
                continue
            rows = read_series(path)
            if len(rows) != EXPECTED_SAMPLES:
                failures.append(
                    f"{path.name}: {len(rows)} samples, expected {EXPECTED_SAMPLES}"
                )
                continue
            tick, fired, seeded, chem, produced, deposited, microbial, occupied, _ = rows[-1]
            if tick != TICKS:
                failures.append(f"{path.name}: final sample at tick {tick}")
                continue
            # The hard conservation gate, independent of the in-run check.
            if produced + deposited != chem + microbial:
                failures.append(
                    f"{path.name}: identity broken "
                    f"({produced}+{deposited} != {chem}+{microbial})"
                )
                continue
            window = [row for row in rows if row[0] >= WINDOW_START]
            persistent = (
                all(row[6] > 0 for row in window)
                and microbial >= PERSIST_FLOOR_MILLI
            )
            worlds[(condition, seed)] = {
                "rate_per_1e6_cell_ticks": fired * 1_000_000 / (CELLS * TICKS),
                "persistent": persistent,
                "sustainment": microbial / max(seeded, 1),
                "occupancy": occupied / CELLS,
            }
    if failures:
        print(f"REFUSED: {len(failures)} defective series", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1

    summary = {}
    for condition in CONDITIONS:
        rows = [worlds[(condition, seed)] for seed in SEEDS]
        summary[condition] = {
            "persistent": sum(1 for row in rows if row["persistent"]),
            "rate": median(row["rate_per_1e6_cell_ticks"] for row in rows),
            "sustainment": median(row["sustainment"] for row in rows),
            "occupancy": median(row["occupancy"] for row in rows),
        }

    print("# phase15 field-scaffold reduction (pre-registered observables)")
    print(f"# worlds: {len(worlds)} of {len(CONDITIONS) * len(SEEDS)}, all identities exact")
    print(
        "# condition persistent/30 median_rate_per_1e6_cell_ticks "
        "median_sustainment median_occupancy"
    )
    for condition in CONDITIONS:
        row = summary[condition]
        print(
            f"{condition} {row['persistent']}/30 {row['rate']:.3f} "
            f"{row['sustainment']:.3f} {row['occupancy']:.4f}"
        )
    print("# differences from N (S minus N)")
    base = summary["N"]
    for condition in CONDITIONS[1:]:
        row = summary[condition]
        print(
            f"{condition}-N persistent {row['persistent'] - base['persistent']:+d} "
            f"rate {row['rate'] - base['rate']:+.3f} "
            f"sustainment {row['sustainment'] - base['sustainment']:+.3f} "
            f"occupancy {row['occupancy'] - base['occupancy']:+.4f}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
