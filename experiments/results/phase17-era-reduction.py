#!/usr/bin/env python3
"""Phase 17 era-detector reduction (C17.5 and the FULL contrast).

Reads one `era-report 1` text (the output of `lifesim era` over the
validation campaign at the LOCKED parameters) and prints, per condition,
the count of worlds reporting any boundary after the burn-in, the
segment-count distribution, and the five features that most often lead
a boundary's delta list. Refuses a report whose header does not echo the
locked parameters or that is short a world (counted, never silent).

Usage:

    python3 phase17-era-reduction.py <report.txt> <conditions> <worlds_per_condition>

The pre-registration (experiments/phase17-era-preregistration.md) fixes
the parameters and the decision rule; nothing below decides a criterion.
"""

import re
import sys
from collections import Counter
from pathlib import Path

LOCKED_HEADER = (
    "era-report 1 campaign phase17-era-null detector lifesim-era-v1 window 1000 "
    "penalty 200000000 max_segments 8 burn_in 10000 features 22"
)
WORLD = re.compile(
    r"world condition=(\S+) seed=(\S+) config=(\S+) schema=(\d+) windows=(\d+) "
    r"dropped=(\d+) segments=(\d+) cost=(-?\d+)"
)
DELTA = re.compile(r"delta (\S+)=(-?\d+)")


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__, file=sys.stderr)
        return 2
    lines = Path(sys.argv[1]).read_text().splitlines()
    conditions = [name for name in sys.argv[2].split(",") if name]
    expected = int(sys.argv[3])
    if not lines or lines[0].strip() != LOCKED_HEADER:
        print(f"REFUSED header is not the locked one: {lines[0] if lines else '<empty>'!r}")
        return 1
    worlds: dict[str, list[dict]] = {name: [] for name in conditions}
    current = None
    for line in lines[1:]:
        match = WORLD.fullmatch(line.strip())
        if match:
            condition = match.group(1)
            if condition not in worlds:
                print(f"REFUSED unknown condition {condition}")
                return 1
            current = {
                "seed": match.group(2),
                "segments": int(match.group(7)),
                "boundaries": 0,
                "leaders": [],
            }
            worlds[condition].append(current)
            continue
        if current is None:
            continue
        if line.startswith("boundary "):
            current["boundaries"] += 1
            current["leader_pending"] = True
        elif line.startswith("delta ") and current.get("leader_pending"):
            current["leaders"].append(DELTA.match(line).group(1))
            current["leader_pending"] = False
    failures = [f"{name}: {len(rows)} worlds, expected {expected}" for name, rows in worlds.items() if len(rows) != expected]
    if failures:
        for failure in failures:
            print(f"REFUSED {failure}")
        return 1
    print("# condition n worlds_with_boundary/n segments_distribution leading_features")
    for name in conditions:
        rows = worlds[name]
        with_boundary = sum(1 for row in rows if row["boundaries"] > 0)
        distribution = Counter(row["segments"] for row in rows)
        leaders = Counter(feature for row in rows for feature in row["leaders"])
        dist = " ".join(f"{k}:{v}" for k, v in sorted(distribution.items()))
        lead = " ".join(f"{k}:{v}" for k, v in leaders.most_common(5)) or "-"
        print(f"{name} {len(rows)} {with_boundary}/{len(rows)} {dist} {lead}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
