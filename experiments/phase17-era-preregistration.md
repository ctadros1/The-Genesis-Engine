# Phase 17 era-detector pre-registration (C17.4, C17.5)

**STATUS: DRAFT, NOT LOCKED.** The detector's parameters are calibrated
in Stage A/B (the synthetic fixtures in the suite and a pilot over four
disjoint seeds of the null campaign) and the record is LOCKED and
committed before the C17.5 campaign's thirty seeds are read
(`experiments/phase17-era-null.campaign`). Calibration may set the
window, the burn-in, the penalty and the segment cap; it may not change
an observable's definition, and each value carries the number that set
it.

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
threshold. Analysis version outside the config hash.

## Parameters, calibrated then locked

- `window_ticks`: `[CALIBRATE]` (candidate 1,000 - sixty windows over
  the horizon).
- `burn_in_ticks`: `[CALIBRATE]` (candidate 10,000 - the founders'
  settlement, a real regime the detector would be right to find; the
  analyzed range is what "nothing can happen" must be true of).
- `penalty_milli`: `[CALIBRATE]` - the smallest penalty at which the
  four-seed null pilot reports no boundary in every world, rounded up to
  one significant figure, and then checked against the synthetic fixtures
  for recall; both numbers recorded here.
- `max_segments`: 8.

## Decision rules, stated in advance

- **C17.5**: at the locked parameters, the count of NULL worlds (of 30)
  reporting any boundary after the burn-in; the criterion holds at 5 or
  fewer. The FULL arm's count is reported beside it.
- **C17.4**: the suite's synthetic fixtures (`SYNTHETIC_FIXTURE_VERSION`
  1: one boundary, three boundaries, one-group boundary, null) at the
  locked penalty: precision and recall at `+/- 1` window both 1.0 on the
  fixture set, and the synthetic null yields no boundary.
- **C17.3**: the CLI report for one world, produced in two processes,
  compared byte for byte in the verify script.

## Hard gates

- Every NULL and FULL log decodes fail-closed (`decode_log_events`); a
  world whose log does not decode is a defect, counted, never skipped.
- The reduction requires all sixty reports; a missing world is fatal.
- The FULL arm differs from NULL only in the declared `vary` fields.

## Expected outcomes, recorded in advance

NULL: the a-priori expectation is 0 to 3 of 30 worlds with a boundary
after the burn-in - a schema-2 ecology with physiology drifts
demographically but has no mechanism that switches regime. FULL: the
relocating patch moves every 2,000 ticks, so demographic and object
rates are expected to show boundaries in many worlds; that is reported,
not decided. `[PILOT]` facts from the four-seed pilot go here.

## Out of scope here

C17.1 and C17.2 are suite tests; C17.6 and C17.7 are the Phase 13
tradition detector's format (ADR-0033 records D-126's conviction as the
reading); C17.8 is the benchmark record.
