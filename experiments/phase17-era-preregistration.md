# Phase 17 era-detector pre-registration (C17.4, C17.5)

**LOCKED 2026-09-02, before any validation-campaign world was read.**
Calibrated in Stage A/B - the synthetic fixtures in the suite and the
four-seed pilot `experiments/phase17-era-pilot.campaign` (seeds
18031..18034, disjoint from the campaign's; archived at
`runs/phase17-era-pilot-0xa29832d1958f074c`, 8 worlds, 0 failed) - and
committed ahead of `experiments/phase17-era-null.campaign`. Every
parameter below carries the pilot number that set it; nothing here
changes an observable's definition.

## Question

C17.5: on event logs from runs with artifacts, signalling and contest
ablated, does the detector report no segment above threshold in at
least 25 of 30 seeds? C17.4: on synthetic logs with injected boundaries,
does it recover them within one window at stated precision and recall?
The two are read together: a detector that passes C17.5 by never finding
anything fails C17.4, and one that passes C17.4 by finding everything
fails C17.5. The FULL arm is reported beside the null as a contrast, with
no bar.

## Detector (`lifesim-era-v1`, ADR-0033)

Twenty-two integer features per window (rates per 1,000 organism-ticks,
two level features), absent groups excluded rather than zeroed; exact
optimal partitioning with a per-boundary penalty; the penalty is the
threshold. Analysis version outside the config hash. The command is
`lifesim era --manifest <manifest> --penalty 200000000 --window 1000
--burn-in 10000 --max-segments 8`.

## Parameters, locked

- `window_ticks` = **1,000** (60 windows over the horizon, 50 after the
  burn-in). The pilot's FULL worlds carry a relocating patch every 2,000
  ticks, so this window resolves that period; a coarser window would
  average it away, a finer one would multiply the demographic noise.
- `burn_in_ticks` = **10,000** (the first ten windows dropped). The
  founders' settlement is a real regime the detector would be right to
  find; the analyzed range is what "nothing can happen" must be true of.
- `penalty_milli` = **200,000,000** (2 x 10^8). The pilot sweep on the
  four NULL worlds (window 1,000, burn-in 10,000): at 1 x 10^8 one world
  reported a boundary (tick 25,001: starvation deaths +3,266 and births
  +2,709 milli per 1,000 organism-ticks across it - a demographic shift),
  at 1.5 x 10^8 the same world still reported it, at 2 x 10^8 and above
  no NULL world reported any. The rule stated in the draft - the
  smallest penalty at which every null pilot world reports no boundary,
  rounded up to one significant figure - gives 2 x 10^8.
- `max_segments` = **8**.

Feature scale, for reading the penalty: in the pilot, births run
~900-1,600 and starvation deaths ~450-1,200 milli per 1,000
organism-ticks with population ~33 milli of `max_entities`; a boundary
must buy a within-segment sum-of-squares reduction of 2 x 10^8, which
a step of ~4,000 in one feature over twenty windows on each side does
(20 x 20 / 40 x 4,000^2 = 1.6 x 10^8, plus the other features that move
with it) and a step of ~2,000 alone does not.

## Decision rules, stated in advance

- **C17.5**: at the locked parameters, the count of NULL worlds (of 30)
  reporting any boundary after the burn-in; the criterion holds at 5 or
  fewer. The FULL arm's count is reported beside it.
- **C17.4**: the suite's synthetic fixtures at the locked penalty and
  the pilot's feature scale (`SYNTHETIC_FIXTURE_VERSION` 1 driven at
  population 330 with steps of at least fourfold on the driven features:
  one boundary, three boundaries, a one-group boundary, and the null):
  precision and recall at `+/- 1` window both 1.0 on the fixture set,
  and the synthetic null yields no boundary.
- **C17.3**: the CLI report for one pilot world, produced in two
  processes, compared byte for byte in `verify-phase17-determinism.sh`.

## Hard gates

- Every NULL and FULL log decodes fail-closed (`decode_log_events`); a
  world whose log does not decode is a defect, counted, never skipped.
- The reduction requires all sixty reports; a missing world is fatal.
- The FULL arm differs from NULL only in the declared `vary` fields (the
  parser refused the first draft's redundant declarations, which is the
  gate working).

## Expected outcomes, recorded in advance

NULL: 0 to 3 of 30 worlds with a boundary after the burn-in - the pilot
showed one demographic shift in four worlds just below the locked
penalty, so a few such shifts clearing it across thirty seeds would not
surprise; more than five would fail C17.5 and be reported as such.
FULL: near saturation of the segment cap in most worlds (the pilot: 8,
8, 8 and 5 segments at 10^9; the relocating patch is a regime change
every two windows), reported, not decided.

## Out of scope here

C17.1 and C17.2 are suite tests (`era_neutrality.rs`,
`dependency_direction.rs`, both green); C17.6 and C17.7 are the Phase 13
tradition detector's format (ADR-0033 records D-126's conviction as the
reading); C17.8 is the benchmark record.
