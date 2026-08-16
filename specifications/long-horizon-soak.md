# Long-Horizon Soak Specification

Status: design specification, not implemented. Prerequisite for the flagship
run mode (ADR-0023). Policy version `lifesim-soak-v1`.

## Problem

The longest verified continuous run in the project is 864,000 ticks, the
24-hour-equivalent Phase 1 release test. A flagship world is intended to run
for months. Thirty days at 1x is 25.9 million ticks, which is **30 times**
the longest horizon ever verified, and ninety days is 90 times.

Nothing currently establishes that the engine survives that. Every bound in
the plan is stated per tick or per campaign, and several quantities that are
provably bounded within a campaign grow monotonically across a long run.

This is not a performance question. It is a correctness question about
whether a world left alone for a season is still the same world.

## What Is Actually At Risk

Each row is a quantity that is bounded per tick and unbounded across time,
or whose bound has never been checked at this horizon.

| Quantity | Why it grows | Current bound |
|---|---|---|
| Event log file | Append-only by design | Per-tick buffer bounded; **total unbounded** |
| Snapshot chain | Checkpoints accumulate | Retention prunes count, not total bytes over time |
| Genome size | Duplication outpacing deletion over many generations | Caps exist per genome, not per lineage trend |
| Object count | Placement outpacing decay | Cell occupancy capped and, as built (Phase 12), the world total too: `artifact.max_objects` refuses creation at the cap and ledgers the refused mass to dust, so the row's "world total not" is closed - the C12.6 ledger soak is the check that the cap and the ledger hold over 10^6 ticks |
| Terrain modification set | Sparse set fills toward dense | Threshold flips representation; never shrinks |
| Learned state | Plastic edge count evolving upward | Per-organism cap; population total unbounded |
| Entity ID space | Monotonic, never reused | u64, fine, but worth stating |
| Tick duration | Any of the above feeding back into per-tick cost | Measured at campaign length only |
| Ancestry depth | Monotonic | u32, and 127 generations reached in 200k ticks |

The interaction that concerns me most is not any single row: it is that
several of these feed per-tick cost, so a slow drift in object count or
plastic-edge count shows up as tick time creeping upward over weeks. A
campaign would never see it.

## Soak Tiers

| Tier | Ticks | 1x equivalent | Purpose |
|---|---:|---|---|
| Existing | 864,000 | 1 day | Current Phase 1 release test, preserved |
| **Soak-7** | 6,048,000 | 7 days | Gate for unattended operation |
| **Soak-30** | 25,920,000 | 30 days | Gate for the flagship mode (ADR-0023) |
| Soak-90 | 77,760,000 | 90 days | Confidence tier, not a gate |

Soak runs execute headless at maximum speed. The tick count is what matters,
not the wall clock: Soak-30 at the measured Phase 2 rate is about 14.6 hours
of real time, and considerably longer once later phases land.

## Acceptance Criteria

A soak tier passes only if **every** criterion holds. Acceptance is
conjunctive (ADR-0022 A7); a good throughput number does not rescue a
growth failure.

- [ ] **S1 Bounded storage growth.** Event log bytes per 10^6 ticks, and
      total snapshot bytes under the retention policy, are both measured and
      **linear or sublinear** in tick count. Superlinear growth in either is
      a failure regardless of the absolute number, because it does not
      survive extension to the next tier.
- [ ] **S2 Tick time does not drift.** Tick p50 and p95 measured over the
      final 10^6 ticks are within a stated tolerance of the same statistics
      over the first 10^6 ticks, at matched population. Drift beyond
      tolerance is a failure and its cause must be identified, not absorbed.
- [ ] **S3 Memory is stationary.** RSS sampled at regular intervals shows no
      upward trend beyond a stated tolerance once the population stabilizes.
      This is the criterion most likely to catch a slow leak that campaign
      runs never expose.
- [ ] **S4 Invariants hold throughout.** The exact energy and biomass ledger,
      entity ordering, bounds, and every phase-specific conservation
      invariant are checked at a fixed cadence across the entire run and
      never fail. A single failure at tick 20 million is a failure of the
      tier.
- [ ] **S5 Checkpoint and restore survive the horizon.** A checkpoint taken
      near the end of the run restores successfully, verifies its recorded
      state checksum, and continues bit-identically. This is the criterion
      that makes a flagship world's history trustworthy, since its
      checkpoint chain is the only record of a world that will never be
      re-run.
- [ ] **S6 Determinism survives the horizon.** Two clean processes running
      the same seed and config to the full tick count produce identical
      final state checksums. This is expensive and is the point: a
      determinism defect that only manifests after 10^7 ticks is exactly
      the kind this project cannot tolerate, and no shorter test finds it.
- [ ] **S7 Structural quantities are stationary or explained.** Median
      genome size, plastic-edge count, object count, and terrain
      modification density either stabilize, or their continued growth is
      reported with the mechanism and its projected ceiling. An unexplained
      monotonic trend is a failure.
- [ ] **S8 The world is still interesting, or the null is recorded.**
      Population, ancestry depth, and diversity at the end of the run are
      reported. A world that went extinct at tick 3 million and then ran
      empty for 20 million more has passed S1 to S7 and failed the point of
      the exercise. Extinction is a valid world state and a valid soak
      outcome; it must be visible in the record rather than hidden behind
      green checkmarks.

## Method

- Run headless with checkpointing at the flagship cadence, so the soak
  exercises the same write path the flagship mode will use.
- Sample tick timing, RSS, and the structural quantities at a fixed tick
  interval, never a wall-clock interval, so the samples are reproducible.
- Instrument through the existing `TickObserver` boundary; the kernel still
  reads no clock.
- Resume from checkpoint at least once mid-run, so the tier also verifies
  that a long run survives an operator restart. A flagship world will be
  restarted, and a soak that never restarts does not model it.
- Record everything under a benchmark ID with full provenance, following
  the existing record format.

## Failure Handling

A soak failure is a defect report, not a tuning exercise. The specific
prohibited response is raising a tolerance until the run passes.

If a tier fails, the flagship mode is not available at that horizon, and
`ADR-0023` says so: unattended long-horizon operation is gated on Soak-30.
Campaigns are unaffected, because campaign runs are short by construction.

## Relationship To Other Work

- **Prerequisite for** the flagship run mode (ADR-0023).
- **Depends on** Phase 5's event log and asynchronous checkpointing, both of
  which it stresses harder than any campaign will.
- **Extends** the existing 864,000-tick release test rather than replacing
  it. That test stays as the fast gate.
- **Must be re-run** after any phase that adds a growth term. Phases 9, 10,
  11, 12, and 13 each add at least one, so each re-runs at least Soak-7 and
  restates the numbers. A soak result is valid only for the policy versions
  it ran under.

## Test Requirements

- Soak-7 in the standard suite as an `#[ignore]` release test, following the
  existing long-run convention.
- Soak-30 as an explicit operator-invoked run, not part of any routine
  suite.
- Growth-rate assertions (S1, S3, S7) implemented as trend tests over
  sampled series, not as single end-of-run threshold checks, so a slow drift
  is caught rather than a final value that happens to land inside a bound.
